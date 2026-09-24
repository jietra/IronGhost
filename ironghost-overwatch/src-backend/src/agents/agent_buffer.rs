use crate::core::board::BoardMessage;

pub struct AgentBuffer {
    pub messages: Vec<BoardMessage>,
}

impl AgentBuffer {
    pub fn new() -> Self {
        Self { messages: Vec::new() }
    }

    pub fn push(&mut self, msg: BoardMessage) {
        self.messages.push(msg);
    }

    fn extract_recipients(msg: &BoardMessage) -> String {
        let tags: Vec<&str> = msg.content
            .split_whitespace()
            .filter_map(|word| {
                if !word.starts_with('@') || word.len() <= 1 {
                    return None;
                }
                // clean tag (ex: "@Coder," -> "@Coder")
                let clean_tag = word.trim_matches(|c: char| !c.is_alphanumeric() && c != '_');
                
                // Ignore empty tags or self-citation
                if clean_tag.len() <= 1 || &clean_tag[1..] == msg.agent {
                    None
                } else {
                    Some(clean_tag)
                }
            })
            .collect();

        if tags.is_empty() {
            "@All".to_string()
        } else {
            tags.join(", ")
        }
    }

    pub fn flush(&mut self, agent_name: &str) -> String {
        if self.messages.is_empty() {
            return String::new();
        }

        let mut prompt = String::from("[BLACKBOARD RECENT MESSAGES]\n");

        for msg in &self.messages {
            // Extract main recipient if any @tag
            let recipient = Self::extract_recipients(msg);
            
            prompt.push_str(&format!(
                "From: {} | To: {}\n> {}\n\n",
                msg.agent, recipient, msg.content
            ));
        }

        // Lock role: force agent to respond
        prompt.push_str(&format!(
            "[YOUR TURN]\nYou are {}. Respond strictly to tasks addressed to you or @All.\n{}:",
            agent_name, agent_name
        ));

        self.messages.clear();
        prompt
    }
}
