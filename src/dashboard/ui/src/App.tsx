import { useCallback, useMemo, useRef } from "react";
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
import { useEventStream } from "./useEventStream";
import type { ActivityEvent } from "./types";

const FLASH_MS = 320;

export default function App() {
  const [nodes, setNodes, onNodesChange] = useNodesState<Node<ActivityNodeData>>(initialNodes);
  const [edges, setEdges, onEdgesChange] = useEdgesState<Edge>(initialEdges);
  const nodeTypes = useMemo(() => ({ activity: ActivityNode }), []);

  const counts = useRef<Record<string, number>>({});
  const nodeTimers = useRef<Record<string, ReturnType<typeof setTimeout>>>({});
  const edgeTimers = useRef<Record<string, ReturnType<typeof setTimeout>>>({});

  const flashNode = useCallback(
    (id: string, phase: string) => {
      counts.current[id] = (counts.current[id] ?? 0) + 1;
      const count = counts.current[id];
      setNodes((ns) =>
        ns.map((node) =>
          node.id === id
            ? { ...node, data: { ...node.data, count, active: true, phase } }
            : node,
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
            return {
              ...edge,
              animated: isAsync,
              style: { ...edge.style, stroke: base, strokeWidth: 2 },
            };
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
        <Timeline events={stream.events} />
      </main>
    </div>
  );
}
