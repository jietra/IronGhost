use tokio::sync::broadcast;
use serde::{Serialize, Deserialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MissionNode {
    pub id   : String,
    pub kind : String, // "goal", "subgoal", "task", "asset", "vuln", "risk"
    pub title: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MissionEdge {
    pub from    : String,
    pub to      : String,
    pub relation: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct Mission {
    pub nodes: Vec<MissionNode>,
    pub edges: Vec<MissionEdge>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct BoardMessage {
    pub agent    : String,  // not of type Agent to allow human agent or a worker (not AI). TODO: consider adding a Type: Human, AI, Worker
    pub content  : String,
    pub timestamp: u64,
}

pub struct Board {
    pub messages: Vec<BoardMessage>,
    pub mission : Mission,
    pub tx      : broadcast::Sender<BoardEvent>,  // Board transmitter (sender)
}

#[derive(Clone, Serialize)]
#[serde(tag = "type", content = "payload")]
pub enum BoardEvent {
    #[serde(rename = "message")]
    NewMessage(BoardMessage),
    #[serde(rename = "mission_state")]
    MissionState(Mission),
}

impl Board {
    pub fn new() -> Board {
        let (tx, _) = broadcast::channel(1024);     // 1024 messages max history in broadcast; transmitter only (tx), no receiver (rx)
            Self {
                messages: Vec::new(),
                mission : Mission::default(),
                tx,
            }
    }

    // publish to board
    pub async fn publish(&mut self, msg: BoardMessage) {

        // add msg to board messages
        println!("[BOARD] add message from {} to board messages", msg.agent);

        // detect and extract mission graphe update
        if let Some(new_mission) = Self::extract_mission_from_content(&msg.content) {
            println!("[BOARD] mission graph update detected");

            self.mission = new_mission.clone();

            // broadcast new mission state
            let _ = self.tx.send(BoardEvent::MissionState(new_mission));
        }

        self.messages.push(msg.clone());
        
        // broadcast new publication
        println!("[BOARD] broadcast new publication");

        //let _ = self.tx.send(msg);
        let _ = self.tx.send(BoardEvent::NewMessage(msg));
    }

    /// Extract JSON bloc inside <mission_state>...</mission_state>
    fn extract_mission_from_content(content: &str) -> Option<Mission> {
        let start_tag = "<mission_state>";
        let end_tag   = "</mission_state>";

        if let Some(start) = content.find(start_tag) {
            if let Some(end) = content[start..].find(end_tag) {
                let json_str = &content[start + start_tag.len()..start + end];
                if let Ok(mission) = serde_json::from_str::<Mission>(json_str.trim()) {
                    return Some(mission);
                }
            }
        }
        None
    }
}

impl Mission {
    pub fn new() -> Mission {
        Self {
            nodes: Vec::new(),
            edges: Vec::new(),
        }
    }
}