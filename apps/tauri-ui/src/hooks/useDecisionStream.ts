import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import useWebSocket, { ReadyState } from "react-use-websocket";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

import type { DecisionEvent } from "../types/monitoring";

export type DecisionStreamStatus =
  | "connecting"
  | "connected"
  | "disconnected"
  | "error";

const MAX_EVENTS = 100;

export function useDecisionStream(
  tenantId?: string,
  enabled: boolean = true,
) {
  const didUnmount = useRef(false);
  const [decisions, setDecisions] = useState<DecisionEvent[]>([]);
  const [connectionStatus, setConnectionStatus] =
    useState<DecisionStreamStatus>(enabled ? "connecting" : "disconnected");
  const [baseEndpoint, setBaseEndpoint] = useState<string | null>(null);

  useEffect(() => {
    return () => {
      didUnmount.current = true;
    };
  }, []);

  const resolveStreamEndpoint = useCallback(async () => {
    if (!enabled || didUnmount.current) {
      return;
    }

    setConnectionStatus("connecting");

    try {
      const endpoint = await invoke<string>("get_enforcer_ws_url");
      if (didUnmount.current) {
        return;
      }
      setBaseEndpoint(endpoint);
    } catch (error) {
      if (didUnmount.current) {
        return;
      }
      console.error("Failed to resolve decision stream endpoint", error);
      setBaseEndpoint(null);
      setConnectionStatus("error");
    }
  }, [enabled]);

  useEffect(() => {
    void resolveStreamEndpoint();
  }, [resolveStreamEndpoint]);

  useEffect(() => {
    let unlisten: UnlistenFn | null = null;
    let cancelled = false;

    void listen("service-config-updated", () => {
      void resolveStreamEndpoint();
    })
      .then((fn) => {
        if (cancelled) {
          fn();
          return;
        }
        unlisten = fn;
      })
      .catch((error) => {
        console.warn("Failed to subscribe to service-config-updated events", error);
      });

    return () => {
      cancelled = true;
      if (unlisten) {
        unlisten();
      }
    };
  }, [resolveStreamEndpoint]);

  useEffect(() => {
    if (!enabled) {
      setBaseEndpoint(null);
    }
  }, [enabled]);

  const streamUrl = useMemo(() => {
    if (!enabled || !baseEndpoint) {
      return null;
    }

    try {
      const url = new URL(baseEndpoint);
      if (tenantId && tenantId.length > 0) {
        url.searchParams.set("tenant_id", tenantId);
      } else {
        url.searchParams.delete("tenant_id");
      }
      return url.toString();
    } catch (error) {
      console.error("Invalid decision stream endpoint", error);
      return null;
    }
  }, [baseEndpoint, tenantId, enabled]);

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
