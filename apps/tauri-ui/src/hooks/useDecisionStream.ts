import { useEffect, useMemo, useRef, useState } from "react";
import useWebSocket, { ReadyState } from "react-use-websocket";

import type { DecisionEvent } from "../types/monitoring";

export type DecisionStreamStatus =
  | "connecting"
  | "connected"
  | "disconnected"
  | "error";

const STREAM_ENDPOINT = "ws://127.0.0.1:8181/v1/stream/decisions";
const MAX_EVENTS = 100;

export function useDecisionStream(
  tenantId?: string,
  enabled: boolean = true,
) {
  const didUnmount = useRef(false);
  const [decisions, setDecisions] = useState<DecisionEvent[]>([]);
  const [connectionStatus, setConnectionStatus] =
    useState<DecisionStreamStatus>(enabled ? "connecting" : "disconnected");

  useEffect(() => {
    return () => {
      didUnmount.current = true;
    };
  }, []);

  const streamUrl = useMemo(() => {
    if (!enabled) {
      return null;
    }
    if (tenantId && tenantId.length > 0) {
      const params = new URLSearchParams({ tenant_id: tenantId });
      return `${STREAM_ENDPOINT}?${params.toString()}`;
    }
    return STREAM_ENDPOINT;
  }, [tenantId, enabled]);

  const { sendMessage, lastMessage, readyState } = useWebSocket(
    streamUrl,
    {
      shouldReconnect: () => enabled && !didUnmount.current,
      retryOnError: true,
      reconnectAttempts: 20,
      reconnectInterval: (attemptNumber) =>
        Math.min(1000 * Math.pow(2, attemptNumber), 10_000),
      onOpen: () => setConnectionStatus("connected"),
      onClose: () => setConnectionStatus("disconnected"),
      onError: () => setConnectionStatus("error"),
    },
    enabled,
  );

  useEffect(() => {
    if (!enabled) {
      setDecisions([]);
      setConnectionStatus("disconnected");
      return;
    }

    switch (readyState) {
      case ReadyState.CONNECTING:
        setConnectionStatus("connecting");
        break;
      case ReadyState.OPEN:
        setConnectionStatus("connected");
        break;
      case ReadyState.CLOSING:
      case ReadyState.CLOSED:
        setConnectionStatus("disconnected");
        break;
      case ReadyState.UNINSTANTIATED:
      default:
        setConnectionStatus("connecting");
        break;
    }
  }, [readyState, enabled]);

  useEffect(() => {
    if (!lastMessage || !enabled) {
      return;
    }

    if (typeof lastMessage.data !== "string") {
      return;
    }

    try {
      const payload = JSON.parse(lastMessage.data) as {
        type: string;
        data?: DecisionEvent;
        message?: string;
      };

      if (payload.type === "connected") {
        setConnectionStatus("connected");
        return;
      }

      if (payload.type === "decision" && payload.data) {
        setDecisions((current) => {
          const next = [payload.data as DecisionEvent, ...current];
          if (next.length > MAX_EVENTS) {
            next.length = MAX_EVENTS;
          }
          return next;
        });
      }
    } catch (error) {
      console.error("Failed to parse decision stream payload", error);
      setConnectionStatus("error");
    }
  }, [lastMessage, enabled]);

  return {
    decisions,
    connectionStatus,
    isConnected: readyState === ReadyState.OPEN,
    sendMessage,
  };
}
