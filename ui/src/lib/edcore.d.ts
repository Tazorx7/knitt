// edcore.main is the editor and the diff view without the language-service
// contributions. The package ships no .d.ts for that path; its public API is
// the same as the package root.
declare module "monaco-editor/esm/vs/editor/edcore.main" {
  export * from "monaco-editor";
}
