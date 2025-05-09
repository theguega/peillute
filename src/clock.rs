use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Clock {
    lamport_clock: i64,
    vector_clock: HashMap<String, i64>, // site_id -> clock value
}

impl Clock {
    pub fn new() -> Self {
        Clock {
            lamport_clock: 0,
            vector_clock: HashMap::new(),
        }
    }

    pub fn add_peer(&mut self, site_id: &str) {
        if !self.vector_clock.contains_key(site_id) {
            self.vector_clock.insert(site_id.to_string(), 0);
        }
    }

    pub fn increment_lamport(&mut self) -> i64 {
        self.lamport_clock += 1;
        self.lamport_clock
    }

    pub fn increment_vector(&mut self, site_id: &str) -> i64 {
        let clock = self.vector_clock.entry(site_id.to_string()).or_insert(0);
        *clock += 1;
        *clock
    }

    pub fn get_lamport(&self) -> i64 {
        self.lamport_clock
    }

    pub fn get_vector(&self) -> &HashMap<String, i64> {
        &self.vector_clock
    }

    pub fn update_vector(&mut self, received_vc: &HashMap<String, i64>) {
        for (site_id, clock_value) in received_vc {
            let current_value = self.vector_clock.entry(site_id.clone()).or_insert(0);
            *current_value = (*current_value).max(*clock_value);
        }
    }

    pub fn get_vector_clock(&self) -> Vec<i64> {
        let mut vc: Vec<i64> = vec![0; self.vector_clock.len()];
        for (i, clock_value) in self.vector_clock.values().enumerate() {
            vc[i] = *clock_value;
        }
        vc
    }

    pub fn set_site_id(&mut self, site_id: &str) {
        if !self.vector_clock.contains_key(site_id) {
            self.vector_clock.insert(site_id.to_string(), 0);
        }
    }

    pub fn change_current_site_id(&mut self, old_site_id: &str, new_site_id: &str) {
        if let Some(value) = self.vector_clock.remove(old_site_id) {
            self.vector_clock.insert(new_site_id.to_string(), value);
        }
    }

    pub fn update_lamport(&mut self, received_lamport: i64) {
        self.lamport_clock = self.lamport_clock.max(received_lamport);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_increment_vector() {
        let mut clock = Clock::new();
        let site_id = "A";

        let initial_value = clock.get_vector().get(site_id).cloned().unwrap_or(0);
        let updated_value = clock.increment_vector(site_id);

        assert_eq!(updated_value, initial_value + 1);
    }

    #[test]
    fn test_update_vector() {
        let mut clock = Clock::new();
        let mut received_vc = HashMap::new();
        received_vc.insert("A".to_string(), 2);
        received_vc.insert("B".to_string(), 3);

        clock.update_vector(&received_vc);

        let vector_clock = clock.get_vector();
        assert_eq!(vector_clock.get("A").cloned().unwrap_or(0), 2);
        assert_eq!(vector_clock.get("B").cloned().unwrap_or(0), 3);
    }

    #[test]
    fn test_increment_lamport_clock() {
        let mut clock = Clock::new();

        let initial_value = clock.get_lamport();
        let updated_value = clock.increment_lamport();

        assert_eq!(updated_value, initial_value + 1);
    }

    #[test]
    fn test_get_lamport_clock() {
        let clock = Clock::new();

        assert_eq!(clock.get_lamport(), 0);
    }

    #[test]
    fn test_get_vector_clock() {
        let mut clock = Clock::new();
        clock.increment_vector("A");
        clock.increment_vector("B");

        let vector_clock = clock.get_vector_clock();
        assert_eq!(vector_clock.len(), 2);
        assert!(vector_clock.contains(&1));
    }
}
