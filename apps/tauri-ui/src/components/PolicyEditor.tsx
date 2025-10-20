import { useCallback, useEffect, useRef } from "react";
import type { Monaco, OnMount } from "@monaco-editor/react";
import { Editor } from "@monaco-editor/react";
import type { editor } from "monaco-editor";

import type { CompilationError } from "../types/policy";
import {
  createErrorMarkers,
  registerPolicyDslLanguage,
} from "../utils/monaco-dsl-language";

interface PolicyEditorProps {
  value: string;
  onChange: (value: string) => void;
  errors?: CompilationError[];
  readOnly?: boolean;
  height?: string;
  onEditorReady?: (editor: editor.IStandaloneCodeEditor) => void;
}

export function PolicyEditor({
  value,
  onChange,
  errors,
  readOnly = false,
  height = "500px",
  onEditorReady,
}: PolicyEditorProps) {
  const editorRef = useRef<editor.IStandaloneCodeEditor | null>(null);
  const monacoRef = useRef<Monaco | null>(null);

  const handleMount = useCallback<OnMount>((editorInstance, monacoInstance) => {
    editorRef.current = editorInstance;
    monacoRef.current = monacoInstance;

    registerPolicyDslLanguage(monacoInstance);
    editorInstance.updateOptions({
      wordWrap: "on",
      tabSize: 2,
    });

    onEditorReady?.(editorInstance);
  }, [onEditorReady]);

  useEffect(() => {
    if (!editorRef.current || !monacoRef.current) {
      return;
    }

    const model = editorRef.current.getModel();
    if (!model) {
      return;
    }

    const markers = createErrorMarkers(monacoRef.current, errors ?? []);
    monacoRef.current.editor.setModelMarkers(model, "policy-dsl", markers);
  }, [errors]);

  const handleChange = useCallback(
    (nextValue: string | undefined) => {
      onChange(nextValue ?? "");
    },
    [onChange],
  );

  return (
    <Editor
      value={value}
      language="policy-dsl"
      theme="vs-dark"
      height={height}
      onChange={handleChange}
      onMount={handleMount}
      options={{
        readOnly,
        minimap: { enabled: false },
        lineNumbers: "on",
        scrollBeyondLastLine: false,
        automaticLayout: true,
        renderValidationDecorations: "on",
      }}
    />
  );
}
