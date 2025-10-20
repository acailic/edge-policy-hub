import { useEffect, useMemo, useRef, useState } from "react";
import {
  Loader2,
  Pause,
  Play,
  RefreshCcw,
  Wifi,
  WifiOff,
} from "lucide-react";

import { useDecisionStream } from "../hooks/useDecisionStream";
import type { DecisionEvent } from "../types/monitoring";
import { DecisionEventCard } from "./DecisionEventCard";

interface DecisionStreamFeedProps {
  tenantId?: string;
  maxItems?: number;
  onNewDecision?: (event: DecisionEvent) => void;
}

type DecisionFilter = "all" | "allow" | "deny";
type ProtocolFilter = "all" | "http" | "mqtt";

function extractProtocol(event: DecisionEvent): string {
  const input = event.input ?? {};
  if (typeof input.protocol === "string") {
    return input.protocol.toLowerCase();
  }

  const environment = input.environment as Record<string, unknown> | undefined;
  if (environment && typeof environment.protocol === "string") {
    return environment.protocol.toLowerCase();
  }

  const action = typeof input.action === "string" ? input.action : "";
  if (["publish", "subscribe"].some((value) => action.includes(value))) {
    return "mqtt";
  }

  return "http";
}

export function DecisionStreamFeed({
  tenantId,
  maxItems = 50,
  onNewDecision,
}: DecisionStreamFeedProps) {
  const [paused, setPaused] = useState(false);
  const [decisionFilter, setDecisionFilter] =
    useState<DecisionFilter>("all");
  const [protocolFilter, setProtocolFilter] =
    useState<ProtocolFilter>("all");
  const [displayedDecisions, setDisplayedDecisions] = useState<
    DecisionEvent[]
  >([]);

  const serverDecisionFilter = decisionFilter === "all" ? undefined : decisionFilter;
  const { decisions, connectionStatus, isConnected } =
    useDecisionStream(tenantId, true, serverDecisionFilter);

  const lastNotifiedId = useRef<string | null>(null);

  useEffect(() => {
    if (paused) {
      return;
    }
    setDisplayedDecisions(decisions.slice(0, maxItems));
    const newest = decisions[0];
    if (newest && newest.event_id !== lastNotifiedId.current) {
      lastNotifiedId.current = newest.event_id;
      onNewDecision?.(newest);
    }
  }, [decisions, paused, maxItems, onNewDecision]);

  const filtered = useMemo(() => {
    return displayedDecisions.filter((event) => {
      if (decisionFilter !== "all") {
        const allow = decisionFilter === "allow";
        if (event.decision.allow !== allow) {
          return false;
        }
      }

      if (protocolFilter !== "all") {
        const protocol = extractProtocol(event);
        if (protocol !== protocolFilter) {
          return false;
        }
      }

      return true;
    });
  }, [displayedDecisions, decisionFilter, protocolFilter]);

  const statusIndicator = useMemo(() => {
    switch (connectionStatus) {
      case "connected":
        return (
          <span className="decision-stream__status decision-stream__status--online">
            <Wifi size={16} /> Connected
          </span>
        );
      case "connecting":
        return (
          <span className="decision-stream__status decision-stream__status--connecting">
            <Loader2 className="spin" size={16} /> Connecting…
          </span>
        );
      case "error":
        return (
          <span className="decision-stream__status decision-stream__status--error">
            <WifiOff size={16} /> Error
          </span>
        );
      default:
        return (
          <span className="decision-stream__status decision-stream__status--offline">
            <WifiOff size={16} /> Disconnected
          </span>
        );
    }
  }, [connectionStatus]);

  return (
    <section className="decision-stream">
      <header className="decision-stream__header">
        <div>
          <h3>Decision Stream</h3>
          {statusIndicator}
        </div>
        <div className="decision-stream__actions">
          <button
            type="button"
            onClick={() => setPaused((prev) => !prev)}
            aria-pressed={paused}
            title={paused ? "Resume stream" : "Pause stream"}
          >
            {paused ? <Play size={16} /> : <Pause size={16} />}
            {paused ? "Resume" : "Pause"}
          </button>
          <button
            type="button"
            onClick={() => setDisplayedDecisions([])}
            disabled={!displayedDecisions.length}
            title="Clear displayed decisions"
          >
            <RefreshCcw size={16} /> Clear
          </button>
        </div>
      </header>

      <div className="decision-stream__filters">
        <div className="decision-stream__filter">
          <label htmlFor="decision-filter">Decision</label>
          <select
            id="decision-filter"
            value={decisionFilter}
            onChange={(event) =>
              setDecisionFilter(event.target.value as DecisionFilter)
            }
          >
            <option value="all">All</option>
            <option value="allow">Allow</option>
            <option value="deny">Deny</option>
          </select>
        </div>

        <div className="decision-stream__filter">
          <label htmlFor="protocol-filter">Protocol</label>
          <select
            id="protocol-filter"
            value={protocolFilter}
            onChange={(event) =>
              setProtocolFilter(event.target.value as ProtocolFilter)
            }
          >
            <option value="all">All</option>
            <option value="http">HTTP</option>
            <option value="mqtt">MQTT</option>
          </select>
        </div>
      </div>

      <div className="decision-stream__list">
        {filtered.length === 0 && (
          <div className="decision-stream__empty">
            {isConnected
              ? "Waiting for policy decisions…"
              : "No decisions received yet."}
          </div>
        )}
        {filtered.map((event) => (
          <DecisionEventCard key={event.event_id} event={event} compact />
        ))}
      </div>
    </section>
  );
}
