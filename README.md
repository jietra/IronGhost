# IronGhost

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/Rust-1.75%2B-orange.svg?logo=rust)](https://www.rust-lang.org/)
[![C++17](https://img.shields.io/badge/C%2B%2B-17-blue.svg?logo=cplusplus)](https://en.cppreference.com/w/cpp/17)
[![Tauri](https://img.shields.io/badge/Tauri-v1%2Fv2-24C8D8.svg?logo=tauri)](https://tauri.app/)
[![Svelte](https://img.shields.io/badge/Svelte-3%2F4-FF3E00.svg?logo=svelte)](https://svelte.dev/)
[![Platform](https://img.shields.io/badge/Platform-macOS%20%7C%20Linux-lightgrey.svg)](https://apple.com)
[![Architecture](https://img.shields.io/badge/Architecture-Montesquieu--C3-red.svg)](#-philosophy-the-montesquieu-c3-protocol)

> **An Autonomous, Human-in-the-Loop Cyber Command & Control (C3) Architecture Powered by Multi-Agent Cognitive Orchestration.**

![IronGhost Overwatch Dashboard](./screenshots/dashboard.png)

---

## 🏛️ Philosophy: The "Montesquieu-C3" Protocol

**IronGhost** is built around a core architectural principle: **The Separation of Powers**. 

Traditional AI agent frameworks often grant Large Language Models (LLMs) direct code execution capabilities—creating catastrophic security and reliability risks. IronGhost solves this by decoupling cognition, orchestration, and execution into strict, isolated boundaries:

1. **Dual-Channel Governance**:
   * **Cognitive Cooperative Channel (Blackboard)**: A shared forum where specialized LLM agents collaborate, propose strategies, and build a real-time **Mission Knowledge Graph**.
   * **Hierarchical Command & Control (C2)**: A strict military-grade command chain. Human Operators hold the highest authority and must validate actions before execution.
2. **Strict Isolation & Execution Safety**:
   * **No Direct AI Execution**: LLMs *cannot* execute code. They are cognitive advisors, not execution runtimes.
   * **Deterministic Workers**: Execution is offloaded to isolated, non-AI worker environments (e.g., dedicated Kali Linux VMs) running deterministic, audited scripts.
   * **Safe Orchestration**: A central **Rust Orchestrator** manages message routing, state transitions, human validations, and alert escalations without using any internal LLM logic.
   * **Sentinel Filtering**: A special agent, *Sentinel*, is tasked to filter all codes before they reach workers. Sentinel is isolated from other AI agents, and has an *amnesia* protocol (stateless - clear KV-cache etc.).

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

---

## 🏗️ Architecture & Modules

The repository currently exposes the core infrastructure modules:

```code
                  +-----------------------------------+
                  |      Human Operator (Tauri UI)    |
                  +-----------------+-----------------+
                                    | (WebSocket)
                                    v
+-----------------------------------+-----------------------------------+
|                     IronGhost Overwatch (Rust)                        |
|   - Mission Graph & State Engine      - Human-in-the-Loop Validation  |
|   - Multi-Agent Message Router        - Guardrails & Command Sanitizer|
+-----------------+---------------------------------+-------------------+
                  |                                 |
        (Binary over UDS Socket)        (C2 Link + Sentinel filtering)
                  |                                 |
                  v                                 v
+-----------------+-----------------+     +---------+-------------------+
|      ironghost-services (C++)     |     |   Deterministic Workers     |
|   - Custom Lean LLM Engine        |     |   - Kali Linux Target VM    |
|   - Zero-Copy Binary Protocol     |     |   - Isolated Script Exec    |
+-----------------------------------+     +-----------------------------+
```

> Note: the C2 Link, Sentinel filtering, and determistic workers are not currently published.

### 📦 Published Modules

* **`ironghost-overwatch`** *(Rust / Tauri / Svelte)*:
  * The memory-safe, asynchronous orchestrator.
  * Manages agent lifecycle, event broadcasting, and mission topology state.
  * Renders a real-time 3D/2D tactical mission graph and operational chat interface.
* **`ironghost-services`** *(C++ / Native)*:
  * High-performance, lean LLM inference service designed for local execution (model agnostic).
  * Communicates via custom **Binary IPC over Unix Domain Sockets (UDS)** for ultra-low latency and zero-copy performance.

---

## ⚡ Performance Highlights

* **Zero-Copy IPC**: Bypasses heavy REST/gRPC overhead by utilizing native C++ binary framing over Unix Domain Sockets, offering near-zero inter-agent messaging latency.
* **Grammar-Constrained Output**: Forces LLM inference to adhere strictly to GBNF grammars, guaranteeing 100% valid JSON payload generation for mission graph updates.
* **Memory Safety**: Core orchestration written in Rust, eliminating memory corruption vulnerabilities at the control layer.

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

Other sub-directories in `ironghost-services/` correspond to an independant inference service (you can create as many services as you need). Three examples are given in this repo: `agent-strategist`, `agent-coder` and `agent-generic`.

For each service:

- configure your `.env` file (in `ironghost-services/agent-<your_service>/.env`) (examples provided, to be adapted your own setup) ;
- in `src/prompts/` you may provide a system prompt (`.txt`) and a grammar (`.gbnf`).

Build your service:

```bash
cd ironghost-services/agent-<your_service>
mkdir build && cd build
rm -rf * 
cmake .. -DCMAKE_BUILD_TYPE=Release
make -j$(nproc 2>/dev/null || sysctl -n hw.ncpu)
```

Launch your services:

```bash
cd build/
./agent_<your_service>
```

> TODO: script this.

### Running `ironghost-overwatch` (Rust)

```bash
cd ironghost-overwatch
cargo run --release
```

> Note that, at this stage of development, agents react to tagging `@<name_of_you_agent>` in board messages. Automatic tagging will be implemented in future version.

#### For dev mode:

##### 1. launch *overwatch* backend:

```bash
cd ironghost-overwatch/src-backend
cargo run -p backend --bin main 
```

##### 2. launch *overwatch* frontend (accesible at `http://localhost:5173/`):

```bash
cd ironghost-overwatch/src-front
npm run dev
```

##### 3. launch *overwatch* app (for desktop app view):

```bash
cd ironghost-overwatch
npm run tauri dev 
```

## 🗺️ Roadmap

- [x] Lean C++ Binary LLM IPC Service over Unix Domain Sockets
- [x] Asynchronous Rust Board Orchestrator & Event Pipeline
- [x] Real-time Svelte/Tauri Tactical Overwatch Dashboard
- [x] Dynamic Mission Knowledge Graph Generation (<mission_state>)
- [ ] Integration of Deterministic Kali Linux Execution Workers
- [ ] Formalized C2 Protocol for Sandboxed Worker Execution

## 📄 License

Distributed under the MIT License. See `LICENSE` for more information.