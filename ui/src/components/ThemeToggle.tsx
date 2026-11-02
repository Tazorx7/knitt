import { useEffect, useState } from "react";

export default function ThemeToggle() {
  const [theme, setTheme] = useState<"light" | "dark">("light");

  useEffect(() => {
    const saved = localStorage.getItem("theme") as "light" | "dark" | "system" | null;
    if (saved === "dark") setTheme("dark");
    else setTheme("light");
    const root = document.documentElement;
    root.classList.remove("light", "dark");
    const isDark = saved === "dark" || (saved === "system" && window.matchMedia("(prefers-color-scheme: dark)").matches);
    root.classList.add(isDark ? "dark" : "light");
  }, []);

  const toggleTheme = () => {
    const newTheme = theme === "light" ? "dark" : "light";
    setTheme(newTheme);
    const root = document.documentElement;
    root.classList.remove("light", "dark");
    root.classList.add(newTheme);
    localStorage.setItem("theme", newTheme);
  };

  return (
    <button
      type="button"
      className="px-2 py-1 text-xs bg-gray-200 dark:bg-gray-700 rounded"
      onClick={toggleTheme}
      title="Toggle theme"
    >
      {theme === "light" ? "🌙" : "☀️"}
    </button>
  );
}
