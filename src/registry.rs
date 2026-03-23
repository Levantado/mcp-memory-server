use crate::graph::MemoryGraph;
use crate::models::MemoryScope;
use crate::storage;
use dashmap::DashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tracing::{info, error};

pub struct GraphRegistry {
    root_dir: PathBuf,
    // (ProjectID, Scope) -> MemoryGraph
    graphs: DashMap<(String, MemoryScope), Arc<MemoryGraph>>,
}

impl GraphRegistry {
    pub fn new<P: AsRef<Path>>(root_dir: P) -> Self {
        Self {
            root_dir: root_dir.as_ref().to_path_buf(),
            graphs: DashMap::new(),
        }
    }

    pub fn get_or_load(&self, project_id: &str, scope: MemoryScope) -> Arc<MemoryGraph> {
        let key = (project_id.to_string(), scope.clone());
        
        self.graphs.entry(key).or_insert_with(|| {
            // Lazy load
            let graph_path = self.resolve_path(project_id, &scope);
            
            let graph = match storage::load_from_file(&graph_path) {
                Ok(g) => {
                    info!("Loaded graph for project: {}, scope: {} from {:?}", project_id, scope, graph_path);
                    g
                }
                Err(e) => {
                    info!("Creating new graph for project: {}, scope: {} (Failed to load: {})", project_id, scope, e);
                    MemoryGraph::new()
                }
            };

            Arc::new(graph)
        }).value().clone()
    }

    fn resolve_path(&self, project_id: &str, scope: &MemoryScope) -> PathBuf {
        let path = self.root_dir.join(project_id);
        // Ensure project directory exists
        if let Err(e) = std::fs::create_dir_all(&path) {
            error!("Failed to create project directory {:?}: {}", path, e);
        }
        
        let file_name = match scope {
            MemoryScope::Shared => "shared.json".to_string(),
            MemoryScope::Agent(id) => format!("agent_{}.json", id),
        };
        
        path.join(file_name)
    }

    pub fn save_all(&self) {
        for entry in self.graphs.iter() {
            let (project_id, scope) = entry.key();
            let graph = entry.value();
            if graph.is_dirty() {
                let path = self.resolve_path(project_id, scope);
                if let Err(e) = storage::save_to_file(graph, &path) {
                    error!("Failed to save graph for project: {}, scope: {}: {}", project_id, scope, e);
                } else {
                    info!("Saved graph for project: {}, scope: {} to {:?}", project_id, scope, path);
                }
            }
        }
    }
}

