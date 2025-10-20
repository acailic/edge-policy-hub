import { DiffEditor } from "@monaco-editor/react";

interface PolicyDiffViewerProps {
  originalRego: string;
  modifiedRego: string;
  originalVersion: number;
  modifiedVersion: number;
}

export function PolicyDiffViewer({
  originalRego,
  modifiedRego,
  originalVersion,
  modifiedVersion,
}: PolicyDiffViewerProps) {
  return (
    <div className="policy-diff-viewer">
      <header className="policy-diff-viewer__header">
        <h3>
          Comparing v{originalVersion} → v{modifiedVersion}
        </h3>
        <p>Green = added lines · Red = removed lines · Blue = modified lines</p>
      </header>
      <DiffEditor
        original={originalRego}
        modified={modifiedRego}
        language="rego"
        theme="vs-dark"
        options={{
          readOnly: true,
          renderSideBySide: true,
          minimap: { enabled: false },
          scrollBeyondLastLine: false,
        }}
        height="400px"
      />
    </div>
  );
}
