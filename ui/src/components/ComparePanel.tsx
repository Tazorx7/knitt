import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

type ComparedFile = {
  path: string;
  status: string;
};

type Comparison = {
  ahead: number;
  behind: number;
  files: ComparedFile[];
};

type ComparePanelProps = {
  repoPath: string;
  currentBranch?: string;
  onOpenDiff: (file: string, contents: { original: string; modified: string }) => void;
};

const STATUS_LABEL: Record<string, string> = {
  added: "added",
  deleted: "deleted",
  modified: "modified",
  renamed: "renamed",
  copied: "copied",
  typechange: "type changed",
  unreadable: "unreadable",
};

export default function ComparePanel({
  repoPath,
  currentBranch,
  onOpenDiff,
}: ComparePanelProps) {
  const [branches, setBranches] = useState<string[]>([]);
  const [base, setBase] = useState("");
  const [head, setHead] = useState("");
  const [comparison, setComparison] = useState<Comparison | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);

  useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        const res: any = await invoke("repo_branches", { path: repoPath });
        const names: string[] = Array.isArray(res) ? res : [];
        if (cancelled) return;
        setBranches(names);
        const target = currentBranch && names.includes(currentBranch) ? currentBranch : names[0] ?? "";
        const other = names.find((name) => name !== target) ?? target;
        setHead(target);
        setBase(other);
      } catch {
        if (!cancelled) setError("Could not load branches.");
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [repoPath, currentBranch]);

  useEffect(() => {
    if (!base || !head || base === head) {
      setComparison(null);
      return;
    }
    let cancelled = false;
    setLoading(true);
    setError(null);
    (async () => {
      try {
        const res = await invoke<Comparison>("repo_compare_branches", {
          path: repoPath,
          base,
          head,
        });
        if (!cancelled) setComparison(res);
      } catch (e) {
        if (!cancelled) {
          setComparison(null);
          setError(typeof e === "string" ? e : "Could not compare those branches.");
        }
      } finally {
        if (!cancelled) setLoading(false);
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [repoPath, base, head]);

  async function openFile(file: string) {
    setError(null);
    try {
      const contents = await invoke<{ original: string; modified: string }>(
        "repo_compare_file",
        { path: repoPath, base, head, file },
      );
      onOpenDiff(file, contents);
    } catch {
      setError("Could not load that file's changes.");
    }
  }

  return (
    <div className="panel p-4">
      <h2 className="text-lg font-semibold mb-2">Compare branches</h2>

      <div className="flex items-end gap-2">
        <label className="flex-1 text-xs">
          <span className="block pb-1 opacity-70">Base</span>
          <select
            value={base}
            onChange={(event) => setBase(event.target.value)}
            className="w-full rounded border border-neutral-300 bg-white px-2 py-1 text-sm dark:border-neutral-700 dark:bg-neutral-900"
          >
            {branches.map((name) => (
              <option key={name} value={name}>
                {name}
              </option>
            ))}
          </select>
        </label>
        <label className="flex-1 text-xs">
          <span className="block pb-1 opacity-70">Compare</span>
          <select
            value={head}
            onChange={(event) => setHead(event.target.value)}
            className="w-full rounded border border-neutral-300 bg-white px-2 py-1 text-sm dark:border-neutral-700 dark:bg-neutral-900"
          >
            {branches.map((name) => (
              <option key={name} value={name}>
                {name}
              </option>
            ))}
          </select>
        </label>
      </div>

      {base === head ? (
        <p className="mt-3 text-xs opacity-70">Choose two different branches.</p>
      ) : loading ? (
        <p className="mt-3 text-xs opacity-70">Comparing…</p>
      ) : comparison ? (
        <>
          <p className="mt-3 text-xs opacity-80" role="status">
            {head} is {comparison.ahead} {comparison.ahead === 1 ? "commit" : "commits"} ahead and{" "}
            {comparison.behind} behind {base}.
          </p>
          {comparison.files.length === 0 ? (
            <p className="mt-2 text-sm opacity-70">No file differences.</p>
          ) : (
            <ul className="mt-2 space-y-1" aria-label="Changed files">
              {comparison.files.map((file) => (
                <li key={`${file.status}:${file.path}`}>
                  <button
                    type="button"
                    onClick={() => openFile(file.path)}
                    className="flex w-full items-center justify-between gap-2 rounded px-2 py-1 text-left text-sm hover:bg-neutral-100 dark:hover:bg-neutral-900 focus:outline-none focus-visible:ring-2 focus-visible:ring-[color:var(--accent)]"
                  >
                    <span className="min-w-0 truncate" title={file.path}>
                      {file.path}
                    </span>
                    <span className="shrink-0 text-xs opacity-60">
                      {STATUS_LABEL[file.status] ?? file.status}
                    </span>
                  </button>
                </li>
              ))}
            </ul>
          )}
        </>
      ) : null}

      {error && (
        <p role="status" className="mt-2 text-xs text-red-700 dark:text-red-400">
          {error}
        </p>
      )}
    </div>
  );
}
