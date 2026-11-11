<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import ForceGraph, { type NodeObject } from 'force-graph';

  // 1. TypeScript interfaces for knowledge graph
  interface KnowledgeGraphNode {
    id     : string;
    kind   : 'concept' | 'claim' | 'evidence';
    label  : string;
    status?: string | null;
  }

  interface KnowledgeGraphEdge {
    from    : string;
    to      : string;
    relation: | 'claim_of' | 'supports' | 'related_to';
  }

  interface KnowledgeGraphData {
    nodes: KnowledgeGraphNode[];
    edges: KnowledgeGraphEdge[];
  }

  // Extend NodeObject type
  interface CustomNode extends NodeObject {
    id          : string;
    name        : string;
    kind        : KnowledgeGraphNode['kind'];
    val         : number;
    status?     : string | null;
  }

  type KnowledgeDepth = | 'concepts' | 'claims' | 'all';

  // 2. Retrieve (Props)
  let { knowledge_graph }: { knowledge_graph: KnowledgeGraphData } = $props();

  let depth         : KnowledgeDepth              = $state('concepts');
  let selectedNode  : KnowledgeGraphNode | null   = $state(null);

  function visibleKinds(): Set<KnowledgeGraphNode['kind']> {
    switch (depth) {
      case 'concepts':
        return new Set([ 'concept' ]);

      case 'claims':
        return new Set([ 'concept', 'claim' ]);

      case 'all':
        return new Set([ 'concept', 'claim', 'evidence' ]);
    }
  }

  let graphEl       : HTMLDivElement | undefined  = $state();
  let graphInstance : any                         = null;
  let resizeObserver: ResizeObserver | null       = null;

  const KIND_COLORS = {
    concept : '#00f0ff',
    claim   : '#a855f7',
    evidence: '#64748b',
  };

  const STATUS_COLORS:
    Record<string, string> =
  {
    // concepts
    new          : '#60a5fa', // blue
    investigating: '#fbbf24', // amber
    resolved     : '#34d399', // emerald

    // claims
    hypothesis   : '#c084fc', // violet
    supported    : '#22d3ee', // cyan
    confirmed    : '#a3e635', // lime
    weakened     : '#fb923c', // orange
    refuted      : '#f43f5e', // rose/red
    undetermined : '#a78bfa', // indigo/lavender
  };

  function nodeColor(node: CustomNode): string {
    if (node.status) {
      const status = node.status.toLowerCase();
      if (STATUS_COLORS[status]) { return STATUS_COLORS[status]; }
    }

    return (
      KIND_COLORS[node.kind] ??
      '#94a3b8'
    );
  }

  function relationColor(relation: string): string {
    switch (relation) {
      case 'claim_of':
        return 'rgba(168, 85, 247, 0.35)';

      case 'supports':
        return 'rgba(59, 130, 246, 0.30)';

      case 'related_to':
        return 'rgba(0, 240, 255, 0.28)';

      default:
        return 'rgba(255, 255, 255, 0.12)';
    }
  }

  function relationArrowColor(relation: string): string {
    switch (relation) {
      case 'claim_of':
        return '#a855f7';

      case 'supports':
        return '#3b82f6';

      case 'related_to':
        return '#00f0ff';

      default:
        return '#64748b';
    }
  }

  function nodeValue(kind: KnowledgeGraphNode['kind']): number {
    switch (kind) {
      case 'concept':
        return 5;

      case 'claim':
        return 2;

      case 'evidence':
        return 1;
    }
  }

  function handleResetView(): void {
    if (graphInstance) {
      graphInstance.zoomToFit(400, 100);
    }
  }

  // Extract data structure
  function formatGraphData() {
    if (!knowledge_graph || !Array.isArray(knowledge_graph.nodes)) {
      return { nodes: [], links: [] };
    }

    const kinds   = visibleKinds();
    const nodes   = knowledge_graph.nodes.filter(
      node =>
        kinds.has(node.kind)
    );
    const nodeIds = new Set( nodes.map(node => node.id) );
    const edges   = knowledge_graph.edges.filter(
      edge =>
        nodeIds.has(edge.from) &&
        nodeIds.has(edge.to)
    );

    return {
      nodes: nodes.map(n => ({
        id         : n.id,
        name       : n.label,
        kind       : n.kind,
        status     : n.status ?? null,
        val        : nodeValue(n.kind),
      })),
      links: edges.map(e => ({
        source: e.from,
        target: e.to,
        label : e.relation
      }))
    };
  }

  function initGraph(): void {
    if (!graphEl || !knowledge_graph) return;

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
      .nodeColor((node: any) => nodeColor(node as CustomNode))
      .nodeVal('val')
      .onNodeClick((node: any) => {
        const clicked = node as CustomNode;
        selectedNode = knowledge_graph.nodes.find(
            candidate => candidate.id === clicked.id
          ) ?? null;
      })
      .nodeLabel((node: any) => {
        const n = node as CustomNode;
        const safeName   = escapeHtml(n.name);
        const safeStatus = n.status? escapeHtml(n.status): null;

        const status = safeStatus
          ? `<div style="
              margin-top: 5px;
              color: #94a3b8;
            ">status: <strong>${safeStatus}</strong></div>`
          : '';
          
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
            <strong style="color: ${KIND_COLORS[n.kind] || '#00f0ff'};">
              [${n.kind.toUpperCase()}]
            </strong>
            ${safeName}
            ${status}
          </div>
        `;
      })
      .nodeCanvasObject((node: any, ctx: CanvasRenderingContext2D, globalScale: number) => {
        const n = node as CustomNode;
        if (n.x === undefined || n.y === undefined) return;

        const color    = nodeColor(n);
        const showLabel= n.kind === 'concept' || globalScale > 1.5;
        const fontSize = 11 / globalScale;
        const radius   = n.kind === 'concept' ? 5 : 3.5;

        // Node shape (small point)
        ctx.beginPath();
        ctx.arc(n.x, n.y, radius, 0, 2 * Math.PI, false);
        ctx.fillStyle = color;
        ctx.fill();

        // Glow neon (point edge)
        ctx.shadowColor = color;
        ctx.shadowBlur  = 6;
        ctx.lineWidth   = 1 / globalScale;
        ctx.strokeStyle = '#ffffff';
        ctx.stroke();
        ctx.shadowBlur  = 0; // reset

        // Label under node
        if (showLabel) {
          ctx.font         = `${fontSize}px 'JetBrains Mono', monospace`;
          ctx.textAlign    = 'center';
          ctx.textBaseline = 'middle';
          ctx.fillStyle    = '#cbd5e1';
          const maxLabelLength = n.kind === 'concept'? 32: 20;
          const label      = n.name.length > maxLabelLength? `${n.name.slice(0,maxLabelLength)}…`: n.name;
          ctx.fillText(label, n.x, n.y + radius + 8);
        }
      })
      // --- LINKS & FORCES ---
      .linkColor((link: any) =>relationColor(link.label))
      .linkWidth(1.5)
      .linkDirectionalArrowLength(6)
      .linkDirectionalArrowRelPos(0.85)
      .linkDirectionalArrowColor((link: any) =>relationArrowColor(link.label))
      //.linkDirectionalParticles(1)
      //.linkDirectionalParticleSpeed(0.004)
      //.linkDirectionalParticleWidth(1.5)
      //.linkDirectionalParticleColor(() => '#00f0ff')
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
    const currentDepth = depth;           // only for explicit mention
    const currentGraph = knowledge_graph;

    if (graphInstance && currentGraph && Array.isArray(currentGraph.nodes)) {
      graphInstance.graphData(formatGraphData());

      setTimeout(() => {
        graphInstance?.zoomToFit(400, 100);
      }, 300);
    }
  });

  function escapeHtml(value: string): string {
    return value
      .replaceAll('&', '&amp;')
      .replaceAll('<', '&lt;')
      .replaceAll('>', '&gt;')
      .replaceAll('"', '&quot;')
      .replaceAll("'", '&#039;');
  }
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

  /* Graph toolbar */
  .graph-toolbar {
    position   : absolute;
    top        : 12px;
    right      : 12px;
    z-index    : 5;

    display    : flex;
    align-items: center;
    gap        : 8px;
  }

  /* Reset view button */
  .reset-btn {
    /*position        : absolute;
    top             : 12px;
    right           : 12px;*/
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

  /* Depth controls */
  .depth-controls {
    top             : 12px;
    right           : 12px;
    /*background      : rgba(13, 19, 33, 0.85);*/
    backdrop-filter : blur(8px);
    /*border          : 1px solid rgba(0, 240, 255, 0.3);*/
    color           : #00f0ff;
    padding         : 6px 12px;
    /*border-radius   : 6px;*/
    font-size       : 0.7rem;
    font-family     : monospace;
    font-weight     : 500;
    cursor          : pointer;
    display         : flex;
    align-items     : center;
    gap             : 6px;
    transition      : all 0.2s ease;
    z-index         : 5;
    /*box-shadow      : 0 0 10px rgba(0, 0, 0, 0.5);*/
  }

  /* Depth control button */
  .depth-control {
    /*position        : absolute;*/
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

  .depth-control:hover {
    background  : rgba(0, 240, 255, 0.15);
    border-color: #00f0ff;
    box-shadow  : 0 0 12px rgba(0, 240, 255, 0.3);
    transform   : translateY(-1px);
  }

  .depth-control.active {
    color       : #ffffff;
    background  : rgba(0, 240, 255, 0.15);
    border-color: #00f0ff;
    box-shadow  :
      0 0 12px rgba(0, 240, 255, 0.25);
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

  /* Node inspector */
  .node-inspector {
    position       : absolute;
    top            : 58px;
    right          : 12px;
    width          : min(320px, calc(100% - 24px));
    padding        : 14px;
    z-index        : 6;

    background     : rgba(8, 13, 24, 0.94);
    backdrop-filter: blur(12px);

    border         : 1px solid rgba(0, 240, 255, 0.18);
    border-left    : 3px solid #00f0ff;
    border-radius  : 7px;

    box-shadow     : 0 12px 32px rgba(0, 0, 0, 0.4);
  }

  .inspector-close {
    position  : absolute;
    top       : 6px;
    right     : 8px;

    border    : 0;
    background: transparent;
    color     : #64748b;
    cursor    : pointer;

    font-size : 1rem;
  }

  .inspector-kind {
    margin-bottom  : 5px;
    color          : #00f0ff;
    font-size      : 0.58rem;
    font-weight    : 700;
    letter-spacing : 0.8px;
    text-transform : uppercase;
  }

  .inspector-title {
    padding-right : 16px;
    color         : #e2e8f0;
    font-size     : 0.74rem;
    line-height   : 1.55;
    overflow-wrap : anywhere;
  }

  .inspector-status {
    display       : inline-block;
    margin-top    : 10px;
    padding       : 2px 7px;
    color         : #94a3b8;
    background    : rgba(148, 163, 184, 0.08);
    border-radius : 10px;
    font-size     : 0.56rem;
    text-transform: uppercase;
  }

  .inspector-id {
    margin-top   : 10px;
    padding-top  : 8px;
    border-top   : 1px solid rgba(148, 163, 184, 0.1);
    color        : #475569;
    font-family  : monospace;
    font-size    : 0.57rem;
    overflow-wrap: anywhere;
  }
</style>
<div class="wrapper">
  <!-- Action button -->
  <div class="graph-toolbar">
    <div class="depth-controls">
      <button class="depth-control" class:active={depth === 'concepts'} onclick={() => { depth = 'concepts'; }}>
        [ OVERVIEW ]
      </button>

      <button class="depth-control" class:active={depth === 'claims'} onclick={() => { depth = 'claims'; }}>
        [ CLAIMS ]
      </button>

      <button class="depth-control" class:active={depth === 'all'} onclick={() => { depth = 'all'; }}>
        [ FULL ]
      </button>
    </div>

    <button class="reset-btn" onclick={handleResetView} title="Center graph">
      <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <path d="M15 3h6v6M9 21H3v-6M21 3l-7 7M3 21l7-7"/>
      </svg>
    </button>
  </div>

  <div bind:this={graphEl} class="graph-container"></div>

  <div class="legend">
    <div class="legend-item"><span class="dot" style="background: #60a5fa"></span> Concept:new</div>
    <div class="legend-item"><span class="dot" style="background: #fbbf24"></span> Concept:investigating</div>
    <div class="legend-item"><span class="dot" style="background: #34d399"></span> Concept:resolved</div>

    <div class="legend-item"><span class="dot" style="background: #c084fc"></span> Claim:hypothesis</div>
    <div class="legend-item"><span class="dot" style="background: #22d3ee"></span> Claim:supported</div>
    <div class="legend-item"><span class="dot" style="background: #a3e635"></span> Claim:confirmed</div>
    <div class="legend-item"><span class="dot" style="background: #fb923c"></span> Claim:weakened</div>
    <div class="legend-item"><span class="dot" style="background: #f43f5e"></span> Claim:refuted</div>
    <div class="legend-item"><span class="dot" style="background: #a78bfa"></span> Claim:undetermined</div>

    <div class="legend-item"><span class="dot" style="background: #64748b"></span> Evidence</div>
  </div>
</div>
{#if selectedNode}
  <aside class="node-inspector">
    <button
      type="button"
      class="inspector-close"
      aria-label="Close node details"
      onclick={() => {
        selectedNode = null;
      }}
    >
      ×
    </button>

    <div class="inspector-kind">
      {selectedNode.kind}
    </div>

    <div class="inspector-title">
      {selectedNode.label}
    </div>

    {#if selectedNode.status}
      <span
        class="inspector-status"
        data-status={
          selectedNode.status.toLowerCase()
        }
      >
        {selectedNode.status}
      </span>
    {/if}

    <div class="inspector-id">
      {selectedNode.id}
    </div>
  </aside>
{/if}