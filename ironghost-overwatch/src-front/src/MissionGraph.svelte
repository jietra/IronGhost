<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import ForceGraph, { type NodeObject } from 'force-graph';

  // 1. TypeScript interfaces for tactical graph
  interface GraphNode {
    id    : string;
    title : string;
    kind  : 'goal' | 'subgoal' | 'task' | 'asset' | 'vuln' | 'risk';
  }

  interface GraphEdge {
    from    : string;
    to      : string;
    relation: string;
  }

  interface MissionGraphData {
    nodes: GraphNode[];
    edges: GraphEdge[];
  }

  // Extend NodeObject type
  interface CustomNode extends NodeObject {
    id  : string;
    name: string;
    kind: keyof typeof KIND_COLORS;
    val : number;
  }

  // 2. Retrieve (Props)
  let { mission_graph }: { mission_graph: MissionGraphData } = $props();

  let graphEl       : HTMLDivElement | undefined  = $state();
  let graphInstance : any                         = null;
  let resizeObserver: ResizeObserver | null       = null;

  const KIND_COLORS = {
    goal    : '#00f0ff',        // Cyan (luminous)
    subgoal : '#3b82f6',        // Blue (neon)
    task    : '#a855f7',        // Purple
    asset   : '#10b981',        // Green (emerald)
    vuln    : '#f59e0b',        // Amber / Orange
    risk    : '#ef4444'         // Red (alert)
  };

  function handleResetView(): void {
    if (graphInstance) {
      graphInstance.zoomToFit(400, 100);
    }
  }

  // Extract data structure
  function formatGraphData() {
    if (!mission_graph || !Array.isArray(mission_graph.nodes)) {
      return { nodes: [], links: [] };
    }
    return {
      nodes: mission_graph.nodes.map(n => ({
        id  : n.id,
        name: n.title,
        kind: n.kind,
        val : n.kind === 'goal' ? 3 : n.kind === 'subgoal' ? 2 : 1
      })),
      links: mission_graph.edges.map(e => ({
        source: e.from,
        target: e.to,
        label : e.relation
      }))
    };
  }

  function initGraph(): void {
    if (!graphEl || !mission_graph) return;

    const width  = graphEl.clientWidth  || 800;
    const height = graphEl.clientHeight || 600;

    if (graphInstance) {
      // @ts-ignore - Access force-graph internal destructor
      graphInstance._destructor?.();
    }

    graphInstance = (ForceGraph as any)()(graphEl)
      .width(width)
      .height(height)
      .backgroundColor('#060911')
      .graphData(formatGraphData())
      .nodeRelSize(1)
      .nodeColor((node: any) => {
        const n = node as CustomNode;
        return KIND_COLORS[n.kind] || '#94a3b8';
      })
      .nodeVal('val')
      .nodeLabel((node: any) => {
        const n = node as CustomNode;
        return `
          <div style="
            background: rgba(13, 19, 33, 0.95); 
            border: 1px solid ${KIND_COLORS[n.kind] || '#00f0ff'}; 
            padding: 6px 10px; 
            border-radius: 6px; 
            font-family: monospace; 
            font-size: 0.75rem;
            color: #fff;
            box-shadow: 0 0 10px rgba(0,0,0,0.5);
          ">
            <strong style="color: ${KIND_COLORS[n.kind] || '#00f0ff'};">[${n.kind.toUpperCase()}]</strong> ${n.name}
          </div>
        `;
      })
      .nodeCanvasObject((node: any, ctx: CanvasRenderingContext2D, globalScale: number) => {
        const n = node as CustomNode;
        if (n.x === undefined || n.y === undefined) return;

        const label    = n.name;
        const fontSize = 11 / globalScale;
        const radius   = n.kind === 'goal' ? 5 : 3.5;

        // Node shape (small point)
        ctx.beginPath();
        ctx.arc(n.x, n.y, radius, 0, 2 * Math.PI, false);
        ctx.fillStyle = KIND_COLORS[n.kind] || '#94a3b8';
        ctx.fill();

        // Glow neon (point edge)
        ctx.shadowColor = KIND_COLORS[n.kind] || '#00f0ff';
        ctx.shadowBlur  = 6;
        ctx.lineWidth   = 1 / globalScale;
        ctx.strokeStyle = '#ffffff';
        ctx.stroke();
        ctx.shadowBlur  = 0; // reset

        // Label under node
        if (globalScale > 0.5) {
          ctx.font         = `${fontSize}px 'JetBrains Mono', monospace`;
          ctx.textAlign    = 'center';
          ctx.textBaseline = 'middle';
          ctx.fillStyle    = '#cbd5e1';
          ctx.fillText(label, n.x, n.y + radius + 8);
        }
      })
      // --- LINKS & FORCES ---
      .linkColor(() => 'rgba(255, 255, 255, 0.12)')
      .linkWidth(2)
      .linkDirectionalArrowLength(3)
      .linkDirectionalArrowRelPos(1)
      .linkDirectionalArrowColor(() => 'rgba(0, 240, 255, 0.6)')
      .linkDirectionalParticles(1)
      .linkDirectionalParticleSpeed(0.004)
      .linkDirectionalParticleWidth(1.5)
      .linkDirectionalParticleColor(() => '#00f0ff')
      .linkLabel((link: any) => `<span style="color: #94a3b8; font-family: monospace; font-size: 0.7rem;">${link.label}</span>`);

    // Space nodes from one another
    graphInstance.d3Force( 'charge' )?.strength( -320 );
    graphInstance.d3Force( 'link'   )?.distance( 90   );

    // Auto-fit init after forces stabilisation
    setTimeout(() => {
      if (graphInstance) graphInstance.zoomToFit(400, 100);
    }, 500);

    // Dynamic canvas resizing
    resizeObserver = new ResizeObserver(entries => {
      for (let entry of entries) {
        const { width, height } = entry.contentRect;
        if (graphInstance && width > 0 && height > 0) {
          graphInstance.width(width).height(height);
        }
      }
    });

    resizeObserver.observe(graphEl);
  }

  onMount(() => {
    initGraph();
  });

  onDestroy(() => {
    if (resizeObserver) resizeObserver.disconnect();
    // @ts-ignore
    if (graphInstance) graphInstance._destructor?.();
  });

  // Graph reactive update
  $effect(() => {
    if (graphInstance && mission_graph && Array.isArray(mission_graph.nodes)) {
      graphInstance.graphData(formatGraphData());

      setTimeout(() => {
        graphInstance?.zoomToFit(400, 100);
      }, 300);
    }
  });
</script>

<style>
  .wrapper {
    width           : 100%;
    height          : 100%;
    min-height      : 0;
    overflow        : hidden;
    position        : relative;
    background      : #060911;
    background-image: radial-gradient(rgba(0, 240, 255, 0.05) 1px, transparent 0);
    background-size : 24px 24px;
  }
  .graph-container {
    width : 100%;
    height: 100%;
  }

  /* Reset view button */
  .reset-btn {
    position        : absolute;
    top             : 12px;
    right           : 12px;
    background      : rgba(13, 19, 33, 0.85);
    backdrop-filter : blur(8px);
    border          : 1px solid rgba(0, 240, 255, 0.3);
    color           : #00f0ff;
    padding         : 6px 12px;
    border-radius   : 6px;
    font-size       : 0.75rem;
    font-family     : monospace;
    font-weight     : 600;
    cursor          : pointer;
    display         : flex;
    align-items     : center;
    gap             : 6px;
    transition      : all 0.2s ease;
    z-index         : 5;
    box-shadow      : 0 0 10px rgba(0, 0, 0, 0.5);
  }

  .reset-btn:hover {
    background  : rgba(0, 240, 255, 0.15);
    border-color: #00f0ff;
    box-shadow  : 0 0 12px rgba(0, 240, 255, 0.3);
    transform   : translateY(-1px);
  }
  
  /* Optional overlay HUD */
  .legend {
    position        : absolute;
    bottom          : 12px;
    right           : 12px;
    background      : rgba(13, 19, 33, 0.85);
    backdrop-filter : blur(8px);
    border          : 1px solid rgba(255, 255, 255, 0.1);
    padding         : 6px 12px;
    border-radius   : 6px;
    font-size       : 0.7rem;
    display         : flex;
    gap             : 12px;
    pointer-events  : none;
    z-index         : 5;
    font-family     : monospace;
  }

  .legend-item {
    display     : flex;
    align-items : center;
    gap         : 6px;
    color       : #94a3b8;
  }

  .dot {
    width         : 6px;
    height        : 6px;
    border-radius : 50%;
  }
</style>

<div class="wrapper">
  <!-- Action button -->
  <button class="reset-btn" onclick={handleResetView} title="Center graph">
    <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
      <path d="M15 3h6v6M9 21H3v-6M21 3l-7 7M3 21l7-7"/>
    </svg>
  </button>

  <div bind:this={graphEl} class="graph-container"></div>

  <div class="legend">
    <div class="legend-item"><span class="dot" style="background: #00f0ff"></span> Goal</div>
    <div class="legend-item"><span class="dot" style="background: #a855f7"></span> Task</div>
    <div class="legend-item"><span class="dot" style="background: #10b981"></span> Asset</div>
    <div class="legend-item"><span class="dot" style="background: #ef4444"></span> Risk/Vuln</div>
  </div>
</div>
