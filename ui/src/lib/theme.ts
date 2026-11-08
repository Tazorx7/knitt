import type { ThemeMode } from "./settings";

export function systemPrefersDark(): boolean {
  return window.matchMedia("(prefers-color-scheme: dark)").matches;
}

export function resolveDark(mode: ThemeMode): boolean {
  return mode === "dark" || (mode === "system" && systemPrefersDark());
}

/**
 * Applies the theme to the document root and returns whether it resolved dark.
 * Both class names are replaced so Tailwind's `dark:` variants follow.
 */
export function applyTheme(mode: ThemeMode): boolean {
  const dark = resolveDark(mode);
  const root = document.documentElement;
  root.classList.remove("light", "dark");
  root.classList.add(dark ? "dark" : "light");
  return dark;
}
