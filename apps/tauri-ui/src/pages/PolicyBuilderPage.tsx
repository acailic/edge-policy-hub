import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { Editor } from "@monaco-editor/react";
import type { editor } from "monaco-editor";
import { useQuery } from "@tanstack/react-query";
import {
  FileCode2,
  History,
  Loader2,
  Play,
  ShieldPlus,
  Sparkles,
  TestTube,
} from "lucide-react";
import { useParams } from "react-router-dom";

import { getTenant, listPolicyBundles } from "../lib/api";
import { usePolicyCompilation } from "../hooks/usePolicyCompilation";
import { usePolicyDeployment } from "../hooks/usePolicyDeployment";
import type {
  CompilationError,
  PolicyBundle,
  PolicyMetadata,
} from "../types/policy";
import { PolicyEditor } from "../components/PolicyEditor";
import { PolicyMetadataForm } from "../components/PolicyMetadataForm";
import { PolicyTemplateSelector } from "../components/PolicyTemplateSelector";
import { TestSimulator } from "../components/TestSimulator";
import { PolicyVersionHistory } from "../components/PolicyVersionHistory";
import { CompilationStatusBar } from "../components/CompilationStatusBar";
import { DeploymentDialog } from "../components/DeploymentDialog";
import { PolicyDiffViewer } from "../components/PolicyDiffViewer";

import "../styles/policy-builder.css";

type SidePanelTab = "rego" | "test" | "history";

function createDefaultMetadata(): PolicyMetadata {
  return {
    version: "1.0.0",
    author: undefined,
    description: undefined,
    created_at: new Date().toISOString(),
  };
}

function PolicyBuilderPage() {
  const { id: tenantId } = useParams<{ id: string }>();
  const [metadata, setMetadata] = useState<PolicyMetadata>(
    createDefaultMetadata,
  );
  const [activeTab, setActiveTab] = useState<SidePanelTab>("rego");
  const [isDeploymentDialogOpen, setDeploymentDialogOpen] = useState(false);
  const [selectedBundle, setSelectedBundle] = useState<PolicyBundle | null>(
    null,
  );
  const [regoView, setRegoView] = useState<"compiled" | "selected" | "diff">(
    "compiled",
  );

  const editorRef = useRef<editor.IStandaloneCodeEditor | null>(null);

  const {
    data: tenant,
    isLoading: tenantLoading,
    isError: tenantError,
    error: tenantErrorValue,
  } = useQuery({
    queryKey: ["tenant", tenantId],
    queryFn: () => getTenant(tenantId ?? ""),
    enabled: Boolean(tenantId),
  });

  const compilation = usePolicyCompilation(tenantId ?? "", "");
  const deployment = usePolicyDeployment(tenantId ?? "");
  const { data: bundles } = useQuery({
    queryKey: ["policy-bundles", tenantId],
    queryFn: () => listPolicyBundles(tenantId ?? ""),
    enabled: Boolean(tenantId),
  });

  const activeBundle = bundles?.find((bundle) => bundle.status === "active");
  const maxVersion = bundles?.reduce((max, bundle) => Math.max(max, bundle.version), 0);

  const handleCompile = useCallback(() => {
    compilation.compile({ metadata });
  }, [compilation, metadata]);

  useEffect(() => {
    if (!tenantId) {
      return;
    }

    const handler = (event: KeyboardEvent) => {
      const metaKey = event.metaKey || event.ctrlKey;
      if (!metaKey) {
        return;
      }

      const key = event.key.toLowerCase();
      if (key === "s") {
        event.preventDefault();
        handleCompile();
      } else if (event.shiftKey && key === "d") {
        event.preventDefault();
        if (!compilation.hasErrors) {
          setDeploymentDialogOpen(true);
        }
      } else if (key === "t") {
        event.preventDefault();
        setActiveTab((tab) => (tab === "test" ? "rego" : "test"));
      }
    };

    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [compilation.hasErrors, handleCompile, tenantId]);

  const handleFocusError = useCallback(
    (error: CompilationError) => {
      if (!editorRef.current) {
        return;
      }

      const line = error.line ?? 1;
      const column = error.column ?? 1;
      editorRef.current.revealLineInCenter(line);
      editorRef.current.setPosition({ lineNumber: line, column });
      editorRef.current.focus();
    },
    [],
  );

  const handleSelectTemplate = useCallback(
    (dsl: string, description: string) => {
      compilation.setSource(dsl);
      setMetadata((current) => ({
        ...current,
        description,
        created_at: new Date().toISOString(),
      }));
      setSelectedBundle(null);
      setRegoView("compiled");
    },
    [compilation],
  );

  const handleDeploy = useCallback(
    async (activate: boolean) => {
      if (!compilation.compiledRego) {
        alert("Compile the policy before deploying.");
        return;
      }

      await deployment.deploy(compilation.compiledRego, metadata, activate);
      setDeploymentDialogOpen(false);
    },
    [compilation.compiledRego, deployment, metadata],
  );

  const handleSelectBundle = useCallback((bundle: PolicyBundle) => {
    setSelectedBundle(bundle);
    setActiveTab("rego");
    setRegoView("selected");
  }, []);

  const compiledRegoToShow = useMemo(() => {
    if (regoView === "compiled") {
      return compilation.compiledRego ?? "";
    }

    if (regoView === "selected" && selectedBundle) {
      return selectedBundle.rego_code;
    }

    return compilation.compiledRego ?? "";
  }, [compilation.compiledRego, regoView, selectedBundle]);

  if (!tenantId) {
    return <p>Tenant ID missing from route.</p>;
  }

  if (tenantLoading) {
    return (
      <div className="policy-builder-loading">
        <Loader2 className="spin" /> Loading tenant details…
      </div>
    );
  }

  if (tenantError) {
    return (
      <div className="policy-builder-error">
        Failed to load tenant:{" "}
        {tenantErrorValue instanceof Error
          ? tenantErrorValue.message
          : "Unknown error"}
      </div>
    );
  }

  return (
    <div className="policy-builder">
      <header className="policy-builder__header">
        <div>
          <h2>Policy Builder · {tenant?.name ?? tenantId}</h2>
          <p>
            Author, test, and deploy ABAC policies for this tenant. Use the
            Monaco editor for DSL authoring, then validate with the simulator
            before deployment.
          </p>
          {activeBundle && (
            <p className="active-version-indicator">
              Active version: v{activeBundle.version} ·{" "}
              {activeBundle.metadata?.description ?? "No description"}
            </p>
          )}
        </div>
        <div className="builder-actions">
          <button type="button" onClick={handleCompile}>
            <Sparkles /> Compile
          </button>
          <button
            type="button"
            onClick={() => setActiveTab("test")}
            className={activeTab === "test" ? "active" : undefined}
          >
            <TestTube /> Test
          </button>
          <button
            type="button"
            onClick={() => setActiveTab("history")}
            className={activeTab === "history" ? "active" : undefined}
          >
            <History /> Versions
          </button>
          <button
            type="button"
            className="primary"
            disabled={compilation.hasErrors}
            onClick={() => setDeploymentDialogOpen(true)}
          >
            <ShieldPlus /> Deploy
          </button>
        </div>
      </header>

      <div className="policy-builder__layout">
        <div className="editor-panel">
          <div className="editor-toolbar">
            <PolicyMetadataForm metadata={metadata} onChange={setMetadata} />
          </div>
          <PolicyEditor
            value={compilation.dslSource}
            onChange={compilation.setSource}
            errors={compilation.compilationErrors}
            onEditorReady={(instance) => {
              editorRef.current = instance;
            }}
          />
          <CompilationStatusBar
            isCompiling={compilation.isCompiling}
            errors={compilation.compilationErrors}
            lastCompiled={compilation.lastCompiled}
            compiledRego={compilation.compiledRego}
            onFocusError={handleFocusError}
          />
        </div>

        <aside className="side-panel">
          <nav className="side-tabs">
            <button
              type="button"
              className={activeTab === "rego" ? "active" : undefined}
              onClick={() => setActiveTab("rego")}
            >
              <FileCode2 /> Compiled Rego
            </button>
            <button
              type="button"
              className={activeTab === "test" ? "active" : undefined}
              onClick={() => setActiveTab("test")}
            >
              <Play /> Test Simulator
            </button>
            <button
              type="button"
              className={activeTab === "history" ? "active" : undefined}
              onClick={() => setActiveTab("history")}
            >
              <History /> Version History
            </button>
          </nav>

          <div className="side-panel__content">
            {activeTab === "rego" && (
              <div className="rego-viewer">
                {selectedBundle && (
                  <div className="rego-viewer__bundle">
                    <span>
                      Viewing bundle v{selectedBundle.version} ·{" "}
                      {selectedBundle.status}
                    </span>
                    <div className="rego-viewer__toggles">
                      <button
                        type="button"
                        className={regoView === "compiled" ? "active" : undefined}
                        onClick={() => setRegoView("compiled")}
                      >
                        Current
                      </button>
                      <button
                        type="button"
                        className={regoView === "selected" ? "active" : undefined}
                        onClick={() => setRegoView("selected")}
                      >
                        Selected
                      </button>
                      <button
                        type="button"
                        className={regoView === "diff" ? "active" : undefined}
                        onClick={() => setRegoView("diff")}
                        disabled={!compilation.compiledRego}
                      >
                        Diff
                      </button>
                    </div>
                  </div>
                )}

                {regoView === "diff" && selectedBundle && compilation.compiledRego ? (
                  <PolicyDiffViewer
                    originalRego={selectedBundle.rego_code}
                    modifiedRego={compilation.compiledRego}
                    originalVersion={selectedBundle.version}
                    modifiedVersion={selectedBundle.version + 1}
                  />
                ) : (
                  <Editor
                    value={compiledRegoToShow}
                    language="rego"
                    theme="vs-dark"
                    height="100%"
                    options={{
                      readOnly: true,
                      minimap: { enabled: false },
                      scrollBeyondLastLine: false,
                      automaticLayout: true,
                    }}
                  />
                )}
              </div>
            )}

            {activeTab === "test" && tenantId && (
              <TestSimulator tenantId={tenantId} />
            )}

            {activeTab === "history" && (
              <PolicyVersionHistory
                tenantId={tenantId}
                onSelectVersion={handleSelectBundle}
              />
            )}
          </div>
        </aside>
      </div>

      <section className="policy-builder__templates">
        <h3>
          <Sparkles /> Templates
        </h3>
        <PolicyTemplateSelector onSelectTemplate={handleSelectTemplate} />
      </section>

      {deployment.deploymentError && (
        <div className="policy-builder__feedback error">
          {deployment.deploymentError.message}
        </div>
      )}

      {deployment.deploymentResult && (
        <div className="policy-builder__feedback success">
          Deployed bundle {deployment.deploymentResult.bundle_id} (v
          {deployment.deploymentResult.version})
        </div>
      )}

      <DeploymentDialog
        isOpen={isDeploymentDialogOpen}
        onClose={() => setDeploymentDialogOpen(false)}
        onConfirm={handleDeploy}
        tenantId={tenantId}
        maxVersion={maxVersion}
        activeVersion={activeBundle?.version}
        hasErrors={compilation.hasErrors}
        metadata={metadata}
      />
    </div>
  );
}

export default PolicyBuilderPage;
