// src/assessment/action.rs

use serde::{Serialize, Deserialize};

//
// -------------------------------------
// Action
// = tool calling
// -------------------------------------
//

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct NextAction {
    pub intention       : String,
    pub action          : Action,
    pub target          : Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Action {
    FolderExplorer,
    FileReader,
    WebSearcher,
    WebReader,
    AskForMoreContext,
    Finish,
}

impl Default for Action {
    fn default() -> Self {
        Action::AskForMoreContext
    }
}