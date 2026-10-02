// src/engine/registry.rs

use std::collections::HashMap;

pub struct AgentRegistry {
    workers: HashMap<String, String>,
}

impl AgentRegistry {
    pub fn new() -> Self {
        let mut workers = HashMap::new();

        workers.insert(
            "coder".into(),
            "/tmp/agent_coder.sock".into()
        );

        workers.insert(
            "sentinel-code".into(),
            "/tmp/sentinel_code.sock".into()
        );

        Self { workers }
    }

    pub fn resolve(&self, worker: &str) -> Option<&str> {
        self.workers
            .get(worker)
            .map(|s| s.as_str())
    }
}