/**
 * Light/dark switch. A two-state toggle rather than a menu: the third state
 * (follow the system) stays available in Settings → Appearance.
 */
export default function ThemeToggle({
  dark,
  onToggle,
}: {
  dark: boolean;
  onToggle: () => void;
}) {
  const label = dark ? "Switch to light theme" : "Switch to dark theme";
  return (
    <button
      type="button"
      role="switch"
      aria-checked={dark}
      className="relative inline-flex h-6 w-11 shrink-0 items-center rounded-full bg-neutral-200 transition-colors dark:bg-neutral-700 focus:outline-none focus-visible:ring-2 focus-visible:ring-[color:var(--accent)]"
      onClick={onToggle}
      title={label}
      aria-label={label}
    >
      <span
        aria-hidden="true"
        className={`pointer-events-none flex h-5 w-5 items-center justify-center rounded-full bg-white text-[10px] shadow transition-transform dark:bg-neutral-900 ${
          dark ? "translate-x-5" : "translate-x-0.5"
        }`}
      >
        {dark ? "🌙" : "☀️"}
      </span>
    </button>
  );
}
