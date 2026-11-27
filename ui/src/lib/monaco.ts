import * as monaco from "monaco-editor/esm/vs/editor/edcore.main";
import { loader } from "@monaco-editor/react";
// Tokenizers only. They give syntax highlighting for every language and are
// small. The TypeScript, CSS, HTML and JSON language services are 7.3 MB of
// workers that provide IntelliSense, which a read-only diff view never uses.
import "monaco-editor/esm/vs/basic-languages/monaco.contribution";
import editorWorker from "monaco-editor/esm/vs/editor/editor.worker?worker";

// Bundle Monaco locally so the app works fully offline (no CDN fetch).
(self as unknown as { MonacoEnvironment: unknown }).MonacoEnvironment = {
  getWorker() {
    return new editorWorker();
  },
};

loader.config({ monaco });

export const monacoLoaderConfigured = true;
