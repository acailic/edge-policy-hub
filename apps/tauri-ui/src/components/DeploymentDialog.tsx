import { useEffect, useMemo, useState } from "react";
import { AlertTriangle, Rocket } from "lucide-react";

import type { PolicyMetadata } from "../types/policy";

interface DeploymentDialogProps {
  isOpen: boolean;
  onClose: () => void;
  onConfirm: (activate: boolean) => void;
  tenantId: string;
  maxVersion?: number;
  activeVersion?: number;
  hasErrors: boolean;
  metadata?: PolicyMetadata;
}

export function DeploymentDialog({
  isOpen,
  onClose,
  onConfirm,
  tenantId,
  maxVersion,
  activeVersion,
  hasErrors,
  metadata,
}: DeploymentDialogProps) {
  const [mode, setMode] = useState<"draft" | "active">("draft");
  const [acknowledged, setAcknowledged] = useState(false);

  useEffect(() => {
    if (isOpen) {
      setMode("draft");
      setAcknowledged(false);
    }
  }, [isOpen]);

  const nextVersion = useMemo(() => {
    if (typeof maxVersion === "number" && maxVersion > 0) {
      return maxVersion + 1;
    }

    return 1;
  }, [maxVersion]);

  if (!isOpen) {
    return null;
  }

  const disableConfirm =
    hasErrors || (mode === "active" && acknowledged === false);

  const handleConfirm = () => {
    if (disableConfirm) {
      return;
    }

    onConfirm(mode === "active");
  };

  return (
    <div className="deployment-dialog__backdrop" role="presentation">
      <div
        className="deployment-dialog"
        role="dialog"
        aria-modal="true"
        aria-labelledby="deployment-dialog-title"
      >
        <header className="deployment-dialog__header">
          <h2 id="deployment-dialog-title">
            Deploy Policy for {tenantId}
          </h2>
          <button type="button" onClick={onClose} aria-label="Close dialog">
            ×
          </button>
        </header>

        <div className="deployment-dialog__content">
          {hasErrors && (
            <div className="deployment-dialog__warning">
              <AlertTriangle />
              <div>
                <strong>Resolve compilation errors before deploying.</strong>
                <p>
                  The current policy still has compilation issues. Fix them and
                  re-run the compiler before deployment.
                </p>
              </div>
            </div>
          )}

          <section className="deployment-dialog__section">
            <h3>Deployment Mode</h3>
            <label className="deployment-dialog__option">
              <input
                type="radio"
                name="deployment-mode"
                value="draft"
                checked={mode === "draft"}
                onChange={() => setMode("draft")}
              />
              <div>
                <strong>Deploy as Draft</strong>
                <p>
                  Saves the compiled policy for later activation. Use this to
                  review or test the policy without affecting production traffic.
                </p>
              </div>
            </label>

            <label className="deployment-dialog__option">
              <input
                type="radio"
                name="deployment-mode"
                value="active"
                checked={mode === "active"}
                onChange={() => setMode("active")}
              />
              <div>
                <strong>Deploy &amp; Activate</strong>
                <p>
                  Immediately enforces the new policy for tenant {tenantId}.
                  Existing traffic will be evaluated against this version.
                </p>
              </div>
            </label>
          </section>

          <section className="deployment-dialog__section">
            <h3>Version Details</h3>
            <ul>
              <li>
                Current active version:{" "}
                {typeof activeVersion === "number" ? `v${activeVersion}` : "None"}
              </li>
              <li>New bundle version: v{nextVersion}</li>
            </ul>
          </section>

          {metadata && (
            <section className="deployment-dialog__section">
              <h3>Metadata</h3>
              <dl className="deployment-dialog__metadata">
                <div>
                  <dt>Version</dt>
                  <dd>{metadata.version}</dd>
                </div>
                {metadata.author && (
                  <div>
                    <dt>Author</dt>
                    <dd>{metadata.author}</dd>
                  </div>
                )}
                {metadata.description && (
                  <div>
                    <dt>Description</dt>
                    <dd>{metadata.description}</dd>
                  </div>
                )}
              </dl>
            </section>
          )}

          {mode === "active" && (
            <div className="deployment-dialog__confirmation">
              <Rocket />
              <label>
                <input
                  type="checkbox"
                  checked={acknowledged}
                  onChange={(event) => setAcknowledged(event.target.checked)}
                />
                I have tested this policy and understand it will take effect
                immediately.
              </label>
            </div>
          )}
        </div>

        <footer className="deployment-dialog__footer">
          <button type="button" onClick={onClose}>
            Cancel
          </button>
          <button
            type="button"
            className="primary"
            onClick={handleConfirm}
            disabled={disableConfirm}
          >
            Deploy
          </button>
        </footer>
      </div>
    </div>
  );
}
