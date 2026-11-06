<script lang="ts">
  import { marked } from "marked";
  import DOMPurify from "dompurify";

  // 1. Define TypeScript interfaces
  interface MessageItem {
    agent     : string;
    content   : string;
    timestamp : number;
    //severity  : string;
  }

  interface GraphNode {
    id          : string;
    kind        : 'goal' | 'subgoal' | 'task' | 'asset' | 'vuln' | 'risk';
    title       : string;
    description?: string | null;
    worker?     : string | null;
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

  interface NextMoveData {
    intention: string;
    action   : string;
    target   : string | null;
  }

  interface StrategistOutput {
    assessment_report: string;
    next_move        : NextMoveData;
    mission_graph    : MissionGraphData | null;
  }

  interface SentinelVerdict {
    compliant           : boolean;
    score               : number;
    missing_requirements: string[];
  }

  interface GeneratedResponse {
    environment: string;
    runtime    : string;
    source     : string;
  }

  interface ProbeOutput {
    trial             : string;
    max_retries       : string;
    task              : string;
    sentinel_verdict  : SentinelVerdict;
    generated_response: GeneratedResponse;
  }

  interface ParsedMessage {
    think     : string | null;
    final     : string;
    json      : unknown | null;
    strategist: StrategistOutput | null;
    probe     : ProbeOutput | null;

    /*
    * legacy graph display
    */
    graph_legacy: unknown | null;
  }

  // 2. Retrieve (Props)
  let {
    msg, 
    isStreaming = false 
  }: { 
    msg         : MessageItem; 
    isStreaming?: boolean 
  } = $props();

  // (Accordions)
  let showThink   = $state(true);
  let showMission = $state(true);

  function isRecord(value: unknown): value is Record<string, unknown> {
    return (
      typeof value === "object" &&
      value !== null &&
      !Array.isArray(value)
    );
  }

  function isNextMove(value: unknown): value is NextMoveData {
    if (!isRecord(value)) return false;

    return (
      typeof value.intention === "string" &&
      typeof value.action === "string" &&
      (
        value.target === null ||
        typeof value.target === "string"
      )
    );
  }

  function isMissionGraph(
    value: unknown
  ): value is MissionGraphData {
    if (!isRecord(value)) return false;

    return (
      Array.isArray(value.nodes) &&
      Array.isArray(value.edges)
    );
  }

  function isStrategistOutput(
    value: unknown
  ): value is StrategistOutput {
    if (!isRecord(value)) return false;

    return (
      typeof value.assessment_report === "string" &&
      isNextMove(value.next_move) &&
      (
        value.mission_graph === null ||
        isMissionGraph(value.mission_graph)
      )
    );
  }

  function isProbeOutput(value: unknown): value is ProbeOutput {
    if (!isRecord(value)) return false;

    const verdict = value.sentinel_verdict;
    const response = value.generated_response;

    if (!isRecord(verdict) || !isRecord(response)) {
      return false;
    }

    return (
      typeof value.trial === "string" &&
      typeof value.max_retries === "string" &&
      typeof value.task === "string" &&

      typeof verdict.compliant === "boolean" &&
      typeof verdict.score === "number" &&
      Array.isArray(verdict.missing_requirements) &&

      typeof response.environment === "string" &&
      typeof response.runtime === "string" &&
      typeof response.source === "string"
    );
  }

  function tryParseJson(raw: string): unknown | null {
    const trimmed = raw.trim();

    if (
      !trimmed.startsWith("{") &&
      !trimmed.startsWith("[")
    ) {
      return null;
    }

    try {
      return JSON.parse(trimmed);
    } catch {
      return null;
    }
  }

  function splitMessage(raw: string = ""): ParsedMessage {
    if (typeof raw !== "string") {
      raw = "";
    }

    const thinkMatch = raw.match(
      /<think>([\s\S]*?)(?:<\/think>|$)/i
    );

    const think =
      thinkMatch
        ? thinkMatch[1].trim()
        : null;

    let final = raw
      .replace(
        /<think>[\s\S]*?(?:<\/think>|$)/gi,
        ""
      )
      .trim();

    /*
    * Temporary backwards compatibility.
    * Remove this after all backends have migrated.
    */

    const graphMatch = raw.match(
      /<mission_state>([\s\S]*?)(?:<\/mission_state>|$)/i
    );

    const graph_legacy_str =
      graphMatch
        ? graphMatch[1].trim()
        : null;
    
    const graph_legacy = graph_legacy_str? tryParseJson(graph_legacy_str) : null;

    final = final
      .replace(
        /<mission_state>[\s\S]*?(?:<\/mission_state>|$)/gi,
        ""
      )
      .trim();

    const json = tryParseJson(final);

    const strategist =
      isStrategistOutput(json)
        ? json
        : null;

    const probe =
      isProbeOutput(json)
        ? json
        : null;
      
    return {
      think,
      final,
      json,
      strategist,
      probe,

      /*
      * legacy graph display
      */
      graph_legacy
    };
  }

  let parsed = $derived(splitMessage(msg.content || ""));

  function formatAction(action: string): string {
    return action
      .replace(/([a-z])([A-Z])/g, "$1 $2")
      .replace(/_/g, " ")
      .toUpperCase();
  }

  function prettyJson(value: unknown): string {
    try {
      return JSON.stringify(value, null, 2);
    } catch {
      return String(value);
    }
  }

  function graphSummary(
    graph: MissionGraphData
  ): string {
    const nodeCount = graph.nodes.length;
    const edgeCount = graph.edges.length;

    return `${nodeCount} nodes • ${edgeCount} edges`;
  }

  // sanitize markdown
  function renderMarkdown(markdown: string): string {
    return DOMPurify.sanitize(
      marked.parse(markdown) as string
    );
  }

  function formatTime(ts: number): string {
    if (!ts) return "";
    return new Date( ts ).toLocaleTimeString( [], { hour: '2-digit', minute: '2-digit', second: '2-digit' } );
  }

  function roleColor(agent: string = ""): string {
    switch ( agent.toLowerCase() ) {
      case "strategist"     : return "#a855f7"; // Purple (neon)
      case "coder"          : return "#4fc3f7";
      case "generic"        : return "#81c784";
      case "operator"       : return "#00f0ff"; // Cyan (luminous)
      case "reconagent"     : return "#3b82f6"; // Navy blue
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

  .strategist-output {
    display       : flex;
    flex-direction: column;
    gap           : 10px;
  }

  .artifact-header {
    display       : flex;
    align-items   : center;
    gap           : 7px;
    padding       : 6px 9px;
    color         : #c084fc;
    background    : rgba(168, 85, 247, 0.08);
    border        : 1px solid rgba(168, 85, 247, 0.25);
    border-radius : 6px;
    font-size     : 0.7rem;
    font-weight   : 700;
    letter-spacing: 0.8px;
  }

  .artifact-icon {
    color      : #d8b4fe;
    text-shadow: 0 0 8px rgba(192, 132, 252, 0.7);
  }

  .assessment-body {
    padding      : 1px 2px;
    overflow-wrap: anywhere;
  }

  .next-move-card {
    display       : flex;
    flex-direction: column;
    gap           : 7px;
    padding       : 9px 10px;
    background    : rgba(0, 240, 255, 0.04);
    border        : 1px solid rgba(0, 240, 255, 0.18);
    border-left   : 3px solid #00f0ff;
    border-radius : 6px;
  }

  .next-move-header {
    display        : flex;
    align-items    : center;
    justify-content: space-between;
    gap            : 8px;
    color          : #67e8f9;
    font-size      : 0.69rem;
    font-weight    : 700;
    letter-spacing : 0.7px;
  }

  .action-badge {
    padding       : 2px 7px;
    border-radius : 20px;
    background    : rgba(59, 130, 246, 0.12);
    border        : 1px solid rgba(59, 130, 246, 0.3);
    color         : #93c5fd;
    font-size     : 0.62rem;
    white-space   : nowrap;
  }

  .action-badge.finish-action {
    background  : rgba(16, 185, 129, 0.12);
    border-color: rgba(16, 185, 129, 0.3);
    color       : #6ee7b7;
  }

  .next-move-intention {
    color      : #cbd5e1;
    font-size  : 0.76rem;
    line-height: 1.5;
  }

  .next-move-target {
    display    : flex;
    align-items: center;
    gap        : 7px;
    color      : #94a3b8;
    font-size  : 0.69rem;
    min-width  : 0;
  }

  .field-label {
    color         : #64748b;
    font-size     : 0.62rem;
    font-weight   : 700;
    letter-spacing: 0.7px;
  }

  .next-move-target code {
    overflow     : hidden;
    text-overflow: ellipsis;
    white-space  : nowrap;
  }

  .graph-summary {
    margin-left : auto;
    color       : #64748b;
    font-size   : 0.62rem;
    font-weight : 500;
    white-space : nowrap;
  }

  .json-box {
    margin-top   : 8px;
    border       : 1px solid rgba(129, 140, 248, 0.2);
    border-radius: 6px;
    overflow     : hidden;
    background   : rgba(15, 23, 42, 0.45);
  }

  .json-summary {
    display       : flex;
    align-items   : center;
    gap           : 7px;
    padding       : 6px 10px;
    cursor        : pointer;
    color         : #a5b4fc;
    background    : rgba(129, 140, 248, 0.08);
    font-size     : 0.7rem;
    font-weight   : 700;
    letter-spacing: 0.6px;
  }

  .json-body {
    margin        : 0;
    padding       : 9px 10px;
    max-height    : 320px;
    overflow      : auto;
    color         : #bfdbfe;
    background    : rgba(0, 0, 0, 0.35);
    font-size     : 0.68rem;
    line-height   : 1.45;
    white-space   : pre-wrap;
    overflow-wrap : anywhere;
  }

  .probe-output {
    display       : flex;
    flex-direction: column;
    gap           : 10px;
  }

  .probe-header {
    color       : #67e8f9;
    border-color: rgba(0, 240, 255, 0.25);
    background  : rgba(0, 240, 255, 0.06);
  }

  .probe-trial {
    margin-left : auto;
    color       : #64748b;
    font-size   : 0.62rem;
    font-weight : 600;
    white-space : nowrap;
  }

  .probe-task {
    display       : flex;
    flex-direction: column;
    gap           : 4px;
    padding       : 9px 10px;
    background    : rgba(255, 255, 255, 0.025);
    border        : 1px solid rgba(255, 255, 255, 0.06);
    border-radius : 6px;
    color         : #cbd5e1;
    font-size     : 0.75rem;
  }

  /* Sentinel verdict */

  .verdict-card {
    display        : flex;
    align-items    : center;
    justify-content: space-between;
    gap            : 12px;
    padding        : 9px 11px;
    border-radius  : 6px;
  }

  .verdict-ok {
    background : rgba(16, 185, 129, 0.07);
    border     : 1px solid rgba(16, 185, 129, 0.25);
    border-left: 3px solid #10b981;
  }

  .verdict-fail {
    background : rgba(239, 68, 68, 0.07);
    border     : 1px solid rgba(239, 68, 68, 0.25);
    border-left: 3px solid #ef4444;
  }

  .verdict-status {
    display       : flex;
    align-items   : center;
    gap           : 7px;
    color         : #6ee7b7;
    font-size     : 0.7rem;
    font-weight   : 700;
    letter-spacing: 0.7px;
  }

  .verdict-fail .verdict-status {
    color: #f87171;
  }

  .verdict-score {
    display    : flex;
    align-items: center;
    gap        : 7px;
  }

  .verdict-score strong {
    color      : #e2e8f0;
    font-family: monospace;
    font-size  : 0.75rem;
  }

  /* Requirements */

  .missing-requirements {
    padding      : 8px 10px;
    color        : #fbbf24;
    background   : rgba(245, 158, 11, 0.05);
    border       : 1px solid rgba(245, 158, 11, 0.16);
    border-radius: 6px;
  }

  .missing-requirements ul {
    margin     : 5px 0 0 0;
    padding-left: 18px;
  }

  /* Generated response */

  .generated-response {
    overflow     : hidden;
    border       : 1px solid rgba(59, 130, 246, 0.18);
    border-radius: 6px;
    background   : rgba(2, 6, 23, 0.55);
  }

  .generated-header {
    display        : flex;
    align-items    : center;
    justify-content: space-between;
    gap            : 8px;
    padding        : 6px 10px;
    color          : #93c5fd;
    background     : rgba(59, 130, 246, 0.07);
    border-bottom  : 1px solid rgba(59, 130, 246, 0.12);
    font-size      : 0.68rem;
    font-weight    : 700;
    letter-spacing : 0.7px;
  }

  .runtime-badges {
    display: flex;
    gap    : 5px;
  }

  .runtime-badge {
    padding      : 2px 6px;
    color        : #67e8f9;
    background   : rgba(0, 240, 255, 0.07);
    border       : 1px solid rgba(0, 240, 255, 0.18);
    border-radius: 4px;
    font-size    : 0.6rem;
    font-family  : monospace;
  }

  .source-code {
    margin     : 0;
    padding    : 10px 12px;
    max-height : 320px;
    overflow   : auto;
    color      : #bae6fd;
    background : rgba(0, 0, 0, 0.32);
    font-size  : 0.7rem;
    line-height: 1.55;
    white-space: pre;
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
            {@html renderMarkdown(parsed.think)}
          </div>
        </details>
      {/if}

      {#if parsed.strategist}
        <section class="strategist-output">
          <div class="artifact-header">
            <span class="artifact-icon">◈</span>
            <span>MISSION ASSESSMENT</span>
          </div>

          <div class="assessment-body">
            {@html renderMarkdown(
              parsed.strategist.assessment_report
            )}
          </div>

          <div class="next-move-card">
            <div class="next-move-header">
              <span>NEXT MOVE</span>

              <span
                class:finish-action={
                  parsed.strategist.next_move.action === "Finish"
                }
                class="action-badge"
              >
                {formatAction(
                  parsed.strategist.next_move.action
                )}
              </span>
            </div>

            <div class="next-move-intention">
              {parsed.strategist.next_move.intention}
            </div>

            {#if parsed.strategist.next_move.target}
              <div class="next-move-target">
                <span class="field-label">TARGET</span>
                <code>
                  {parsed.strategist.next_move.target}
                </code>
              </div>
            {/if}
          </div>

          {#if parsed.strategist.mission_graph}
            <details class="mission-box">
              <summary class="mission-summary">
                <span>🗺️</span>
                <span>MISSION GRAPH</span>
                <span class="graph-summary">
                  {graphSummary(
                    parsed.strategist.mission_graph
                  )}
                </span>
              </summary>

              <div class="mission-body">
                <pre><code>{prettyJson(
                  parsed.strategist.mission_graph
                )}</code></pre>
              </div>
            </details>
          {/if}
        </section>

      {:else if parsed.probe}
        <section class="probe-output">

          <!--
          <div class="artifact-header probe-header">
            <span class="artifact-icon">◈</span>
            <span>EXECUTION PROBE</span>

            <span class="probe-trial">
              TRY {parsed.probe.trial}/{parsed.probe.max_retries}
            </span>
          </div>

          <div class="probe-task">
            <span class="field-label">TASK</span>
            <span>{parsed.probe.task}</span>
          </div>
          -->

          <div
            class="verdict-card"
            class:verdict-ok={parsed.probe.sentinel_verdict.compliant}
            class:verdict-fail={!parsed.probe.sentinel_verdict.compliant}
          >
            <div class="verdict-status">
              <span class="probe-trial">
                TRY {parsed.probe.trial}/{parsed.probe.max_retries}
              </span>
              <span>
                {parsed.probe.sentinel_verdict.compliant ? "✓" : "✕"}
              </span>

              <span>
                {parsed.probe.sentinel_verdict.compliant
                  ? "COMPLIANT"
                  : "REJECTED"}
              </span>
            </div>

            <div class="verdict-score">
              <span class="field-label">SCORE</span>
              <strong>
                {parsed.probe.sentinel_verdict.score}/10
              </strong>
            </div>
          </div>

          {#if parsed.probe.sentinel_verdict.missing_requirements.length > 0}
            <div class="missing-requirements">
              <span class="field-label">
                MISSING REQUIREMENTS
              </span>

              <ul>
                {#each parsed.probe.sentinel_verdict.missing_requirements as requirement}
                  <li>{requirement}</li>
                {/each}
              </ul>
            </div>
          {/if}

          <div class="generated-response">

            <div class="generated-header">
              <span>GENERATED RESPONSE</span>

              <div class="runtime-badges">
                <span class="runtime-badge">
                  {parsed.probe.generated_response.environment}
                </span>

                <span class="runtime-badge">
                  {parsed.probe.generated_response.runtime}
                </span>
              </div>
            </div>

            <pre class="source-code"><code>{parsed.probe.generated_response.source}</code></pre>

          </div>

        </section>

      {:else if parsed.json}
        <details class="json-box">
          <summary class="json-summary">
            <span>◆</span>
            <span>STRUCTURED ARTIFACT</span>
          </summary>

          <pre class="json-body"><code>{prettyJson(
            parsed.json
          )}</code></pre>
        </details>

      {:else if parsed.final}
        {@html marked.parse(parsed.final)}
      {/if}

      {#if parsed.graph_legacy}
        <details class="mission-box" open={isStreaming}>
          <summary class="mission-summary">
            🗺️ // MISSION GRAPH PAYLOAD
          </summary>
          <div class="mission-body">
            <pre><code>{prettyJson(
              parsed.graph_legacy
            )}</code></pre>
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