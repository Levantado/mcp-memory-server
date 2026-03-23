use std::sync::{Arc, RwLock};
use tracing::debug;
use crate::graph::MemoryGraph;
use crate::protocol::*;
use serde_json::json;

pub async fn protocol_handle_request(
    graph: &Arc<MemoryGraph>,
    req: JsonRpcRequest,
    session_version: Option<Arc<RwLock<String>>>,
) -> Option<JsonRpcResponse> {
    let id_opt = req.id.clone();
    let method = req.method.as_str();
    debug!("Received method call: '{}' (id: {:?})", method, id_opt);

    // If it's a notification (no ID), we handle it but return None
    let is_notification = id_opt.is_none();
    let id = id_opt.unwrap_or(json!(null));

    if method == "tools/call" {
        if let Some(params_val) = req.params {
            if let Ok(call_params) = serde_json::from_value::<CallToolParams>(params_val) {
                let tool_name = call_params.name.as_str();
                let default_args = json!({});
                let args = call_params.arguments.as_ref().unwrap_or(&default_args);
                debug!("Tool execution: '{}' with args: {:?}", tool_name, args);

                let result = if tool_name.contains("read_graph") {
                    let (entities, relations) = graph.read_graph();
                    let res_json = json!({ "entities": entities, "relations": relations });
                    Some(json!({ "content": [{ "type": "text", "text": serde_json::to_string(&res_json).unwrap() }] }))
                } else if tool_name.contains("create_entities") {
                    if let Ok(p) = serde_json::from_value::<CreateEntitiesParams>(args.clone()) {
                        graph.create_entities(p.entities);
                        Some(json!({ "content": [{ "type": "text", "text": "Entities created successfully" }] }))
                    } else { None }
                } else if tool_name.contains("create_relations") {
                    if let Ok(p) = serde_json::from_value::<CreateRelationsParams>(args.clone()) {
                        graph.create_relations(p.relations);
                        Some(json!({ "content": [{ "type": "text", "text": "Relations created successfully" }] }))
                    } else { None }
                } else if tool_name.contains("add_observations") {
                    if let Ok(p) = serde_json::from_value::<AddObservationsParams>(args.clone()) {
                        for entry in p.observations {
                            graph.add_observations(&entry.entity_name, entry.contents);
                        }
                        Some(json!({ "content": [{ "type": "text", "text": "Observations added successfully" }] }))
                    } else { None }
                } else if tool_name.contains("search_nodes") {
                    if let Ok(p) = serde_json::from_value::<SearchNodesParams>(args.clone()) {
                        let (entities, relations) = graph.search_nodes(&p.query);
                        let res_json = json!({ "entities": entities, "relations": relations });
                        Some(json!({ "content": [{ "type": "text", "text": serde_json::to_string(&res_json).unwrap() }] }))
                    } else { None }
                } else if tool_name.contains("open_nodes") {
                    if let Ok(p) = serde_json::from_value::<OpenNodesParams>(args.clone()) {
                        let entities = graph.get_entities(p.names);
                        Some(json!({ "content": [{ "type": "text", "text": serde_json::to_string(&entities).unwrap() }] }))
                    } else { None }
                } else if tool_name.contains("delete_entities") {
                    if let Ok(p) = serde_json::from_value::<DeleteEntitiesParams>(args.clone()) {
                        graph.delete_entities(p.entity_names);
                        Some(json!({ "content": [{ "type": "text", "text": "Entities deleted successfully" }] }))
                    } else { None }
                } else if tool_name.contains("delete_observations") {
                    if let Ok(p) = serde_json::from_value::<DeleteObservationsParams>(args.clone()) {
                        graph.delete_observations(&p.entity_name, p.observations);
                        Some(json!({ "content": [{ "type": "text", "text": "Observations deleted successfully" }] }))
                    } else { None }
                } else if tool_name.contains("delete_relations") {
                    if let Ok(p) = serde_json::from_value::<DeleteRelationsParams>(args.clone()) {
                        graph.delete_relations(p.relations);
                        Some(json!({ "content": [{ "type": "text", "text": "Relations deleted successfully" }] }))
                    } else { None }
                } else if tool_name.contains("health_check") {
                    let errors = graph.validate_graph();
                    let status = if errors.is_empty() { "healthy" } else { "unhealthy" };
                    Some(json!({ "content": [{ "type": "text", "text": format!("Status: {}, Errors: {:?}", status, errors) }] }))
                } else {
                    None
                };

                if let Some(res) = result {
                    if is_notification { return None; }
                    return Some(JsonRpcResponse::success(id, res));
                }
            }
        }
        if is_notification { return None; }
        return Some(JsonRpcResponse::error(id, -32602, "Invalid tool call params"));
    }

    let resp = match method {
        "initialize" => {
            let client_version = req.params.as_ref()
                .and_then(|p| p.get("protocolVersion"))
                .and_then(|v| v.as_str())
                .unwrap_or("2024-11-05");
            
            let version = if client_version.starts_with("2024-") || client_version == "2025-11-25" {
                client_version
            } else {
                "2025-11-25"
            };

            if let Some(sv) = session_version {
                if let Ok(mut v) = sv.write() {
                    *v = version.to_string();
                }
            }

            Some(JsonRpcResponse::success(
                id,
                json!({
                    "protocolVersion": version,
                    "capabilities": {
                        "tools": { "listChanged": false },
                        "resources": { "subscribe": false, "listChanged": false },
                        "prompts": { "listChanged": false }
                    },
                    "serverInfo": {
                        "name": "mcp-memory-server-rust",
                        "version": env!("CARGO_PKG_VERSION")
                    }
                }),
            ))
        }
        "notifications/initialized" => {
            debug!("Client confirmed initialization.");
            None
        }
        "tools/list" | "list_tools" => Some(JsonRpcResponse::success(
            id,
            json!({
                "tools": [
                    {
                        "name": "mcp_memory_read_graph",
                        "description": "Read the entire knowledge graph",
                        "inputSchema": { "type": "object", "properties": {} }
                    },
                    {
                        "name": "mcp_memory_create_entities",
                        "description": "Create multiple new entities in the knowledge graph",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "entities": {
                                    "type": "array",
                                    "items": {
                                        "type": "object",
                                        "properties": {
                                            "name": { "type": "string" },
                                            "entityType": { "type": "string" },
                                            "observations": { "type": "array", "items": { "type": "string" } }
                                        },
                                        "required": ["name", "entityType", "observations"]
                                    }
                                }
                            },
                            "required": ["entities"]
                        }
                    },
                    {
                        "name": "mcp_memory_create_relations",
                        "description": "Create multiple new relations between entities",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "relations": {
                                    "type": "array",
                                    "items": {
                                        "type": "object",
                                        "properties": {
                                            "from": { "type": "string" },
                                            "to": { "type": "string" },
                                            "relationType": { "type": "string" }
                                        },
                                        "required": ["from", "to", "relationType"]
                                    }
                                }
                            },
                            "required": ["relations"]
                        }
                    },
                    {
                        "name": "mcp_memory_add_observations",
                        "description": "Add new observations to existing entities",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "observations": {
                                    "type": "array",
                                    "items": {
                                        "type": "object",
                                        "properties": {
                                            "entityName": { "type": "string" },
                                            "contents": { "type": "array", "items": { "type": "string" } }
                                        },
                                        "required": ["entityName", "contents"]
                                    }
                                }
                            },
                            "required": ["observations"]
                        }
                    },
                    {
                        "name": "mcp_memory_search_nodes",
                        "description": "Search for nodes in the knowledge graph based on a query",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "query": { "type": "string" }
                            },
                            "required": ["query"]
                        }
                    },
                    {
                        "name": "mcp_memory_open_nodes",
                        "description": "Open specific nodes in the knowledge graph by their names",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "names": { "type": "array", "items": { "type": "string" } }
                            },
                            "required": ["names"]
                        }
                    },
                    {
                        "name": "mcp_memory_delete_entities",
                        "description": "Delete multiple entities and their associated relations",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "entityNames": { "type": "array", "items": { "type": "string" } }
                            },
                            "required": ["entityNames"]
                        }
                    },
                    {
                        "name": "mcp_memory_delete_observations",
                        "description": "Delete specific observations from entities",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "entityName": { "type": "string" },
                                "observations": { "type": "array", "items": { "type": "string" } }
                            },
                            "required": ["entityName", "observations"]
                        }
                    },
                    {
                        "name": "mcp_memory_delete_relations",
                        "description": "Delete multiple relations from the knowledge graph",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "relations": {
                                    "type": "array",
                                    "items": {
                                        "type": "object",
                                        "properties": {
                                            "from": { "type": "string" },
                                            "to": { "type": "string" },
                                            "relationType": { "type": "string" }
                                        },
                                        "required": ["from", "to", "relationType"]
                                    }
                                }
                            },
                            "required": ["relations"]
                        }
                    },
                    {
                        "name": "mcp_memory_health_check",
                        "description": "Check the integrity of the knowledge graph",
                        "inputSchema": { "type": "object", "properties": {} }
                    }
                ]
            }),
        )),
        "resources/list" => Some(JsonRpcResponse::success(
            id,
            json!({ "resources": [] })
        )),
        "resources/templates/list" => Some(JsonRpcResponse::success(
            id,
            json!({ "resourceTemplates": [] })
        )),
        "prompts/list" => Some(JsonRpcResponse::success(
            id,
            json!({ "prompts": [] })
        )),
        _ => if is_notification { None } else { Some(JsonRpcResponse::error(id, -32601, "Method not found")) },
    };

    if is_notification { None } else { resp }
}
