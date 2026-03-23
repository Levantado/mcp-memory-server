use crate::models::{Entity, Relation, Graph};
use dashmap::DashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

pub struct MemoryGraph {
    entities: DashMap<Arc<str>, Arc<Entity>>,
    relations: DashMap<Relation, ()>, // Relation is already small enough with Arc<str>
    dirty: AtomicBool,
}

impl MemoryGraph {
    pub fn new() -> Self {
        Self {
            entities: DashMap::new(),
            relations: DashMap::new(),
            dirty: AtomicBool::new(false),
        }
    }

    pub fn from_serializable(graph: Graph) -> Self {
        let mem_graph = Self::new();
        mem_graph.create_entities(graph.entities);
        mem_graph.create_relations(graph.relations);
        mem_graph.dirty.store(false, Ordering::SeqCst);
        mem_graph
    }

    pub fn to_serializable(&self) -> Graph {
        let (entities, relations) = self.read_graph();
        Graph { entities, relations }
    }

    pub fn is_dirty(&self) -> bool {
        self.dirty.load(Ordering::SeqCst)
    }

    pub fn clear_dirty(&self) {
        self.dirty.store(false, Ordering::SeqCst);
    }

    fn mark_dirty(&self) {
        self.dirty.store(true, Ordering::SeqCst);
    }

    pub fn create_entities(&self, entities: Vec<Entity>) {
        if entities.is_empty() { return; }
        for entity in entities {
            let name_key: Arc<str> = entity.name.clone();
            self.entities.entry(name_key)
                .and_modify(|existing_arc| {
                    let existing = Arc::make_mut(existing_arc);
                    let mut changed = false;
                    
                    if existing.entity_type != entity.entity_type {
                        existing.entity_type = entity.entity_type.clone();
                        changed = true;
                    }
                    
                    for obs in &entity.observations {
                        if !existing.observations.contains(obs) {
                            existing.observations.push(obs.clone());
                            changed = true;
                        }
                    }
                    
                    if changed {
                        self.mark_dirty();
                    }
                })
                .or_insert_with(|| {
                    self.mark_dirty();
                    Arc::new(entity)
                });
        }
    }

    pub fn create_relations(&self, relations: Vec<Relation>) {
        if relations.is_empty() { return; }
        for relation in relations {
            // Memory optimization: reuse names from existing entities if possible
            // but for simplicity and safety we use the relation as provided
            if self.entities.contains_key(&relation.from) && self.entities.contains_key(&relation.to) {
                if self.relations.insert(relation, ()).is_none() {
                    self.mark_dirty();
                }
            }
        }
    }

    pub fn add_observations(&self, entity_name: &str, observations: Vec<String>) -> bool {
        if let Some(mut entity_arc) = self.entities.get_mut(entity_name) {
            let entity = Arc::make_mut(entity_arc.value_mut());
            let mut changed = false;
            for obs in observations {
                if !entity.observations.contains(&obs) {
                    entity.observations.push(obs);
                    changed = true;
                }
            }
            if changed {
                self.mark_dirty();
            }
            true
        } else {
            false
        }
    }

    pub fn search_nodes(&self, query: &str) -> (Vec<Entity>, Vec<Relation>) {
        let query_lower = query.to_lowercase();
        let mut matched_entities = Vec::new();
        let mut matched_names = std::collections::HashSet::new();

        // Optimized search: only one to_lowercase() per entity name/type/obs
        // Note: For massive graphs, we should pre-calculate lowercase versions
        for entry in self.entities.iter() {
            let entity = entry.value();
            let is_match = entity.name.to_lowercase().contains(&query_lower) 
               || entity.entity_type.to_lowercase().contains(&query_lower)
               || entity.observations.iter().any(|o| o.to_lowercase().contains(&query_lower));
            
            if is_match {
                matched_entities.push((**entity).clone());
                matched_names.insert(entity.name.clone());
            }
        }

        let mut matched_relations = Vec::new();
        for entry in self.relations.iter() {
            let rel = entry.key();
            if matched_names.contains(&rel.from) || matched_names.contains(&rel.to) {
                matched_relations.push(rel.clone());
            }
        }

        (matched_entities, matched_relations)
    }

    pub fn read_graph(&self) -> (Vec<Entity>, Vec<Relation>) {
        let entities = self.entities.iter().map(|e| (**e.value()).clone()).collect();
        let relations = self.relations.iter().map(|r| r.key().clone()).collect();
        (entities, relations)
    }

    pub fn validate_graph(&self) -> Vec<String> {
        let mut errors = Vec::new();
        for entry in self.relations.iter() {
            let rel = entry.key();
            if !self.entities.contains_key(&rel.from) {
                errors.push(format!("Relation from non-existent entity: {}", rel.from));
            }
            if !self.entities.contains_key(&rel.to) {
                errors.push(format!("Relation to non-existent entity: {}", rel.to));
            }
        }
        errors
    }

    pub fn get_entities(&self, names: Vec<String>) -> Vec<Entity> {
        let mut result = Vec::new();
        for name in names {
            if let Some(entity) = self.entities.get(name.as_str()) {
                result.push((**entity.value()).clone());
            }
        }
        result
    }

    pub fn delete_entities(&self, entity_names: Vec<String>) {
        for name in entity_names {
            if self.entities.remove(name.as_str()).is_some() {
                self.mark_dirty();
                let name_arc: Arc<str> = Arc::from(name.as_str());
                self.relations.retain(|rel, _| {
                    let to_remove = rel.from == name_arc || rel.to == name_arc;
                    if to_remove { self.mark_dirty(); }
                    !to_remove
                });
            }
        }
    }

    pub fn delete_observations(&self, entity_name: &str, observations: Vec<String>) -> bool {
        if let Some(mut entity_arc) = self.entities.get_mut(entity_name) {
            let entity = Arc::make_mut(entity_arc.value_mut());
            let initial_len = entity.observations.len();
            entity.observations.retain(|o| !observations.contains(o));
            if entity.observations.len() != initial_len {
                self.mark_dirty();
                true
            } else {
                false
            }
        } else {
            false
        }
    }

    pub fn delete_relations(&self, relations: Vec<Relation>) {
        for rel in relations {
            if self.relations.remove(&rel).is_some() {
                self.mark_dirty();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::Entity;

    #[test]
    fn test_create_and_search() {
        let graph = MemoryGraph::new();
        assert!(!graph.is_dirty());
        graph.create_entities(vec![
            Entity {
                name: Arc::from("Rust"),
                entity_type: Arc::from("Language"),
                observations: vec!["Memory safe".to_string()],
            }
        ]);
        assert!(graph.is_dirty());

        let (entities, _) = graph.search_nodes("Rust");
        assert_eq!(entities.len(), 1);
        assert_eq!(entities[0].name.as_ref(), "Rust");
    }
}
