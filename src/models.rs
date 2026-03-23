use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Entity {
    pub name: Arc<str>,
    pub entity_type: Arc<str>,
    pub observations: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub struct Relation {
    pub from: Arc<str>,
    pub to: Arc<str>,
    pub relation_type: Arc<str>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Graph {
    pub entities: Vec<Entity>,
    pub relations: Vec<Relation>,
}


#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MemoryScope {
    Shared,
    Agent(String), // Agent ID
}

impl std::fmt::Display for MemoryScope {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MemoryScope::Shared => write!(f, "shared"),
            MemoryScope::Agent(id) => write!(f, "agent_{}", id),
        }
    }
}
