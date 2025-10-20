import { useMemo, useState } from "react";
import {
  CheckCircle,
  ChevronDown,
  ChevronUp,
  Clock,
  Shield,
  XCircle,
} from "lucide-react";

import type { DecisionEvent } from "../types/monitoring";
import { formatRelativeTime } from "../utils/date-helpers";

interface DecisionEventCardProps {
  event: DecisionEvent;
  compact?: boolean;
}

function extractAction(input: Record<string, unknown>): string {
  if (typeof input.action === "string") {
    return input.action;
  }
  const resource = input.resource as Record<string, unknown> | undefined;
  if (resource && typeof resource.action === "string") {
    return resource.action;
  }
  return "unknown";
}

function extractResource(input: Record<string, unknown>): string {
  const resource = input.resource as Record<string, unknown> | undefined;
  if (!resource) {
    return "unknown";
  }

  if (typeof resource.type === "string") {
    return resource.type;
  }

  if (typeof resource.kind === "string") {
    return resource.kind;
  }

  return "resource";
}

export function DecisionEventCard({
  event,
  compact = false,
}: DecisionEventCardProps) {
  const [expanded, setExpanded] = useState(false);
  const decision = event.decision;

  const action = useMemo(() => extractAction(event.input), [event.input]);
  const resource = useMemo(() => extractResource(event.input), [event.input]);
  const relativeTimestamp = useMemo(
    () => formatRelativeTime(event.timestamp),
    [event.timestamp],
  );

  const redactTargets = decision.redact?.filter(Boolean) ?? [];
  const statusTone = decision.allow ? "allow" : "deny";

  return (
    <article className={`decision-event decision-event--${statusTone}`}>
      <header className="decision-event__header">
        <span className={`decision-event__badge decision-event__badge--${statusTone}`}>
          {decision.allow ? (
            <>
              <CheckCircle size={16} /> Allow
            </>
          ) : (
            <>
              <XCircle size={16} /> Deny
            </>
          )}
        </span>
        <span className="decision-event__tenant">{event.tenant_id}</span>
        <span className="decision-event__time">{relativeTimestamp}</span>
      </header>

      {!compact && (
        <div className="decision-event__summary">
          <div>
            <strong>Action:</strong> {action}
          </div>
          <div>
            <strong>Resource:</strong> {resource}
          </div>
          <div>
            <strong>Latency:</strong> {event.metrics.eval_duration_micros.toLocaleString()} μs
          </div>
        </div>
      )}

      {!decision.allow && decision.reason && (
        <div className="decision-event__reason">
          <Shield size={16} />
          <span>{decision.reason}</span>
        </div>
      )}

      {redactTargets.length > 0 && (
        <div className="decision-event__redactions">
          <Shield size={16} />
          <span>Redacted: {redactTargets.join(", ")}</span>
        </div>
      )}

      <footer className="decision-event__footer">
        <div className="decision-event__meta">
          <Clock size={14} />
          <span>{new Date(event.timestamp).toLocaleString()}</span>
        </div>
        <button
          type="button"
          className="decision-event__toggle"
          onClick={() => setExpanded((prev) => !prev)}
          aria-expanded={expanded}
        >
          {expanded ? (
            <>
              Hide details <ChevronUp size={14} />
            </>
          ) : (
            <>
              View details <ChevronDown size={14} />
            </>
          )}
        </button>
      </footer>

      {expanded && (
        <div className="decision-event__details">
          <div className="decision-event__details-grid">
            <div>
              <h4>Event ID</h4>
              <code>{event.event_id}</code>
            </div>
            <div>
              <h4>Decision Payload</h4>
              <pre>{JSON.stringify(decision, null, 2)}</pre>
            </div>
            <div>
              <h4>Input</h4>
              <pre>{JSON.stringify(event.input, null, 2)}</pre>
            </div>
          </div>
        </div>
      )}
    </article>
  );
}
