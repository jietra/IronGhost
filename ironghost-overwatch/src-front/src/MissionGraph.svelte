<script>
  import { onMount, onDestroy } from 'svelte';
  import ForceGraph from 'force-graph';

  export let mission;

  let graphEl;
  let graphInstance  = null;
  let resizeObserver = null;

  const KIND_COLORS = {
    goal    : '#00f0ff',        // Cyan (luminous)
    subgoal : '#3b82f6',        // Blue (neon)
    task    : '#a855f7',        // Purple
    asset   : '#10b981',        // Green (emerald)
    vuln    : '#f59e0b',        // Amber / Orange
    risk    : '#ef4444'         // Red (alert)
  };

  function handleResetView() {
    if (graphInstance) {
      // Recenter and adjust zoom to fit view (100px padding, 400ms lasting time)
      graphInstance.zoomToFit(400, 100);
    }
  }

  function initGraph() {
    if (!graphEl || !mission) return;

    const width  = graphEl.clientWidth  || 800;
    const height = graphEl.clientHeight || 600;

    // Destroy previous instance if any
    if (graphInstance) {
      graphInstance._destructor?.();
    }

    graphInstance = ForceGraph()(graphEl)
      .width(width)
      .height(height)
      .backgroundColor('#060911')
      .graphData({
        nodes: mission.nodes.map(n => ({
          id  : n.id,
          name: n.title,
          kind: n.kind,
          val : n.kind === 'goal' ? 3 : n.kind === 'subgoal' ? 2 : 1
        })),
        links: mission.edges.map(e => ({
          source: e.from,
          target: e.to,
          label : e.relation
        }))
      })
      // --- REGULATE NODE SIZES & SPACING ---
      .nodeRelSize(1)
      .nodeColor(node => KIND_COLORS[node.kind] || '#94a3b8')
      .nodeVal('val')
      .nodeLabel(node => `
        <div style="
          background: rgba(13, 19, 33, 0.95); 
          border: 1px solid ${KIND_COLORS[node.kind] || '#00f0ff'}; 
          padding: 6px 10px; 
          border-radius: 6px; 
          font-family: monospace; 
          font-size: 0.75rem;
          color: #fff;
          box-shadow: 0 0 10px rgba(0,0,0,0.5);
        ">
          <strong style="color: ${KIND_COLORS[node.kind] || '#00f0ff'};">[${node.kind.toUpperCase()}]</strong> ${node.name}
        </div>
      `)
      // Custom Canvas Rendering for tags under each points
      .nodeCanvasObject((node, ctx, globalScale) => {
        const label    = node.name;
        const fontSize = 11 / globalScale;
        const radius   = node.kind === 'goal' ? 5 : 3.5;

        // Node shape (small point)
        ctx.beginPath();
        ctx.arc(node.x, node.y, radius, 0, 2 * Math.PI, false);
        ctx.fillStyle = KIND_COLORS[node.kind] || '#94a3b8';
        ctx.fill();

        // Glow neon (point edge)
        ctx.shadowColor = KIND_COLORS[node.kind] || '#00f0ff';
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
          ctx.fillText(label, node.x, node.y + radius + 8);
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
      .linkLabel(link => `<span style="color: #94a3b8; font-family: monospace; font-size: 0.7rem;">${link.label}</span>`);

    // Space nodes from one another
    graphInstance.d3Force( 'charge' ).strength( -320 );
    graphInstance.d3Force( 'link'   ).distance(   90 );

    // Auto-fit init after forces stabilisation
    setTimeout(() => {
      if (graphInstance) {
        graphInstance.zoomToFit(400, 100);
      }
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
    if (graphInstance) graphInstance._destructor?.();
  });

  // Graph reactive update
  $: if (graphInstance && mission && Array.isArray(mission.nodes)) {
    graphInstance.graphData({
      nodes: mission.nodes.map(n => ({
        id  : n.id,
        name: n.title,
        kind: n.kind,
        val : n.kind === 'goal' ? 3 : n.kind === 'subgoal' ? 2 : 1
      })),
      links: mission.edges.map(e => ({
        source: e.from,
        target: e.to,
        label : e.relation
      }))
    });

    // Re-fit
    setTimeout(() => {
      graphInstance?.zoomToFit(400, 100);
    }, 300);
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
  <button class="reset-btn" on:click={handleResetView} title="Recentrer le graphe">
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
