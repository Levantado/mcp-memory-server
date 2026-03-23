use crate::graph::MemoryGraph;
use crate::models::Graph;
use anyhow::{Context, Result};
use std::fs;
use std::io::Write;
use std::path::Path;

pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<MemoryGraph> {
    if !path.as_ref().exists() {
        return Ok(MemoryGraph::new());
    }
    let content = fs::read_to_string(&path)
        .with_context(|| format!("Failed to read memory file: {:?}", path.as_ref()))?;
    let graph_data: Graph =
        serde_json::from_str(&content).with_context(|| "Failed to parse memory JSON")?;
    Ok(MemoryGraph::from_serializable(graph_data))
}

pub fn save_to_file<P: AsRef<Path>>(graph: &MemoryGraph, path: P) -> Result<()> {
    let serializable = graph.to_serializable();
    let json =
        serde_json::to_string_pretty(&serializable).with_context(|| "Failed to serialize graph")?;

    let path_ref = path.as_ref();
    let temp_path = path_ref.with_extension("tmp");

    // Atomic write: write to temp file then rename
    let mut file = fs::File::create(&temp_path)
        .with_context(|| format!("Failed to create temp file: {:?}", temp_path))?;
    file.write_all(json.as_bytes())?;
    file.sync_all()?;

    fs::rename(&temp_path, path_ref)
        .with_context(|| format!("Failed to rename temp file to {:?}", path_ref))?;

    graph.clear_dirty();
    Ok(())
}
