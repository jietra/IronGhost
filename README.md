# IronGhost

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/Rust-1.75%2B-orange.svg?logo=rust)](https://www.rust-lang.org/)
[![C++17](https://img.shields.io/badge/C%2B%2B-17-blue.svg?logo=cplusplus)](https://en.cppreference.com/w/cpp/17)
[![Tauri](https://img.shields.io/badge/Tauri-v1%2Fv2-24C8D8.svg?logo=tauri)](https://tauri.app/)
[![Svelte](https://img.shields.io/badge/Svelte-3%2F4-FF3E00.svg?logo=svelte)](https://svelte.dev/)
[![Platform](https://img.shields.io/badge/Platform-macOS%20%7C%20Linux-lightgrey.svg)](https://apple.com)
[![Architecture](https://img.shields.io/badge/Architecture-Montesquieu--C3-red.svg)](#-philosophy-the-montesquieu-c3-protocol)

> **A distributed cognition system for mission-driven, zero-trust AI orchestration.**

---

AI systems should propose.
Deterministic systems should govern execution.

IronGhost is a distributed cognition system implemented through specialized agents, explicit memory structures, verification mechanisms and tools.

Rather than asking one model to perceive, remember, reason, verify and decide simultaneously, IronGhost decomposes cognition into narrow state transformations.
Persistent mission state, evidence and evolving beliefs are maintained outside individual LLM contexts.

```code
             Strategist
        Assessment / Attention
                  │
                  ▼
              Observer
          Local Perception
                  │
                  ▼
           Observer Artifact
                  │
                  ▼
          Knowledge System
       Cards / Claims / Evidence
                  │
                  ▼
          Mission Assessment
                  │
                  ▼
           Mission Graph
                  │
                  ▼
          Human Approval
                  │
                  ▼
          Execution Layer
```

Cognition is distributed across specialized components rather than concentrated inside a single long-lived agent. LLM contexts remain bounded and disposable; continuity is carried by explicit system state.

## 🎬 Demo video

[![Voir la démo](./screenshots/demo_miniature.png)](https://github.com/user-attachments/assets/82ffeccf-ba36-426b-b21d-a6ecfafab175)

## 🏛️ Philosophy: The "Montesquieu-C3" Protocol

**IronGhost** treats LLMs as *untrusted cognitive components*. They may propose plans, code and assessments, but they never execute actions directly. The Mission Graph is the sole source of truth.

### Explicit Mission State

IronGhost does not rely on LLM context as the authoritative memory of a mission.

Persistent mission state is represented explicitly through several complementary structures:

- **Assessment** — compact working memory and global decision state;
- **Knowledge Cards** — durable concept-level knowledge;
- **Knowledge Claims** — atomic propositions with epistemic status and evidence provenance;
- **Mission Graph** — structured operational plan proposed by the cognitive layer;
- **Execution DAG** — deterministic projection of approved operational work.

Agents are transient.

Contexts are disposable.

Messages are ephemeral.

Mission continuity belongs to explicit system state, not to any individual LLM.

### Mission-Centric Architecture

Traditional AI frameworks are agent-centric:

```code
Agent
 ↓
Tool
 ↓
Action
```

IronGhost is mission-centric:

```code
Agents
 ↓
Mission Graph
 ↓
Execution DAG
 ↓
Workers
```

Agents do not hold the system state. The Mission Graph does.

### Zero-Trust AI

**IronGhost** is built around a core architectural principle: **The Separation of Powers**, inspired by a Zero-Trust approach.

IronGhost assumes that every LLM can:

- hallucinate
- drift
- be manipulated
- produce unsafe code
- misunderstand mission context

Therefore:

- agents are isolated
- specialized Workers use an amnesia protocol
- Worker outputs are independently evaluated by Sentinels
- persistent knowledge is stored outside individual LLM contexts
- deterministic Executors retain operational authority
- execution authority remains outside the cognitive AI layer

### The Amnesia Protocol

IronGhost deliberately separates cognitive continuity from model context.

Specialized Workers:

- receive a bounded task and explicit context;
- perform one narrow cognitive operation;
- produce a structured artifact;
- are independently evaluated by a Sentinel;
- discard their inference context after completion.

Long-term continuity is externalized into explicit mission structures such as Assessments, Knowledge Cards, Claims and Mission Graphs.

This architecture reduces:

- cognitive drift;
- hidden state accumulation;
- context poisoning;
- accidental dependence on conversation history;
- non-reproducible implicit memory.

The goal is not to eliminate memory, but to move memory out of individual models and into explicit, inspectable system state.

### Recursive Assessment

The Strategist maintains a compact, disposable assessment of the current mission state.

Rather than following a fixed investigation workflow, it repeatedly decides whether the highest-value next step is to:

1. request another focused observation;
2. refine the current assessment from new evidence; or
3. stop assessment early and emit a Mission Graph for distributed operational
   investigation.

The assessment budget is therefore a maximum cognitive budget, not a required number of reasoning iterations.

This allows IronGhost to alternate between sequential investigation and graph-based delegation depending on expected information gain.

### Dual-Channel Governance

While traditional AI agent frameworks often grant Large Language Models (LLMs) direct code execution capabilities—creating security and reliability risks, providing agents with more memory and autonomy, IronGhost decouples cognition, orchestration, and execution into strict, isolated boundaries, and follows a "minimal" trust philosophy:

1. **Dual-Channel Governance**:
   * **Cognitive Cooperative Channel (Blackboard)**: A shared forum where cognitive AI agents collaborate, propose strategies, and propose a **Mission Graph**.
   * **Hierarchical Command & Control (C2)**: A strict command chain. Human Operators hold the highest authority and validate actions before execution. The Mission Graph, is rendered in real time, and is translated into an execution graph (deterministic) under strict automated (sentinels) and human supervision.
2. **Strict Isolation & Execution Safety**:
   * **No Direct AI Execution**: LLMs *cannot* execute code. They are either cognitive advisors, specialized agents or sentinels (with amnesia protocol) with no execution runtimes.
   * **Deterministic Executors**: Operational execution is offloaded to isolated, non-AI environments (e.g. dedicated Kali Linux VMs) running controlled execution logic.
   * **Safe Orchestration**: A central **Rust Orchestrator** manages message routing, state transitions, human validations, and alert escalations without using any internal LLM logic.
   * **Sentinel Filtering**: Specialized Workers are paired with independent Sentinels that evaluate their structured outputs before acceptance by the cognitive pipeline. Higher-level cognitive Agents may use different governance mechanisms.

Example of action process:

```code
+-------------------------------------------------------------------+
|                        Mission Graph State                        |
|                                                                   |
|   [Node 1: Recon] ----> [Node 2: Vuln Scan]                       |
|   (Status: SUCCESS)     (Status: PROPOSED)                        |
|                                 |                                 |
|                                 v (Human Approved)                |
|                         [Node 3: Exploitation]                    |
|                         (Status: PENDING_EXECUTION)               |
+---------------------------------+---------------------------------+
                                  |
                        (Rust Engine Dispatch)
                                  v
                  +---------------+---------------+
                  |  Sentinel Filter & Validation |
                  +---------------+---------------+
                                  |
                                  v
                  +---------------+---------------+
                  |  Deterministic Worker (Kali)  |
                  +-------------------------------+
```

### Agents, Workers, Sentinels and Executors

IronGhost distinguishes several cognitive and operational roles:

- **Agents** are higher-level cognitive actors such as the *Strategist*. They may maintain mission-level reasoning loops and communicate through the Board.

- **Workers** are specialized, task-bounded LLM agents. A Worker performs one narrow cognitive transformation such as source observation or knowledge consolidation. Workers do not communicate freely through the Board.

- **Sentinels** independently evaluate Worker outputs before those outputs are accepted by the surrounding cognitive pipeline. Worker + Sentinel pairs are orchestrated through the Dispatcher.

- **Executors** are deterministic, non-LLM components responsible for carrying out approved operational actions in controlled environments.

This separation keeps cognition, verification, orchestration and execution as distinct responsibilities.

---

## What IronGhost Is Not

IronGhost is not:

- an autonomous offensive AI
- a self-executing agent framework
- a direct Tool-Use runtime
- an AGI experimentation platform

IronGhost is a distributed cognition and mission orchestration architecture where specialized AI components perceive, consolidate, verify and plan, while explicit system state preserves continuity and deterministic systems retain execution authority.

---

## 🏗️ Architecture & Modules

The repository currently exposes the core infrastructure modules:

```code
                  +-----------------------------------+
                  |      Human Operator (Tauri UI)    |
                  +-----------------+-----------------+
                                    |
                                    v
+-----------------------------------+------------------------------------+
|                     IronGhost Overwatch (Rust)                         |
|   - Mission Graph & State Engine      - Human-in-the-Loop Validation   |
|   - Multi-Agent Message Router        - Guardrails & Command Sanitizer |
+-----------------+-------------------------------------+----------------+
                  |                                     |
        (Binary over UDS Socket)             (Sentinel filtering)
                  |                                     |
                  v                                     v
+-----------------+-----------------+     +-------------+----------------+
|      ironghost-services (C++)     |     |   ironghost-executor (Rust)  |
|           (LLM Layer)             |     |       (Execution Layer)      |
+-----------------------------------+     +-------------+----------------+
                                                        |
                                                        v
                                               +--------+----------+
                                               |  Local / Kali VM  |
                                               +-------------------+
```

### 📦 Published Modules

* **`ironghost-overwatch`** *(Rust / Tauri / Svelte)*
  * Mission-centric orchestration engine.
  * Manages Mission Graph lifecycle, agent coordination, validation workflows and human-in-the-loop governance.

* **`ironghost-services`** *(C++ / Native)*
  * High-performance local LLM services.
  * Binary IPC over Unix Domain Sockets.
  * Grammar-constrained outputs with GBNF.

* **`ironghost-executor`** *(Rust)*
  * Environment-agnostic execution layer.
  * Decouples execution environments from runtime implementations.
  * Supports isolated execution targets (currently Local and Kali).
  * Designed to evolve toward VM provisioning, disposable environments and stronger isolation.
  * See `ironghost-executor/README.md` for details.

---

## ⚡ Performance Highlights

* **Zero-Copy IPC**: Bypasses heavy REST/gRPC overhead by utilizing native C++ binary framing over Unix Domain Sockets, offering low inter-agent messaging latency. This is mainly designed for local implementations. Cloud implementations are possible but not shown here.
* **Grammar-Constrained Output**: Forces LLM inference to adhere strictly to GBNF grammars.
* **Memory Safety**: Core orchestration written in Rust, eliminating memory corruption vulnerabilities at the control layer.

---

## 📊 Comparative Architecture

How IronGhost compares to traditional agent frameworks:

| Feature / Philosophy | Traditional Frameworks<br>*(LangGraph, AutoGen, CrewAI)* | IronGhost<br>*(Montesquieu-C3)* |
| :--- | :--- | :--- |
| **Primary Philosophy** | Agent-centric (Autonomous execution & tool-use) | **Mission-centric (Zero-Trust & Separation of Powers)** |
| **Graph Construction** | Static / Hardcoded graph logic written by developer | **Dynamically generated by Cognitive LLMs (+ GBNF grammars)** |
| **Execution Runtime** | Direct tool execution within the agent runtime | **Decoupled execution via deterministic Workers (e.g. Kali VMs)** |
| **Cognitive Memory** | Persistent state & accumulated long-term memory | **Amnesia Protocol (Stateless LLMs, zero KV-cache pollution)** |
| **State Ownership** | Shared across agent conversations | **Exclusively owned by the Mission Graph** |
| **LLM Output Safety** | Post-processing validation (e.g. Pydantic retries) | **Token-level GBNF Grammar Constraints (C++17 / llama.cpp) + Sentinels** |
| **Control Layer Stack** | Python (High abstraction, REST / gRPC) | **Rust + C++17 (Memory safety, Zero-Copy UDS IPC)** |
| **Primary Target** | SaaS, B2B workflow automation, web tools | **SecOps, Offensive Security, Air-Gapped & Critical Systems** |

---

## 🚀 Quick Start

### Prerequisites

* Rust toolchain (`cargo`, `rustc`)
* C++17 compliant compiler (`clang++` or `g++`)
* Node (used by `Tauri` in dev mode for the desktop app)
* macOS (Apple Silicon optimized) or Linux

### Building `ironghost-services` (C++)

Install `llama.cpp` in project root (or change config in your service `CMakeLists.txt`):

```bash
git clone https://github.com/ggml-org/llama.cpp
```

Create a `models` directory in which you will put your models (`.gguf` format):

```bash
cd ironghost-services
mkdir models
```

Import your models in this directory.

Other sub-directories in `ironghost-services/` correspond to independant inference services (you can create as many services as you need - providing your machine can hold them).

Inference services can host different cognitive roles, including higher-level Agents, specialized Workers and Sentinels. Roles may use different models, system prompts and GBNF grammars depending on their cognitive responsibility.

Current examples include a Strategist as well as specialized Observer, Knowledge Worker and Sentinel services.

For each service:

- configure your `.env` file (in `ironghost-services/<your_service>/.env`) (examples provided, to be adapted according to your own setup) ;
- in `src/config/` you may provide a custom system prompt (`.txt`) and a grammar (`.gbnf`).

Build your service:

```bash
cd ironghost-services/<your_service>
mkdir build && cd build
rm -rf * 
cmake .. -DCMAKE_BUILD_TYPE=Release
make -j$(nproc 2>/dev/null || sysctl -n hw.ncpu)
```

Launch your services:

```bash
cd build/
./<your_service_binary>
```

> TODO: script this.

### Running `ironghost-overwatch` (Rust)

#### 1. launch *overwatch* backend:

```bash
cd ironghost-overwatch/src-backend
cargo run -p backend --bin main 
```

#### 2. launch *overwatch* app:

```bash
cd ironghost-overwatch
npm run tauri:dev 
```

## 🗺️ Roadmap

- [x] Lean C++ Binary LLM IPC Service over Unix Domain Sockets
- [x] Asynchronous Rust Board Orchestrator & Event Pipeline
- [x] Real-time Svelte/Tauri Tactical Overwatch Dashboard
- [x] Structured Mission Graph Generation (GBNF-constrained JSON)
- [x] Recursive Strategist assessment loop
- [x] Specialized Observer + Sentinel perception pipeline
- [x] Concept-routed Knowledge Cards
- [x] Atomic Knowledge Claims with evidence and epistemic status
- [x] Knowledge Worker + Sentinel consolidation pipeline
- [ ] Knowledge Events and belief-revision propagation
- [ ] Strategist auditing / governance
- [x] Implementation of Mission Graph runner with parallel execution (execution DiGraph)
- [x] Introduction of ironghost-executor service
- [x] Environment / Runtime execution abstraction
- [x] Local and Kali execution environments
- [ ] Automatic VM provisioning
- [ ] Disposable VM environments
- [ ] Formalized worker execution protocol
- [ ] Formalized C2 Protocol for Sandboxed Worker Execution
- [ ] Sentinel policy engine
- [ ] Mission Graph validator
- [ ] Worker cancellation propagation
- [ ] Mission versioning
- [ ] Mission graph signatures
- [ ] RBAC and operator workflow
- [ ] Formal Mission DSL
- [ ] Mission Graph persistence
- [ ] Mission replay & audit trail
- [ ] Deterministic policy engine

## 📄 License

Distributed under the MIT License. See `LICENSE` for more information.