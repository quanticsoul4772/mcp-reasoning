import { useCallback, useMemo, useRef, useState } from "react";
import {
  Background,
  Controls,
  ReactFlow,
  useEdgesState,
  useNodesState,
  type Edge,
  type Node,
} from "@xyflow/react";
import { ActivityNode } from "./ActivityNode";
import { initialEdges, initialNodes, type ActivityNodeData } from "./flowDefs";
import { Timeline } from "./Timeline";
import { Metrics } from "./Metrics";
import { useEventStream } from "./useEventStream";
import { useMetrics } from "./useMetrics";
import type { ActivityEvent, NodeId, Phase } from "./types";

const FLASH_MS = 320;

const NODE_OPTIONS: NodeId[] = [
  "client", "registry", "mode", "anthropic", "sqlite", "voyage", "worker", "si", "heal", "github",
];

export default function App() {
  const [nodes, setNodes, onNodesChange] = useNodesState<Node<ActivityNodeData>>(initialNodes);
  const [edges, setEdges, onEdgesChange] = useEdgesState<Edge>(initialEdges);
  const nodeTypes = useMemo(() => ({ activity: ActivityNode }), []);

  const [nodeFilter, setNodeFilter] = useState<"all" | NodeId>("all");
  const [phaseFilter, setPhaseFilter] = useState<"all" | Phase>("all");
  const [sessionFilter, setSessionFilter] = useState("");

  const counts = useRef<Record<string, number>>({});
  const nodeTimers = useRef<Record<string, ReturnType<typeof setTimeout>>>({});
  const edgeTimers = useRef<Record<string, ReturnType<typeof setTimeout>>>({});

  const flashNode = useCallback(
    (id: string, phase: string) => {
      counts.current[id] = (counts.current[id] ?? 0) + 1;
      const count = counts.current[id];
      setNodes((ns) =>
        ns.map((node) =>
          node.id === id ? { ...node, data: { ...node.data, count, active: true, phase } } : node,
        ),
      );
      clearTimeout(nodeTimers.current[id]);
      nodeTimers.current[id] = setTimeout(() => {
        setNodes((ns) =>
          ns.map((node) =>
            node.id === id ? { ...node, data: { ...node.data, active: false } } : node,
          ),
        );
      }, FLASH_MS);
    },
    [setNodes],
  );

  const pulseEdge = useCallback(
    (id?: string) => {
      if (!id) return;
      setEdges((es) =>
        es.map((edge) =>
          edge.id === id
            ? { ...edge, animated: true, style: { ...edge.style, stroke: "#ffffff", strokeWidth: 3.5 } }
            : edge,
        ),
      );
      clearTimeout(edgeTimers.current[id]);
      edgeTimers.current[id] = setTimeout(() => {
        setEdges((es) =>
          es.map((edge) => {
            if (edge.id !== id) return edge;
            const isAsync = Boolean((edge.data as { async?: boolean } | undefined)?.async);
            const base = ((edge.data as { base?: string } | undefined)?.base) ?? "#3a4a6e";
            return { ...edge, animated: isAsync, style: { ...edge.style, stroke: base, strokeWidth: 2 } };
          }),
        );
      }, FLASH_MS + 40);
    },
    [setEdges],
  );

  const onEvent = useCallback(
    (ev: ActivityEvent) => {
      flashNode(ev.node, ev.phase);
      pulseEdge(ev.edge);
    },
    [flashNode, pulseEdge],
  );

  const stream = useEventStream(onEvent);
  const metrics = useMetrics();

  const filtered = useMemo(
    () =>
      stream.events.filter(
        (e) =>
          (nodeFilter === "all" || e.node === nodeFilter) &&
          (phaseFilter === "all" || e.phase === phaseFilter) &&
          (sessionFilter === "" || (e.session_id ?? "").includes(sessionFilter)),
      ),
    [stream.events, nodeFilter, phaseFilter, sessionFilter],
  );

  return (
    <div className="app">
      <header>
        <h1>mcp-reasoning · live activity</h1>
        <span className="status">
          <span className={`dot${stream.connected ? " live" : ""}`} />
          {stream.connected ? "live" : "connecting…"}
        </span>
        <span className="spacer" />
        <span className="total">{stream.total} events</span>
      </header>
      <main>
        <div className="stage">
          <ReactFlow
            nodes={nodes}
            edges={edges}
            onNodesChange={onNodesChange}
            onEdgesChange={onEdgesChange}
            nodeTypes={nodeTypes}
            fitView
            fitViewOptions={{ padding: 0.15 }}
            proOptions={{ hideAttribution: true }}
            nodesDraggable={false}
            nodesConnectable={false}
            elementsSelectable={false}
          >
            <Background color="#1f2a44" gap={20} />
            <Controls showInteractive={false} />
          </ReactFlow>
        </div>
        <div className="sidebar">
          <Metrics snap={metrics} />
          <div className="filters">
            <select value={nodeFilter} onChange={(e) => setNodeFilter(e.target.value as "all" | NodeId)}>
              <option value="all">all nodes</option>
              {NODE_OPTIONS.map((n) => (
                <option key={n} value={n}>{n}</option>
              ))}
            </select>
            <select value={phaseFilter} onChange={(e) => setPhaseFilter(e.target.value as "all" | Phase)}>
              <option value="all">all phases</option>
              <option value="started">started</option>
              <option value="progress">progress</option>
              <option value="completed">completed</option>
              <option value="failed">failed</option>
              <option value="held_back">held_back</option>
            </select>
            <input
              placeholder="session…"
              value={sessionFilter}
              onChange={(e) => setSessionFilter(e.target.value)}
            />
          </div>
          <Timeline events={filtered} />
        </div>
      </main>
    </div>
  );
}
