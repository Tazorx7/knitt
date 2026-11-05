import { DiffEditor } from "@monaco-editor/react";
import "../lib/monaco";

type DiffViewerProps = {
  original?: string;
  modified?: string;
  language?: string;
  fileName?: string;
};

// Map a file name/extension to a Monaco language id.
export function languageForFile(fileName?: string): string {
  if (!fileName) return "plaintext";
  const ext = fileName.split(".").pop()?.toLowerCase() ?? "";
  const map: Record<string, string> = {
    ts: "typescript",
    tsx: "typescript",
    js: "javascript",
    jsx: "javascript",
    mjs: "javascript",
    cjs: "javascript",
    json: "json",
    md: "markdown",
    markdown: "markdown",
    rs: "rust",
    toml: "ini",
    yml: "yaml",
    yaml: "yaml",
    css: "css",
    scss: "scss",
    less: "less",
    html: "html",
    htm: "html",
    xml: "xml",
    svg: "xml",
    sh: "shell",
    bash: "shell",
    zsh: "shell",
    py: "python",
    go: "go",
    java: "java",
    c: "c",
    h: "c",
    cpp: "cpp",
    hpp: "cpp",
    cs: "csharp",
    rb: "ruby",
    php: "php",
    sql: "sql",
    dockerfile: "dockerfile",
  };
  if (fileName.toLowerCase() === "dockerfile") return "dockerfile";
  if (fileName.toLowerCase() === "makefile") return "makefile";
  return map[ext] ?? "plaintext";
}

export default function DiffViewer({
  original = "",
  modified = "",
  language,
  fileName,
}: DiffViewerProps) {
  return (
    <div className="h-full w-full">
      <DiffEditor
        height="100%"
        language={language ?? languageForFile(fileName)}
        original={original}
        modified={modified}
        theme="vs-dark"
        options={{
          readOnly: true,
          renderSideBySide: true,
          minimap: { enabled: false },
          scrollBeyondLastLine: false,
          fontSize: 12,
          wordWrap: "on",
          renderOverviewRuler: false,
        }}
      />
    </div>
  );
}
