import { useCallback } from "react";
import { Copy, Shield, X } from "lucide-react";

import type { AuditLogEntry } from "../types/monitoring";
import { formatTimestamp } from "../utils/date-helpers";

interface AuditLogDetailsModalProps {
  log: AuditLogEntry | null;
  isOpen: boolean;
  onClose: () => void;
}

export function AuditLogDetailsModal({
  log,
  isOpen,
  onClose,
}: AuditLogDetailsModalProps) {
  if (!isOpen || !log) {
    return null;
  }

  const copyToClipboard = useCallback(
    async (value: unknown) => {
      try {
        await navigator.clipboard.writeText(
          typeof value === "string" ? value : JSON.stringify(value, null, 2),
        );
      } catch (error) {
        console.error("Failed to copy audit log value", error);
      }
    },
    [],
  );

  return (
    <div className="audit-log-modal" role="dialog" aria-modal="true">
      <div className="audit-log-modal__backdrop" onClick={onClose} />
      <div className="audit-log-modal__content">
        <header className="audit-log-modal__header">
          <h3>Audit Log Details</h3>
          <button type="button" onClick={onClose} aria-label="Close dialog">
            <X size={18} />
          </button>
        </header>

        <section className="audit-log-modal__section">
          <div>
            <h4>Log ID</h4>
            <div className="audit-log-modal__code">
              <code>{log.log_id}</code>
              <button type="button" onClick={() => copyToClipboard(log.log_id)}>
                <Copy size={14} />
              </button>
            </div>
          </div>
          <div>
            <h4>Timestamp</h4>
            <p>{formatTimestamp(log.timestamp)}</p>
          </div>
          <div>
            <h4>Decision</h4>
            <span
              className={`audit-log-modal__badge audit-log-modal__badge--${log.decision.toLowerCase()}`}
            >
              {log.decision.toUpperCase()}
            </span>
          </div>
          <div>
            <h4>Protocol</h4>
            <p>{log.protocol.toUpperCase()}</p>
          </div>
          <div>
            <h4>Policy Version</h4>
            <p>{log.policy_version ? `v${log.policy_version}` : "—"}</p>
          </div>
        </section>

        {log.reason && (
          <section className="audit-log-modal__section audit-log-modal__reason">
            <Shield size={16} />
            <p>{log.reason}</p>
          </section>
        )}

        <section className="audit-log-modal__section audit-log-modal__attributes">
          <div>
            <header>
              <h4>Subject</h4>
              <button type="button" onClick={() => copyToClipboard(log.subject)}>
                <Copy size={14} />
              </button>
            </header>
            <pre>{JSON.stringify(log.subject, null, 2)}</pre>
          </div>
          <div>
            <header>
              <h4>Resource</h4>
              <button type="button" onClick={() => copyToClipboard(log.resource)}>
                <Copy size={14} />
              </button>
            </header>
            <pre>{JSON.stringify(log.resource, null, 2)}</pre>
          </div>
          <div>
            <header>
              <h4>Environment</h4>
              <button
                type="button"
                onClick={() => copyToClipboard(log.environment)}
              >
                <Copy size={14} />
              </button>
            </header>
            <pre>{JSON.stringify(log.environment, null, 2)}</pre>
          </div>
        </section>

        <section className="audit-log-modal__section">
          <div>
            <h4>Signature</h4>
            <div className="audit-log-modal__code">
              <code>{log.signature}</code>
              <button type="button" onClick={() => copyToClipboard(log.signature)}>
                <Copy size={14} />
              </button>
            </div>
          </div>
          <div>
            <h4>Uploaded</h4>
            <p>{log.uploaded ? "Yes" : "No"}</p>
          </div>
        </section>
      </div>
    </div>
  );
}
