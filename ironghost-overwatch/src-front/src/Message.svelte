<script>
  import { marked } from "marked";

  export let msg;
  export let isStreaming = false;

  let parsed = { think: null, mission: null, final: "" };

  function splitMessage(raw = "") {
    if (typeof raw !== "string") raw = "";

    // extract <think> block
    const thinkMatch = raw.match( /<think>([\s\S]*?)(?:<\/think>|$)/i );
    const think      = thinkMatch ? thinkMatch[1].trim() : null;

    // extract <mission_state> block
    const missionMatch = raw.match( /<mission_state>([\s\S]*?)(?:<\/mission_state>|$)/i );
    let mission = null;
    if (missionMatch && missionMatch[1].trim()) {
      try {
        mission = JSON.stringify(JSON.parse(missionMatch[1].trim()), null, 2);
      } catch (e) {
        mission = missionMatch[1].trim(); // raw fallback during streaming
      }
    }

    // cleaning for markdown parser
    let final = raw
      .replace(/<think>[\s\S]*?(?:<\/think>|$)/gi, "")
      .replace(/<mission_state>[\s\S]*?(?:<\/mission_state>|$)/gi, "")
      .trim();

    return { think, mission, final };
  }

  $: parsed = splitMessage(msg.content || "");

  function formatTime( ts ) {
    if (!ts) return "";
    return new Date( ts ).toLocaleTimeString( [], { hour: '2-digit', minute: '2-digit', second: '2-digit' } );
  }

  function roleColor( agent = "" ) {
    switch ( agent.toLowerCase() ) {
      case "strategist"     : return "#a855f7"; // Purple (neon)
      case "coder"          : return "#4fc3f7";
      case "generic"        : return "#81c784";
      case "operator"       : return "#00f0ff"; // Cyan (luminous)
      case "reconagent"     : return "#3b82f6"; // Blue (neon)
      case "vulnscanner"    : return "#f59e0b"; // Amber / Alert
      case "exploitagent"   : return "#ef4444"; // Red (alert)
      case "soc-analyst"    : return "#10b981"; // Green (emerald)
      default               : return "#94a3b8"; // Slate grey
    }
  }
</script>

<style>
  .msg-container {
    display         : flex;
    flex-direction  : column;
    width           : 100%;
    margin          : 4px 0;
  }
  .msg {
    max-width       : 82%;
    padding         : 10px 14px;
    border-radius   : 8px;
    font-size       : 0.85rem;
    line-height     : 1.5;
    position        : relative;
    animation       : fadein 0.2s ease-out;
    box-shadow      : 0 4px 12px rgba(0, 0, 0, 0.3);
    word-break      : break-word;
  }
  .msg.operator {
    align-self                  : flex-end;
    background                  : linear-gradient(135deg, rgba(112, 0, 255, 0.25) 0%, rgba(0, 240, 255, 0.15) 100%);
    border                      : 1px solid rgba(0, 240, 255, 0.3);
    border-bottom-right-radius  : 2px;
    color                       : #e2e8f0;
  }
  .msg.other {
    align-self                  : flex-start;
    background                  : rgba(13, 19, 33, 0.85);
    border                      : 1px solid rgba(255, 255, 255, 0.08);
    border-bottom-left-radius   : 2px;
    color                       : #cbd5e1;
  }
  @keyframes fadein {
    from { opacity: 0; transform: translateY( 6px ); }
    to   { opacity: 1; transform: translateY( 0   ); }
  }

  .header {
    display         : flex;
    justify-content : space-between;
    align-items     : center;
    margin-bottom   : 6px;
    padding-bottom  : 4px;
    border-bottom   : 1px solid rgba(255, 255, 255, 0.05);
    gap             : 12px;
  }
  .agent {
    font-size       : 0.75rem;
    font-weight     : 700;
    letter-spacing  : 0.5px;
    text-transform  : uppercase;
    display         : flex;
    align-items     : center;
    gap             : 6px;
  }
  .agent-icon {
    width           : 6px;
    height          : 6px;
    border-radius   : 50%;
    display         : inline-block;
  }
  .timestamp {
    font-size   : 0.65rem;
    color       : #64748b;
    font-family : monospace;
  }

  .content {
    font-family : 'JetBrains Mono', monospace, system-ui;
  }

  /* Thinking block <think> */
  .think-box {
    margin-bottom   : 8px;
    background      : rgba(0, 0, 0, 0.3);
    border          : 1px dashed rgba(168, 85, 247, 0.4);
    border-radius   : 6px;
    overflow        : hidden;
  }

  .think-summary {
    padding         : 6px 10px;
    font-size       : 0.7rem;
    color           : #c084fc;
    cursor          : pointer;
    user-select     : none;
    font-weight     : 600;
    letter-spacing  : 0.5px;
    display         : flex;
    align-items     : center;
    gap             : 6px;
    background      : rgba(168, 85, 247, 0.08);
    transition      : background 0.2s ease;
  }

  .think-summary:hover {
    background  : rgba(168, 85, 247, 0.15);
  }

  .think-summary span {
    color           : #c084fc;
    text-shadow     : 0 0 6px rgba(192, 132, 252, 0.6);
    animation       : pulse 2s infinite alternate;
  }

  @keyframes pulse {
    0%   { opacity: 0.6; }
    100% { opacity: 1; }
  }

  .think-body {
    padding     : 8px 10px;
    font-size   : 0.75rem;
    color       : #94a3b8;
    border-top  : 1px dashed rgba(168, 85, 247, 0.2);
    background  : rgba(0, 0, 0, 0.2);
  }

  /* Mission State block <mission_state> */
  .mission-box {
    margin-top      : 8px;
    background      : rgba(0, 240, 255, 0.03);
    border          : 1px solid rgba(0, 240, 255, 0.2);
    border-radius   : 6px;
    overflow        : hidden;
  }

  .mission-summary {
    padding         : 6px 10px;
    font-size       : 0.7rem;
    color           : #00f0ff;
    cursor          : pointer;
    user-select     : none;
    font-weight     : 600;
    letter-spacing  : 0.5px;
    display         : flex;
    align-items     : center;
    gap             : 6px;
    background      : rgba(0, 240, 255, 0.08);
    transition      : background 0.2s ease;
  }

  .mission-summary:hover {
    background  : rgba(0, 240, 255, 0.15);
  }

  .mission-body pre {
    margin      : 0;
    padding     : 8px 10px;
    font-size   : 0.7rem;
    color       : #38bdf8;
    background  : rgba(0, 0, 0, 0.4);
    border      : none;
  }

  .generation-status {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    margin-top: 6px;
    padding: 2px 8px;
    background: rgba(0, 240, 255, 0.05);
    border: 1px solid rgba(0, 240, 255, 0.2);
    border-radius: 4px;
    font-size: 0.68rem;
    font-weight: 700;
    letter-spacing: 1px;
    color: #00f0ff;
    text-shadow: 0 0 6px rgba(0, 240, 255, 0.5);
    width: fit-content;
  }

  .status-text {
    text-transform: uppercase;
  }

  .dots span {
    display: inline-block;
    font-size: 0.85rem;
    font-weight: bold;
    animation: wave 1.2s infinite ease-in-out;
  }

  .dots span:nth-child(1) { animation-delay: 0s; }
  .dots span:nth-child(2) { animation-delay: 0.2s; }
  .dots span:nth-child(3) { animation-delay: 0.4s; }

  @keyframes wave {
    0%, 100% {
      opacity: 0.2;
      transform: translateY(0);
    }
    50% {
      opacity: 1;
      transform: translateY(-2px);
      color: #ffffff;
      text-shadow: 0 0 8px #00f0ff;
    }
  }
  /*
  :global(.blinking-cursor) {
    display: inline-block;
    color: #00f0ff;
    font-weight: bold;
    margin-left: 2px;
    animation: blink 0.8s steps(2, start) infinite;
    text-shadow: 0 0 8px rgba(0, 240, 255, 0.8);
  }
  @keyframes blink {
    to {
      visibility: hidden;
    }
  }
  */

  /* Global style for Markdown content */
  :global(.content p) {
    margin          : 0 0 6px 0;
  }
  :global(.content p:last-child) {
    margin-bottom   : 0;
  }
  :global(.content code) {
    background      : rgba(0, 0, 0, 0.4);
    color           : #00f0ff;
    padding         : 2px 5px;
    border-radius   : 4px;
    font-size       : 0.75rem;
    border          : 1px solid rgba(0, 240, 255, 0.2);
  }
  :global(.content pre) {
    background      : rgba(0, 0, 0, 0.5);
    padding         : 8px;
    border-radius   : 6px;
    overflow-x      : auto;
    border          : 1px solid rgba(255, 255, 255, 0.05);
  }
  :global(.content ul, .content ol) {
    margin      : 4px 0;
    padding-left: 18px;
  }
</style>

<div class="msg-container">
  <div class="msg {msg.agent === 'Operator' ? 'operator' : 'other'}">
    <div class="header">
      <div class="agent" style="color: {roleColor(msg.agent)}">
        <span class="agent-icon" style="background: {roleColor(msg.agent)}; box-shadow: 0 0 6px {roleColor(msg.agent)}"></span>
        {msg.agent}
      </div>
      <div class="timestamp">
        {formatTime(msg.timestamp)}
      </div>
    </div>

    <div class="content">
      {#if parsed.think}
        <details class="think-box" open={isStreaming}>
          <summary class="think-summary">
            <span>⚡</span> // REFLEXIONS
          </summary>
          <div class="think-body">
            {@html marked(parsed.think)}
          </div>
        </details>
      {/if}

      {@html marked(parsed.final)}

      {#if parsed.mission}
        <details class="mission-box" open={isStreaming}>
          <summary class="mission-summary">
            🗺️ // MISSION GRAPH PAYLOAD (UPDATED)
          </summary>
          <div class="mission-body">
            <pre><code>{parsed.mission}</code></pre>
          </div>
        </details>
      {/if}

      {#if isStreaming}
        <div class="generation-status">
          <span class="status-text">GENERATING</span>
          <span class="dots">
            <span>.</span><span>.</span><span>.</span>
          </span>
        </div>
      {/if}
    </div>
  </div>
</div>