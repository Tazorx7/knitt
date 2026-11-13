import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

type ReflogEntry = {
  message: string;
  id: string;
  time: number;
};

type ReflogPanelProps = {
  repoPath: string;
  onSelectCommit?: (id: string) => void;
};

/** Recent HEAD movements, including commits removed by amend or undo. */
export default function ReflogPanel({ repoPath, onSelectCommit }: ReflogPanelProps) {
  const [entries, setEntries] = useState<ReflogEntry[]>([]);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        const res = await invoke<ReflogEntry[]>("repo_reflog", { path: repoPath });
        if (!cancelled) setEntries(Array.isArray(res) ? res : []);
      } catch {
        if (!cancelled) setError("Could not load the reflog.");
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [repoPath]);

  if (error) {
    return (
      <p role="status" className="text-xs text-red-700 dark:text-red-400">
        {error}
      </p>
    );
  }

  if (entries.length === 0) {
    return <p className="text-sm opacity-60">No reflog entries</p>;
  }

  return (
    <ul className="flex flex-col gap-0.5" aria-label="Reflog">
      {entries.map((entry, index) => (
        <li key={`${entry.id}:${index}`}>
          <button
            type="button"
            onClick={() => onSelectCommit?.(entry.id)}
            disabled={!onSelectCommit}
            title={new Date(entry.time * 1000).toLocaleString()}
            className="flex w-full items-center gap-2 rounded px-2 py-1 text-left text-sm hover:bg-neutral-100 disabled:hover:bg-transparent dark:hover:bg-neutral-800 focus:outline-none focus-visible:ring-2 focus-visible:ring-[color:var(--accent)]"
          >
            <span className="shrink-0 font-mono text-xs opacity-60">
              {entry.id.slice(0, 7)}
            </span>
            <span className="min-w-0 truncate">{entry.message || "no message"}</span>
          </button>
        </li>
      ))}
    </ul>
  );
}
