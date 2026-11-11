<script lang="ts">
  import { onMount }    from "svelte";
  import Message        from "./Message.svelte";
  import MissionGraph   from "./MissionGraph.svelte";
  import KnowledgeGraph from "./KnowledgeGraph.svelte";

  // Define TypeScript interfaces
  interface MessageItem {
    agent     : string;
    content   : string;
    timestamp : number;
  }

  interface GraphData {
    nodes: any[];
    edges: any[];
  }

  interface ActiveStreamItem {
    msg_id    : string;
    agent     : string;
    content   : string;
    timestamp : number;
  }

  type GraphView = 'mission' | 'knowledge';

  let messages        : MessageItem[]                     = $state([]);
  let container       : HTMLDivElement | undefined        = $state();
  let draft           : string                            = $state("");
  let ws              : WebSocket | undefined             = $state();
  let graphView       : GraphView                         = $state('mission');
  let mission_graph   : GraphData                         = $state({ nodes: [], edges: [] });
  let knowledge_graph : GraphData                         = $state({ nodes: [], edges: [] });
  let activeStreams   : Record<string, ActiveStreamItem>  = $state({});

  function isGraphData(value: unknown): value is GraphData {
    return (
      typeof value === "object" &&
      value !== null &&
      Array.isArray((value as GraphData).nodes) &&
      Array.isArray((value as GraphData).edges)
    );
  }

  onMount(() => {
    try {
      ws           = new WebSocket("ws://127.0.0.1:9000");
      ws.onopen    = () => { ws?.send(JSON.stringify({ type: "get_history" })); };
      ws.onmessage = (event: MessageEvent) => {
        console.log("WS message:", event.data);

        // For auto-scroll (blocked when not at the bottom)
        let atBottom = true;
        if (container) {
          atBottom = container.scrollTop + container.clientHeight >= container.scrollHeight - 20;
        }

        const data = JSON.parse(event.data);

        switch (data.type) {
          case "history":
            messages = data.payload;
            break;

          case "mission_state":
            if (isGraphData(data.payload)) {
              mission_graph = data.payload;// as MissionGraphData;
            } else {
              console.warn(
                "Invalid mission graph payload:",
                data.payload
              );
            }
            break;

          case "knowledge_graph_state":
            if (isGraphData(data.payload)) {
              knowledge_graph = data.payload;// as KnowledgeGraphData;
            } else {
              console.warn(
                "Invalid knowledge graph payload:",
                data.payload
              );
            }
            break;

          case "message":
            messages.push(data.payload);
            break;

          case "StreamStart":
            activeStreams[data.payload.msg_id] = {
              msg_id   : data.payload.msg_id,
              agent    : data.payload.agent,
              content  : "",
              timestamp: Date.now()
            };
            break;

          case "StreamChunk":
            if (activeStreams[data.payload.msg_id]) {
              activeStreams[data.payload.msg_id].content += data.payload.delta;
            }
            break;

          case "StreamEnd":
            delete activeStreams[data.payload.msg_id];
            break;

          default:
            console.warn("Event unknown:", data.type);
            break;
        }

        // Auto-scroll only when user is at bottom
        if (atBottom && container) {
          setTimeout(() => { 
            if (container) container.scrollTop = container.scrollHeight; 
          }, 0);
        }
      };
    } catch (err) {
      console.warn("WebSocket not available, entering simulation mode.");
    }
  });

  function sendMessage() {
    if (!draft.trim()) return;

    const msgObj: MessageItem = {
      agent     : "Operator",
      content   : draft,
      timestamp : Date.now(),
      //severity  : "operator"
    };

    if (ws && ws.readyState === WebSocket.OPEN) {
      ws.send(JSON.stringify( msgObj ));
    } else {
      messages.push(msgObj);
    }

    // clear input text area
    draft = "";

    // Auto-scroll
    setTimeout(() => {
      if (container) {
        container.scrollTop = container.scrollHeight;
      }
    }, 0);
  }
</script>

<style>
  :global(body) {
    margin      : 0;
    padding     : 0;
    background  : linear-gradient(180deg, #888, #090d16);
    font-family : 'JetBrains Mono', -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
    overflow    : hidden;
    height      : 100vh;
  }

  .cyber-app {
    display       : flex;
    flex-direction: column;
    height        : 100vh;
    width         : 100vw;
    background    : radial-gradient(circle at 50% 0%, #111b2d 0%, #060911 100%);
  }

  /* Header Bar */
  .cyber-header {
    height          : 48px;
    background      : rgba(13, 19, 33, 0.85);
    backdrop-filter : blur(12px);
    border-bottom   : 1px solid rgba(0, 240, 255, 0.15);
    display         : flex;
    align-items     : center;
    justify-content : space-between;
    padding         : 0 20px;
    z-index         : 10;
  }

  .brand {
    display       : flex;
    align-items   : center;
    gap           : 10px;
    font-weight   : 700;
    font-size     : 0.95rem;
    letter-spacing: 1.5px;
    color         : #00f0ff;
    text-shadow   : 0 0 10px rgba(0, 240, 255, 0.4);
  }

  .status-badge {
    display       : flex;
    align-items   : center;
    gap           : 8px;
    font-size     : 0.75rem;
    padding       : 4px 10px;
    border-radius : 20px;
    background    : rgba(16, 185, 129, 0.1);
    border        : 1px solid rgba(16, 185, 129, 0.3);
    color         : #34d399;
  }

  .dot {
    width         : 6px;
    height        : 6px;
    background    : #34d399;
    border-radius : 50%;
    box-shadow    : 0 0 8px #34d399;
  }

  /* Main Grid Layout */
  .layout {
    display   : flex;
    flex      : 1;
    min-height: 0;
    position  : relative;
  }

  /* Left Column: Feed & Input */
  .left {
    flex          : 1.2;
    min-width     : 0;
    display       : flex;
    flex-direction: column;
    background    : rgba(10, 15, 26, 0.6);
    border-right  : 1px solid rgba(255, 255, 255, 0.07);
    position      : relative;
    overflow-y    : auto;
  }

  .panel-header {
    padding         : 12px 18px;
    font-size       : 0.75rem;
    font-weight     : 700;
    text-transform  : uppercase;
    letter-spacing  : 1px;
    color           : #64748b;
    border-bottom   : 1px solid rgba(255, 255, 255, 0.05);
    display         : flex;
    justify-content : space-between;
  }

  .scroll-area {
    flex          : 1;
    overflow-y    : auto;
    padding       : 16px;
    display       : flex;
    flex-direction: column;
    gap           : 12px;
  }

  .scroll-area::-webkit-scrollbar {
    width: 6px;
  }

  .scroll-area::-webkit-scrollbar-thumb {
    background: rgba(0, 240, 255, 0.2);
    border-radius: 3px;
  }

  /* Input Zone */
  .input-area {
    padding     : 14px 16px;
    background  : rgba(13, 19, 33, 0.95);
    border-top  : 1px solid rgba(0, 240, 255, 0.15);
    display     : flex;
    gap         : 12px;
    align-items : center;
  }

  .input-area input {
    flex          : 1;
    padding       : 12px 16px;
    border-radius : 8px;
    border        : 1px solid rgba(255, 255, 255, 0.1);
    background    : rgba(20, 27, 45, 0.8);
    color         : #f8fafc;
    font-family   : inherit;
    font-size     : 0.85rem;
    outline       : none;
    transition    : all 0.2s ease;
  }

  .input-area input:focus {
    border-color: #00f0ff;
    box-shadow  : 0 0 12px rgba(0, 240, 255, 0.2);
  }

  .input-area button {
    padding       : 12px 20px;
    background    : linear-gradient(135deg, #00f0ff 0%, #7000ff 100%);
    border        : none;
    border-radius : 8px;
    font-weight   : 700;
    font-size     : 0.8rem;
    letter-spacing: 0.5px;
    color         : #ffffff;
    cursor        : pointer;
    transition    : transform 0.1s ease, box-shadow 0.2s ease;
    box-shadow    : 0 0 15px rgba(112, 0, 255, 0.4);
  }

  .input-area button:hover {
    transform : translateY(-1px);
    box-shadow: 0 0 20px rgba(0, 240, 255, 0.6);
  }

  /* Right Column */
  .right {   
    flex          : 1.8;
    min-width     : 0;
    display       : flex;
    flex-direction: column;
    background    : #060911;
    position      : relative;
    border-left   : 1px solid #111;
  }

  .graph-wrapper {
    flex      : 1;
    min-height: 0;
    overflow  : hidden;
    position  : relative;
  }

  .empty-graph {
    height         : 100%;
    display        : flex;
    align-items    : center;
    justify-content: center;
    color          : #334155;
    font-size      : 0.72rem;
    font-weight    : 700;
    letter-spacing : 1px;
  }

  .graph-header {
    min-height      : 42px;
    display         : flex;
    align-items     : center;
    justify-content : space-between;
    gap             : 12px;
    padding         : 0 14px;
    border-bottom   : 1px solid
                      rgba(255, 255, 255, 0.06);
    background      : rgba(8, 12, 22, 0.92);
  }

  .graph-tabs {
    align-self : stretch;
    display    : flex;
    align-items: stretch;
    gap        : 4px;
  }

  .graph-tabs button {
    position      : relative;
    display       : flex;
    align-items   : center;
    gap           : 7px;
    padding       : 0 11px;
    border        : none;
    color         : #64748b;
    background    : transparent;
    font-family   : inherit;
    font-size     : 0.68rem;
    font-weight   : 700;
    letter-spacing: 0.8px;
    cursor        : pointer;
  }

  .graph-tabs button::after {
    content   : "";
    position  : absolute;
    right     : 8px;
    bottom    : 0;
    left      : 8px;
    height    : 2px;
    background: transparent;
  }

  .graph-tabs button:hover {
    color: #cbd5e1;
  }

  .graph-tabs button.active {
    color: #67e8f9;
  }

  .graph-tabs button.active::after {
    background:
      linear-gradient(
        90deg,
        #00f0ff,
        #a855f7
      );

    box-shadow:
      0 0 8px
      rgba(0, 240, 255, 0.55);
  }

  .tab-count {
    min-width      : 16px;
    padding        : 1px 5px;
    border-radius  : 9px;
    color          : #94a3b8;
    background     : rgba(148, 163, 184, 0.08);
    font-size      : 0.56rem;
    letter-spacing : 0;
    text-align     : center;
  }

  .graph-live-state {
    color         : #475569;
    font-size     : 0.62rem;
    font-weight   : 700;
    letter-spacing: 0.7px;
  }
</style>

<div class="cyber-app">
  <!-- Top Navigation HUD Bar -->
  <header class="cyber-header">
    <div class="brand">
      <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <polygon points="12 2 2 7 12 12 22 7 12 2"></polygon>
        <polyline points="2 17 12 22 22 17"></polyline>
        <polyline points="2 12 12 17 22 12"></polyline>
      </svg>
      IRONGHOST // TACTICAL BOARD
    </div>
    <div class="status-badge">
      <span class="dot"></span>
      <span>
        MISSION
        ({mission_graph.nodes.length} NODES)
        ·
        KNOWLEDGE
        ({knowledge_graph.nodes.length} NODES)
      </span>
    </div>
  </header>

  <!-- Main View -->
  <div class="layout">
    <!-- Left Panel: Communication Stream -->
    <div class="left">
      <div class="panel-header">
        <span>Stream & Telemetry</span>
        <span>LIVE BROADCAST</span>
      </div>

      <div class="scroll-area" bind:this={ container }>
        {#each messages as m}
          <Message msg={m} />
        {/each}

        <!-- Streaming messages -->
        {#each Object.values(activeStreams) as stream (stream.msg_id)}
          <Message
            msg={{
              agent: stream.agent,
              content: stream.content + "▌",
              timestamp: stream.timestamp
            }}
            isStreaming={true}
          />
        {/each}
      </div>

      <div class="input-area">
        <input 
          bind:value={draft} 
          type="text" 
          placeholder="Enter command..." 
          onkeydown={(e) => e.key === 'Enter' && sendMessage()} 
        />
        <button onclick={sendMessage}>SEND</button>
      </div>
    </div>

    <!-- Right Panel: Graph -->
    <div class="right">
      <div class="graph-header">
        <div class="graph-tabs" role="tablist" aria-label="Graph view">
          <button
            type="button"
            role="tab"
            aria-selected={  graphView === 'mission'   }
            class:active={   graphView === 'mission'   }
            onclick={() => { graphView  =  'mission'; }}
          >
            MISSION

            <span class="tab-count">
              {mission_graph.nodes.length}
            </span>

          </button>

          <button
            type="button"
            role="tab"
            aria-selected={  graphView === 'knowledge'   }
            class:active={   graphView === 'knowledge'   }
            onclick={() => { graphView  =  'knowledge'; }}
          >
            KNOWLEDGE

            <span class="tab-count">
              {knowledge_graph.nodes.length}
            </span>
            
          </button>
        </div>

        <span class="graph-live-state">
          REALTIME GRAPH
        </span>
      </div>
      <div class="graph-wrapper">
        {#if graphView === 'mission'}
          {#if mission_graph.nodes.length > 0}
            <MissionGraph mission_graph={mission_graph} />
          {:else}
            <div class="empty-graph">
              MISSION GRAPH NOT YET AVAILABLE
            </div>
          {/if}

        {:else}
          {#if knowledge_graph.nodes.length > 0}
            <KnowledgeGraph knowledge_graph={knowledge_graph}/>
          {:else}
            <div class="empty-graph">
              KNOWLEDGE GRAPH NOT YET AVAILABLE
            </div>
          {/if}
        {/if}
      </div>
    </div>
  </div>
</div>