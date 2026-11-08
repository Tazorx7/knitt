import { useEffect, useRef } from "react";
import { DiffEditor } from "@monaco-editor/react";
import type { editor } from "monaco-editor";
import "../lib/monaco";

type DiffViewerProps = {
  original?: string;
  modified?: string;
  language?: string;
  fileName?: string;
  tabSize?: number;
  dark?: boolean;
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
  tabSize = 4,
  dark = false,
}: DiffViewerProps) {
  // Monaco treats tab size as a model option, not an editor option, so it has
  // to be applied after mount and again whenever the setting or content changes.
  const editorRef = useRef<editor.IStandaloneDiffEditor | null>(null);

  function applyTabSize(instance: editor.IStandaloneDiffEditor) {
    const models = instance.getModel();
    models?.original.updateOptions({ tabSize, insertSpaces: true });
    models?.modified.updateOptions({ tabSize, insertSpaces: true });
  }

  useEffect(() => {
    if (editorRef.current) applyTabSize(editorRef.current);
  }, [tabSize, original, modified]);

  return (
    <div className="h-full w-full">
      <DiffEditor
        height="100%"
        language={language ?? languageForFile(fileName)}
        original={original}
        modified={modified}
        theme={dark ? "vs-dark" : "vs-light"}
        onMount={(instance) => {
          editorRef.current = instance;
          applyTabSize(instance);
        }}
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
