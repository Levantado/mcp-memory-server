use std::sync::{Arc, RwLock};
use tracing::debug;
use crate::graph::MemoryGraph;
use crate::protocol::*;
use serde_json::json;

pub async fn protocol_handle_request(
    graph: &Arc<MemoryGraph>,
    req: JsonRpcRequest,
    session_version: Option<Arc<RwLock<String>>>,
    storage_root: &str,
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

                let result = if tool_name == "mcp_memory_read_graph" || tool_name == "read_graph" {
                    let (entities, relations) = graph.read_graph();
                    let res_json = json!({ "entities": entities, "relations": relations });
                    Some(json!({ "content": [{ "type": "text", "text": serde_json::to_string(&res_json).unwrap() }] }))
                } else if tool_name == "mcp_memory_create_entities" || tool_name == "create_entities" {
                    if let Ok(p) = serde_json::from_value::<CreateEntitiesParams>(args.clone()) {
                        graph.create_entities(p.entities);
                        Some(json!({ "content": [{ "type": "text", "text": "Entities created successfully" }] }))
                    } else { None }
                } else if tool_name == "mcp_memory_create_relations" || tool_name == "create_relations" {
                    if let Ok(p) = serde_json::from_value::<CreateRelationsParams>(args.clone()) {
                        graph.create_relations(p.relations);
                        Some(json!({ "content": [{ "type": "text", "text": "Relations created successfully" }] }))
                    } else { None }
                } else if tool_name == "mcp_memory_add_observations" || tool_name == "add_observations" {
                    if let Ok(p) = serde_json::from_value::<AddObservationsParams>(args.clone()) {
                        for entry in p.observations {
                            graph.add_observations(&entry.entity_name, entry.contents);
                        }
                        Some(json!({ "content": [{ "type": "text", "text": "Observations added successfully" }] }))
                    } else { None }
                } else if tool_name == "mcp_memory_search_nodes" || tool_name == "search_nodes" {
                    if let Ok(p) = serde_json::from_value::<SearchNodesParams>(args.clone()) {
                        let (entities, relations) = graph.search_nodes(&p.query);
                        let res_json = json!({ "entities": entities, "relations": relations });
                        Some(json!({ "content": [{ "type": "text", "text": serde_json::to_string(&res_json).unwrap() }] }))
                    } else { None }
                } else if tool_name == "mcp_memory_open_nodes" || tool_name == "open_nodes" {
                    if let Ok(p) = serde_json::from_value::<OpenNodesParams>(args.clone()) {
                        let entities = graph.get_entities(p.names);
                        Some(json!({ "content": [{ "type": "text", "text": serde_json::to_string(&entities).unwrap() }] }))
                    } else { None }
                } else if tool_name == "mcp_memory_delete_entities" || tool_name == "delete_entities" {
                    if let Ok(p) = serde_json::from_value::<DeleteEntitiesParams>(args.clone()) {
                        graph.delete_entities(p.entity_names);
                        Some(json!({ "content": [{ "type": "text", "text": "Entities deleted successfully" }] }))
                    } else { None }
                } else if tool_name == "mcp_memory_delete_observations" || tool_name == "delete_observations" {
                    if let Ok(p) = serde_json::from_value::<DeleteObservationsParams>(args.clone()) {
                        graph.delete_observations(&p.entity_name, p.observations);
                        Some(json!({ "content": [{ "type": "text", "text": "Observations deleted successfully" }] }))
                    } else { None }
                } else if tool_name == "mcp_memory_delete_relations" || tool_name == "delete_relations" {
                    if let Ok(p) = serde_json::from_value::<DeleteRelationsParams>(args.clone()) {
                        graph.delete_relations(p.relations);
                        Some(json!({ "content": [{ "type": "text", "text": "Relations deleted successfully" }] }))
                    } else { None }
                } else if tool_name == "mcp_memory_health_check" || tool_name == "health_check" {
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
        "resources/list" => {
            let mut resources = vec![
                json!({
                    "uri": "mcp://resources/guidelines/effective_work",
                    "name": "Effective Work Policy",
                    "description": "Rules for working effectively in this project"
                }),
                json!({
                    "uri": "mcp://resources/guidelines/agent_usage",
                    "name": "Agent Memory Guidelines",
                    "description": "Instructions for using the memory graph"
                }),
                json!({
                    "uri": "mcp://projects/global/raw",
                    "name": "Global Knowledge Graph",
                    "description": "Raw JSON of the global shared memory"
                })
            ];

            // Add dynamic projects list
            if let Ok(entries) = std::fs::read_dir(storage_root) {
                for entry in entries.flatten() {
                    if entry.path().is_dir() {
                        if let Ok(name) = entry.file_name().into_string() {
                            resources.push(json!({
                                "uri": format!("mcp://projects/{}/raw", name),
                                "name": format!("Project Graph: {}", name),
                                "description": format!("Raw JSON of the {} shared memory", name)
                            }));
                        }
                    }
                }
            }

            Some(JsonRpcResponse::success(
                id,
                json!({ "resources": resources })
            ))
        },
        "resources/read" => {
            if let Some(params_val) = req.params {
                if let Some(uri) = params_val.get("uri").and_then(|v| v.as_str()) {
                    let content = match uri {
                        "mcp://resources/guidelines/effective_work" => {
                            let path = format!("{}/../effective_work.md", storage_root);
                            std::fs::read_to_string(path).unwrap_or_else(|_| "Effective work policy not found".to_string())
                        },
                        "mcp://resources/guidelines/agent_usage" => {
                            let path = format!("{}/../docs/AGENT_GUIDELINES.md", storage_root);
                            std::fs::read_to_string(path).unwrap_or_else(|_| "Agent guidelines not found".to_string())
                        },
                        u if u.starts_with("mcp://projects/") && u.ends_with("/raw") => {
                            let project_name = u.trim_start_matches("mcp://projects/").trim_end_matches("/raw");
                            let path = format!("{}/{}/shared.json", storage_root, project_name);
                            std::fs::read_to_string(path).unwrap_or_else(|_| "Graph not found or empty".to_string())
                        },
                        _ => "Resource not found".to_string()
                    };
                    
                    Some(JsonRpcResponse::success(
                        id,
                        json!({
                            "contents": [{
                                "uri": uri,
                                "mimeType": "text/plain",
                                "text": content
                            }]
                        })
                    ))
                } else {
                    Some(JsonRpcResponse::error(id, -32602, "Missing uri parameter"))
                }
            } else {
                Some(JsonRpcResponse::error(id, -32602, "Missing parameters"))
            }
        },
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
