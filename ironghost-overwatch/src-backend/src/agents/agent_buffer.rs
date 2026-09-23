pub struct AgentBuffer {
    pub messages: Vec<String>,
}

impl AgentBuffer {
    pub fn new() -> Self {
        Self { messages: Vec::new() }
    }

    pub fn push(&mut self, msg: &str) {
        self.messages.push(msg.to_string());
    }

    pub fn flush(&mut self) -> String {
        let prompt = self.messages.join("\n");
        self.messages.clear();
        prompt
    }
}
