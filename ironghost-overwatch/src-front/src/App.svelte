<!--<script lang="ts">-->
<script>
  import { onMount }  from "svelte";
  import   Message    from "./Message.svelte";
  import MissionGraph from "./MissionGraph.svelte";

  let messages    = [];
  let container;
  let draft       = "";
  let ws;
  let mission = { nodes: [], edges: [] };
  let activeStreams = {};

  onMount(() => {
    try {
      ws           = new WebSocket("ws://127.0.0.1:9000");
      ws.onopen    = ()      => { ws.send(JSON.stringify({ type: "get_history" })); };
      ws.onmessage = (event) => {
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
            mission = data.payload;
            break;

          case "message":
            messages = [...messages, data.payload];
            break;

          case "StreamStart":
            activeStreams[data.payload.msg_id] = {
              msg_id   : data.payload.msg_id,
              agent    : data.payload.agent,
              content  : "",
              timestamp: Date.now()
            };
            activeStreams = activeStreams;
            break;

          case "StreamChunk":
            if (activeStreams[data.payload.msg_id]) {
              activeStreams[data.payload.msg_id].content += data.payload.delta;
              activeStreams = activeStreams;
            }
            break;

          case "StreamEnd":
            delete activeStreams[data.payload.msg_id];
            activeStreams = activeStreams;
            break;

          default:
            console.warn("Event unknown:", data.type);
            break;
        }

        // Auto-scroll only when user is at bottom
        if (atBottom && container) {
          setTimeout( () => { container.scrollTop = container.scrollHeight; }, 0 );
        }
      };
    } catch (err) {
      console.warn("WebSocket not available, entering simulation mode.");
    }
  });

  function sendMessage() {
    if (!draft.trim()) return;

    const msgObj = {
      agent     : "Operator",
      content   : draft,
      timestamp : Date.now(),
      severity  : "operator"
    };

    if (ws && ws.readyState === WebSocket.OPEN) {
      ws.send(JSON.stringify( msgObj ));
    } else {
      messages = [...messages, msgObj];
    }

    // clear input text area
    draft = "";

    // Auto-scroll
    setTimeout(() => {
      if (container) {
        container.scrollTop = container.scrollHeight;
      }
    },0);
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
    flex          : 1.2; /*2 1 0;*/
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

  /* Input Zone inside Left Column */
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

  /* Right Column: Mission Graph Visualizer */
  .right {   
    flex          : 1.8; /*1 1 0;*/
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
      AGENTS ACTIVE ({mission.nodes.length} NODES)
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
        { #each messages as m }
          <Message msg={ m } />
        { /each }

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
          type        ="text"
          bind:value  ={ draft }
          placeholder ="Message"
          name        ="Operator_Instruction"
          on:keydown  ={ (e) => e.key === 'Enter' && sendMessage() }
        />
        <button on:click={ sendMessage }>SEND</button>
      </div>      
    </div>

    <!-- Right Panel: Graph -->
    <div class="right">
      <div class="panel-header">
        <span>MISSION TOPOLOGY</span>
        <span>REALTIME GRAPH</span>
      </div>
      <div class="graph-wrapper">
        { #if typeof mission === "object" }
          <MissionGraph { mission } />
        { /if }
      </div>
    </div>
  </div>
</div>