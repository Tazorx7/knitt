import { useCallback, useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useVirtualizer } from "@tanstack/react-virtual";

type Commit = {
  id: string;
  message: string;
  author: string;
  time: number;
};

type HistoryPanelProps = {
  repoPath: string;
  /** Bumped by the parent when the repository is re-read, so a new commit or a
   *  moved branch shows up without polling. */
  refreshSignal?: number;
  onSelectCommit?: (id: string) => void;
};

/** Commits read per request. Long histories are paged in as the list scrolls,
 *  and only the visible rows are rendered. */
const PAGE = 200;

function formatDate(seconds: number): string {
  const date = new Date(seconds * 1000);
  const now = new Date();
  const sameYear = date.getFullYear() === now.getFullYear();
  return date.toLocaleDateString(undefined, {
    month: "short",
    day: "numeric",
    ...(sameYear ? {} : { year: "numeric" }),
  });
}

export default function HistoryPanel({
  repoPath,
  refreshSignal,
  onSelectCommit,
}: HistoryPanelProps) {
  const [commits, setCommits] = useState<Commit[]>([]);
  const [loading, setLoading] = useState(false);
  const [done, setDone] = useState(false);
  const [empty, setEmpty] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const scrollRef = useRef<HTMLDivElement | null>(null);
  const headRef = useRef<string | null>(null);
  const repoRef = useRef<string | null>(null);
  const loadingRef = useRef(false);

  /** Reads the first page. Unless forced, an unchanged HEAD keeps the list and
   *  the scroll position, so staging a file does not jump the history. */
  const loadFirstPage = useCallback(
    async (force: boolean) => {
      if (loadingRef.current) return;
      loadingRef.current = true;
      setLoading(true);
      try {
        const page = await invoke<Commit[]>("repo_log", {
          path: repoPath,
          limit: PAGE,
          skip: 0,
        });
        const first = Array.isArray(page) ? page : [];
        if (!force && first[0]?.id === headRef.current) {
          setEmpty(first.length === 0);
          setError(null);
          return;
        }
        headRef.current = first[0]?.id ?? null;
        setCommits(first);
        setDone(first.length < PAGE);
        setEmpty(first.length === 0);
        setError(null);
      } catch (e) {
        const text =
          typeof e === "string" ? e : "Could not load the history.";
        if (text === "No commits on the current branch.") {
          headRef.current = null;
          setCommits([]);
          setDone(true);
          setEmpty(true);
          setError(null);
        } else {
          setError(text);
          setDone(true);
        }
      } finally {
        loadingRef.current = false;
        setLoading(false);
      }
    },
    [repoPath],
  );

  const loadNextPage = useCallback(async () => {
    if (loadingRef.current || done) return;
    loadingRef.current = true;
    setLoading(true);
    try {
      const page = await invoke<Commit[]>("repo_log", {
        path: repoPath,
        limit: PAGE,
        skip: commits.length,
      });
      const more = Array.isArray(page) ? page : [];
      setCommits((current) => [...current, ...more]);
      setDone(more.length < PAGE);
      setError(null);
    } catch (e) {
      setError(typeof e === "string" ? e : "Could not load more history.");
      setDone(true);
    } finally {
      loadingRef.current = false;
      setLoading(false);
    }
  }, [repoPath, done, commits.length]);

  useEffect(() => {
    const isNewRepo = repoRef.current !== repoPath;
    repoRef.current = repoPath;
    if (isNewRepo) {
      headRef.current = null;
      setCommits([]);
      setDone(false);
      setEmpty(false);
      setError(null);
    }
    loadFirstPage(isNewRepo);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [repoPath, refreshSignal]);

  const virtualizer = useVirtualizer({
    count: commits.length,
    getScrollElement: () => scrollRef.current,
    estimateSize: () => 44,
    overscan: 8,
  });

  const items = virtualizer.getVirtualItems();
  const lastIndex = items.length > 0 ? items[items.length - 1].index : -1;

  useEffect(() => {
    if (lastIndex >= commits.length - 3) loadNextPage();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [lastIndex, commits.length, done]);

  if (error) {
    return (
      <p role="alert" className="text-xs text-red-700 dark:text-red-400">
        {error}
      </p>
    );
  }

  if (empty && commits.length === 0) {
    return <p className="text-xs opacity-60">No commits yet.</p>;
  }

  return (
    <div
      ref={scrollRef}
      className="h-80 overflow-y-auto"
      role="group"
      aria-label="Commit history"
    >
      <div
        className="relative w-full"
        style={{ height: `${virtualizer.getTotalSize()}px` }}
      >
        {items.map((item) => {
          const commit = commits[item.index];
          if (!commit) return null;
          const summary = commit.message.split("\n")[0] || "(no message)";
          return (
            <div
              key={commit.id}
              className="absolute left-0 top-0 w-full"
              style={{
                height: `${item.size}px`,
                transform: `translateY(${item.start}px)`,
              }}
            >
              <button
                type="button"
                onClick={() => onSelectCommit?.(commit.id)}
                disabled={!onSelectCommit}
                title={`${commit.author} · ${new Date(commit.time * 1000).toLocaleString()}\n${commit.message}`}
                aria-label={`View commit ${summary}`}
                className="h-full w-full rounded px-2 py-1 text-left hover:bg-neutral-100 disabled:hover:bg-transparent dark:hover:bg-neutral-800 focus:outline-none focus-visible:ring-2 focus-visible:ring-[color:var(--accent)]"
              >
                <div className="truncate text-sm">{summary}</div>
                <div className="flex items-center gap-2 text-xs opacity-60">
                  <span className="shrink-0 font-mono">
                    {commit.id.slice(0, 7)}
                  </span>
                  <span className="min-w-0 truncate">{commit.author}</span>
                  <span className="shrink-0">{formatDate(commit.time)}</span>
                </div>
              </button>
            </div>
          );
        })}
      </div>
      {loading && (
        <p className="px-2 py-1 text-xs opacity-60">Loading commits…</p>
      )}
    </div>
  );
}
