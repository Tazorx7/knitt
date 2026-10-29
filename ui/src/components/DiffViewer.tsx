import Editor from "@monaco-editor/react";

type DiffViewerProps = {
  original?: string;
  modified?: string;
  language?: string;
};

export default function DiffViewer({ modified = "", language = "plaintext" }: DiffViewerProps) {
  return (
    <div className="h-full w-full">
      <Editor
        height="100%"
        language={language}
        value={modified}
        theme="vs-dark"
        options={{
          readOnly: true,
          minimap: { enabled: false },
          scrollBeyondLastLine: false,
        }}
      />
    </div>
  );
}
