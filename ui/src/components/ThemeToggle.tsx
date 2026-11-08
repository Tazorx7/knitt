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
      className="rounded px-2 py-1 text-xs bg-gray-200 dark:bg-gray-700 focus:outline-none focus-visible:ring-2 focus-visible:ring-blue-500"
      onClick={onToggle}
      title={label}
      aria-label={label}
    >
      {dark ? "☀️" : "🌙"}
    </button>
  );
}
