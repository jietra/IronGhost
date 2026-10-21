// src/core/board.rs

/// Mission declarative view

use std::sync::Arc;
use tokio::sync::broadcast;
use serde::{Serialize, Deserialize};

use crate::{
    config::SystemRegistry,
    core::mission::{
        Mission,
        MissionGraph
    },
};

pub struct Board {
    pub messages: Vec<BoardMessage>,
    pub mission : Mission,
    pub tx      : broadcast::Sender<BoardEvent>,  // Board transmitter (sender)

    pub registry : Arc<SystemRegistry>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct BoardMessage {
    pub agent    : String,  // not of type Agent to allow human agent or a worker (not AI). TODO: consider adding a Type: Human, AI, Worker
    pub content  : String,
    pub timestamp: u64,
}

#[derive(Clone, Serialize)]
#[serde(tag = "type", content = "payload")]
pub enum BoardEvent {

    #[serde(rename = "message")]
    NewMessage(
        BoardMessage
    ),
    
    StreamStart { msg_id: String, agent: String },
    StreamChunk { msg_id: String, delta: String },
    StreamEnd   { msg_id: String },

    // dispatch commands (only emitted by system agent)
    DispatchCommand {
        mission_id  : String,
        to          : String,
        payload     : String,
    },

    // mission commands:
    NewMission {
        mission_prompt: String,
    },

    MissionGo {
        mission_id: String,
    },

    MissionPause {
        mission_id: String,
    },

    MissionResume {
        mission_id: String,
    },

    MissionAbort {
        mission_id: String,
    },

    // mission state:
    #[serde(rename = "mission_state")]
    MissionState(
        MissionGraph
    ),

    // knowledge events:
    #[serde(rename = "artifact_created")]
    ArtifactCreated {
        mission_id : String,
        artifact_id: String,
    },
    
    #[serde(rename = "investigation_state_updated")]
    InvestigationStateUpdated {
        mission_id: String,
    },

}

impl Board {
    pub fn new(registry : Arc<SystemRegistry>,) -> Board {
        let (tx, _) = broadcast::channel(1024);     // 1024 messages max history in broadcast; transmitter only (tx), no receiver (rx)
            Self {
                messages: Vec::new(),
                mission : Mission::default(),
                tx,
                registry,
            }
    }

    // publish message to board
    pub async fn publish(&mut self, msg: BoardMessage) {

        // add msg to board messages
        println!("[BOARD] add message from {} to board messages", msg.agent);

        // detect and extract mission graphe update
        if let Some(new_mission_graph) = Self::extract_mission_from_content(&msg.content) {
            println!("[BOARD] mission graph update detected");

            self.mission.graph = new_mission_graph.clone();

            // broadcast new mission state
            let _ = self.tx.send(BoardEvent::MissionState(new_mission_graph));
        }

        self.messages.push(msg.clone());
        
        // broadcast new publication
        println!("[BOARD] broadcast new publication");

        //let _ = self.tx.send(msg);
        let _ = self.tx.send(BoardEvent::NewMessage(msg));
    }

    /// Extract JSON bloc inside <mission_state>...</mission_state>
    fn extract_mission_from_content(content: &str) -> Option<MissionGraph> {
        let start_tag: &str = "<mission_state>";
        let end_tag  : &str = "</mission_state>";

        if let Some(start) = content.find(start_tag) {
            if let Some(end) = content[start..].find(end_tag) {
                let json_str: &str = &content[start + start_tag.len()..start + end];
                if let Ok(mission) = serde_json::from_str::<MissionGraph>(json_str.trim()) {
                    return Some(mission);
                }
            }
        }
        None
    }

    // generic event
    pub fn publish_event(&self, event: BoardEvent) {
        let _ = self.tx.send(event);
    }

    // current mission
    pub fn set_current_mission( &mut self, mission: Mission ) {
        self.mission = mission;
    }

    pub fn current_mission( &self ) -> Mission {
        self.mission.clone()
    }

    // source tree
    pub fn update_source_tree_list( &mut self, tree: String ) {
        self.mission
            .knowledge
            .source_tree_list = tree;
    }




/*
    // ------

    pub fn current_mission_id( &self ) -> Option<&str> {
        Some(&self.mission.id)
    }

    pub fn current_mission_prompt( &self ) -> Option<&str> {
        Some(&self.mission.prompt)
    }
    
    pub fn get_source_tree_list( &self ) -> Option<&str> {
        Some(&self.mission.knowledge.source_tree_list)
    }
    */

}