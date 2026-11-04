use serde::{Serialize, Deserialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Position {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Position {
    pub fn distance_to(&self, other: &Position) -> f32 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        let dz = self.z - other.z;
        (dx * dx + dy * dy + dz * dz).sqrt()
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct SpatialEntityState {
    pub entity_id: String,
    pub position: Position,
    pub rotation: Vec<f32>,
    pub timestamp: u64,
    pub node_id: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct SpatialRegistry {
    pub entities: HashMap<String, SpatialEntityState>,
}

impl SpatialRegistry {
    pub fn new() -> Self {
        Self {
            entities: HashMap::new(),
        }
    }

    pub fn apply_delta(&mut self, incoming: SpatialEntityState) -> Result<(), &'static str> {
        let entry_opt = self.entities.get(&incoming.entity_id);

        if let Some(existing) = entry_opt {
            if incoming.timestamp < existing.timestamp {
                return Ok(());
            }

            let distance = existing.position.distance_to(&incoming.position);
            let time_delta = incoming.timestamp.saturating_sub(existing.timestamp);
            
            if time_delta < 5000 && distance > 50.0 {
                return Err("HOARE Contract Violation: Physical velocity bounds exceeded.");
            }
        }

        let entry = self.entities.entry(incoming.entity_id.clone()).or_insert(incoming.clone());

        if incoming.timestamp > entry.timestamp {
            *entry = incoming;
        } else if incoming.timestamp == entry.timestamp {
            if incoming.node_id > entry.node_id {
                *entry = incoming;
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_crdt_convergence() {
        let mut registry = SpatialRegistry::new();

        let state_a = SpatialEntityState {
            entity_id: "obj_1".to_string(),
            position: Position { x: 1.0, y: 2.0, z: 3.0 },
            rotation: vec![0.0, 0.0, 0.0, 1.0],
            timestamp: 100,
            node_id: "node_alpha".to_string(),
        };

        let state_b = SpatialEntityState {
            entity_id: "obj_1".to_string(),
            position: Position { x: 2.0, y: 2.0, z: 3.0 },
            rotation: vec![0.0, 0.0, 0.0, 1.0],
            timestamp: 100,
            node_id: "node_beta".to_string(),
        };

        registry.apply_delta(state_a).unwrap();
        registry.apply_delta(state_b.clone()).unwrap();

        assert_eq!(registry.entities.get("obj_1").unwrap().position.x, 2.0);
    }

    #[test]
    fn test_hoare_velocity_invariant() {
        let mut registry = SpatialRegistry::new();

        let initial = SpatialEntityState {
            entity_id: "obj_2".to_string(),
            position: Position { x: 0.0, y: 0.0, z: 0.0 },
            rotation: vec![0.0, 0.0, 0.0, 1.0],
            timestamp: 1000,
            node_id: "node_gamma".to_string(),
        };
        registry.apply_delta(initial).unwrap();

        let malicious = SpatialEntityState {
            entity_id: "obj_2".to_string(),
            position: Position { x: 1000.0, y: 1000.0, z: 1000.0 },
            rotation: vec![0.0, 0.0, 0.0, 1.0],
            timestamp: 1010,
            node_id: "node_gamma".to_string(),
        };

        let result = registry.apply_delta(malicious);
        assert!(result.is_err());
    }
}
