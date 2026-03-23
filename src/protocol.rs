use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Deserialize, Serialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub id: Option<Value>,
    pub method: String,
    pub params: Option<Value>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum RpcPayload {
    Single(JsonRpcRequest),
    Batch(Vec<JsonRpcRequest>),
}

#[derive(Debug, Serialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    pub id: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

#[derive(Debug, Serialize)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

impl JsonRpcResponse {
    pub fn success(id: Value, result: Value) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            result: Some(result),
            error: None,
        }
    }

    pub fn error(id: Value, code: i32, message: &str) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            result: None,
            error: Some(JsonRpcError {
                code,
                message: message.to_string(),
                data: None,
            }),
        }
    }
}

// MCP Specific Params
#[derive(Debug, Deserialize)]
pub struct CreateEntitiesParams {
    pub entities: Vec<crate::models::Entity>,
}

#[derive(Debug, Deserialize)]
pub struct CreateRelationsParams {
    pub relations: Vec<crate::models::Relation>,
}

#[derive(Debug, Deserialize)]
pub struct AddObservationsParams {
    #[serde(rename = "observations")]
    pub observations: Vec<ObservationEntry>,
}

#[derive(Debug, Deserialize)]
pub struct ObservationEntry {
    #[serde(rename = "entityName")]
    pub entity_name: String,
    pub contents: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct SearchNodesParams {
    pub query: String,
}

#[derive(Debug, Deserialize)]
pub struct OpenNodesParams {
    pub names: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct DeleteEntitiesParams {
    #[serde(rename = "entityNames")]
    pub entity_names: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct DeleteObservationsParams {
    #[serde(rename = "entityName")]
    pub entity_name: String,
    pub observations: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct DeleteRelationsParams {
    pub relations: Vec<crate::models::Relation>,
}

#[derive(Debug, Deserialize)]
pub struct CallToolParams {
    pub name: String,
    pub arguments: Option<Value>,
}
