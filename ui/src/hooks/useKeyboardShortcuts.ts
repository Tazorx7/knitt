import { useEffect } from "react";

type ShortcutHandlers = {
  onOpenRepo?: () => void;
  onCommit?: () => void;
  onPush?: () => void;
  onFetch?: () => void;
};

export function useKeyboardShortcuts(handlers: ShortcutHandlers) {
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      const isMod = e.ctrlKey || e.metaKey;
      
      if (isMod && e.key === "o") {
        e.preventDefault();
        handlers.onOpenRepo?.();
      } else if (isMod && e.key === "Enter") {
        e.preventDefault();
        handlers.onCommit?.();
      } else if (isMod && e.shiftKey && e.key === "P") {
        e.preventDefault();
        handlers.onPush?.();
      } else if (isMod && e.shiftKey && e.key === "F") {
        e.preventDefault();
        handlers.onFetch?.();
      }
    };

    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [handlers]);
}
