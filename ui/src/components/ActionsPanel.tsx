import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

type ActionsPanelProps = {
  repoPath: string;
};

type WorkflowRun = {
  id: number;
  name: string;
  status: string;
  conclusion: string | null;
};

type WorkflowJob = {
  id: number;
  name: string;
  status: string;
  conclusion: string | null;
};

function failureMessage(cause: unknown, fallback: string) {
  return typeof cause === "string" ? cause : fallback;
}

function stateLabel(status: string, conclusion: string | null) {
  return conclusion ?? status.split("_").join(" ");
}

export default function ActionsPanel({ repoPath }: ActionsPanelProps) {
  const [runs, setRuns] = useState<WorkflowRun[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [jobsFor, setJobsFor] = useState<number | null>(null);
  const [jobs, setJobs] = useState<WorkflowJob[]>([]);
  const [jobsError, setJobsError] = useState<string | null>(null);
  const [logFor, setLogFor] = useState<number | null>(null);
  const [log, setLog] = useState("");
  const [logError, setLogError] = useState<string | null>(null);

  async function load() {
    setError(null);
    try {
      const res: any = await invoke("gh_list_repo_workflow_runs", { path: repoPath });
      setRuns(Array.isArray(res) ? res : []);
    } catch (e) {
      setError(failureMessage(e, "Unable to load this repository's GitHub data."));
    }
  }

  useEffect(() => {
    load();
  }, [repoPath]);

  async function toggleRun(run: WorkflowRun) {
    if (jobsFor === run.id) {
      setJobsFor(null);
      return;
    }
    setJobsFor(run.id);
    setJobs([]);
    setJobsError(null);
    setLogFor(null);
    setLog("");
    setLogError(null);
    try {
      const res: any = await invoke("gh_list_workflow_jobs", {
        path: repoPath,
        runId: run.id,
      });
      setJobs(Array.isArray(res) ? res : []);
    } catch (e) {
      setJobsError(failureMessage(e, "Unable to load this run's jobs."));
    }
  }

  async function openLog(job: WorkflowJob) {
    if (logFor === job.id) {
      setLogFor(null);
      return;
    }
    setLogFor(job.id);
    setLog("");
    setLogError(null);
    try {
      const text: any = await invoke("gh_workflow_job_log", {
        path: repoPath,
        jobId: job.id,
      });
      setLog(typeof text === "string" ? text : "");
    } catch (e) {
      setLogError(failureMessage(e, "Unable to load that job's log."));
    }
  }

  return (
    <div className="p-2">
      <div className="text-sm font-medium mb-1">Actions</div>
      {error ? (
        <div className="text-xs text-red-700 dark:text-red-400" role="alert">
          {error}
        </div>
      ) : runs.length === 0 ? (
        <div className="text-xs opacity-60">No recent workflow runs</div>
      ) : (
        <ul className="space-y-1">
          {runs.map((run) => (
            <li key={run.id} className="text-xs">
              <button
                type="button"
                className="w-full truncate text-left rounded px-1 hover:bg-black/5 dark:hover:bg-white/10"
                aria-expanded={jobsFor === run.id}
                onClick={() => toggleRun(run)}
              >
                <span className="opacity-60">{stateLabel(run.status, run.conclusion)}</span>{" "}
                {run.name}
                <span className="sr-only">
                  {jobsFor === run.id ? ", hide jobs" : ", show jobs"}
                </span>
              </button>
              {jobsFor === run.id && (
                <div className="mt-1 ml-2 space-y-1">
                  {jobsError ? (
                    <div className="text-red-700 dark:text-red-400" role="alert">
                      {jobsError}
                    </div>
                  ) : jobs.length === 0 ? (
                    <div className="opacity-60">No jobs in this run</div>
                  ) : (
                    jobs.map((job) => (
                      <div key={job.id}>
                        <button
                          type="button"
                          className="w-full truncate text-left rounded px-1 hover:bg-black/5 dark:hover:bg-white/10"
                          aria-expanded={logFor === job.id}
                          onClick={() => openLog(job)}
                        >
                          <span className="opacity-60">
                            {stateLabel(job.status, job.conclusion)}
                          </span>{" "}
                          {job.name}
                          <span className="sr-only">
                            {logFor === job.id ? ", hide log" : ", show log"}
                          </span>
                        </button>
                        {logFor === job.id && (
                          <div className="mt-1">
                            {logError ? (
                              <div className="text-red-700 dark:text-red-400" role="alert">
                                {logError}
                              </div>
                            ) : (
                              <pre className="max-h-64 overflow-auto rounded bg-black/5 dark:bg-white/10 p-1 text-[11px] leading-snug whitespace-pre-wrap break-words">
                                {log || "Loading log…"}
                              </pre>
                            )}
                          </div>
                        )}
                      </div>
                    ))
                  )}
                </div>
              )}
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}
