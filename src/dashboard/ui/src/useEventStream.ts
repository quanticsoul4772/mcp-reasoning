import { useEffect, useRef, useState } from "react";
import type { ActivityEvent } from "./types";

const MAX_BUFFER = 300;

export interface EventStream {
  /** Most-recent-first ring buffer of received events. */
  events: ActivityEvent[];
  /** Total events received this session. */
  total: number;
  /** Whether the SSE connection is currently open. */
  connected: boolean;
}

/**
 * Subscribe to the sidecar's `/events` SSE stream.
 *
 * `onEvent` fires for every event (use it to drive node/edge animation);
 * the returned ring buffer feeds the timeline. EventSource auto-reconnects.
 */
export function useEventStream(onEvent: (ev: ActivityEvent) => void): EventStream {
  const [events, setEvents] = useState<ActivityEvent[]>([]);
  const [total, setTotal] = useState(0);
  const [connected, setConnected] = useState(false);
  // Keep the latest callback without re-subscribing the EventSource.
  const cb = useRef(onEvent);
  cb.current = onEvent;

  useEffect(() => {
    const es = new EventSource("/events");
    es.onopen = () => setConnected(true);
    es.onerror = () => setConnected(false);
    es.onmessage = (m) => {
      let ev: ActivityEvent;
      try {
        ev = JSON.parse(m.data) as ActivityEvent;
      } catch {
        return; // ignore keep-alive / non-JSON frames
      }
      cb.current(ev);
      setTotal((t) => t + 1);
      setEvents((prev) => {
        const next = [ev, ...prev];
        return next.length > MAX_BUFFER ? next.slice(0, MAX_BUFFER) : next;
      });
    };
    return () => es.close();
  }, []);

  return { events, total, connected };
}
