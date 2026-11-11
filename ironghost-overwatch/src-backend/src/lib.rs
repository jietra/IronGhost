/// IronGhost Organization

// Board
pub mod core;

// Management
pub mod agents;     // parallel independant agents, with Board access

// Workers
pub mod workers;    // amnesic AI, with no Board access: "need to know" basis

// Analytics/Intelligence: knowlege management and viz
pub mod knowledge;          // source of truth + beliefs
pub mod knowledge_manager;  // manages knowledge lifecycle (incl. beliefs revision, knowledge consolidation...)
pub mod knowledge_graph;    // projection of current knowlege

pub mod assessment;         // snapshot/summary/index of mission knowledge and findings

pub mod observation;        // facts and hypotheses
pub mod observation_tools;  // deterministic observation tools (passive)

// Operations
pub mod engine;     // bridge to IgonGhost Execution framework

// Tech support
pub mod llm;        // bridge to IronGhost AI services
pub mod ui;
pub mod config;
pub mod stores;     // data management