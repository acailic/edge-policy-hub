import { useEffect, useRef } from "react";

import { useDecisionStream } from "../hooks/useDecisionStream";
import { useNotifications } from "../hooks/useNotifications";
import type { DecisionEvent } from "../types/monitoring";

interface PolicyViolationMonitorProps {
  tenantId?: string;
  enabled?: boolean;
  onViolation?: (event: DecisionEvent) => void;
}

const VIOLATION_DEBOUNCE = 10_000;

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

export function PolicyViolationMonitor({
  tenantId,
  enabled = true,
  onViolation,
}: PolicyViolationMonitorProps) {
  const { decisions } = useDecisionStream(tenantId, enabled, "deny");
  const { notifyPolicyViolation } = useNotifications();
  const notifiedEvents = useRef<Set<string>>(new Set());
  const lastNotification = useRef<number>(0);

  useEffect(() => {
    if (!enabled) {
      return;
    }

    const now = Date.now();
    for (const event of decisions) {
      if (event.decision.allow) {
        continue;
      }

      if (notifiedEvents.current.has(event.event_id)) {
        continue;
      }

      if (now - lastNotification.current < VIOLATION_DEBOUNCE) {
        continue;
      }

      const action = extractAction(event.input);
      notifyPolicyViolation(
        event.tenant_id,
        action,
        event.decision.reason ?? undefined,
      );
      onViolation?.(event);

      notifiedEvents.current.add(event.event_id);
      lastNotification.current = now;

      if (notifiedEvents.current.size > 1_000) {
        const iterator = notifiedEvents.current.values();
        const first = iterator.next();
        if (!first.done && first.value) {
          notifiedEvents.current.delete(first.value);
        }
      }
    }
  }, [decisions, enabled, notifyPolicyViolation, onViolation]);

  return null;
}
