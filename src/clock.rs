//! Logical clock implementation for distributed synchronization
//!
//! This module provides both Lamport and Vector clock implementations for
//! maintaining causal ordering of events in the distributed system.

#[cfg(feature = "server")]
/// Implements logical clocks for distributed synchronization
///
/// The Clock struct maintains both a Lamport clock for total ordering
/// and a vector clock for causal ordering of events across the distributed system.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Clock {
    /// Lamport clock value for total ordering of events
    lamport_clock: i64,
    /// Vector clock mapping site IDs to their clock values
    vector_clock: std::collections::HashMap<String, i64>, // site_id -> clock value
}

#[cfg(feature = "server")]
impl Clock {
    /// Creates a new Clock instance with initialized clocks
    pub fn new() -> Self {
        Clock {
            lamport_clock: 0,
            vector_clock: std::collections::HashMap::new(),
        }
    }

    /// Adds a new peer to the vector clock
    #[allow(unused)]
    pub fn add_peer(&mut self, site_id: &str) {
        if !self.vector_clock.contains_key(site_id) {
            self.vector_clock.insert(site_id.to_string(), 0);
        }
    }

    /// Increments the Lamport clock and returns the new value
    pub fn increment_lamport(&mut self) -> i64 {
        self.lamport_clock += 1;
        self.lamport_clock
    }

    /// Increments the vector clock for a specific site and returns the new value
    pub fn increment_vector(&mut self, site_id: &str) -> i64 {
        let clock = self.vector_clock.entry(site_id.to_string()).or_insert(0);
        *clock += 1;
        *clock
    }

    /// Returns the current Lamport clock value
    pub fn get_lamport(&self) -> i64 {
        self.lamport_clock
    }

    /// Returns a reference to the vector clock
    pub fn get_vector(&self) -> &std::collections::HashMap<String, i64> {
        &self.vector_clock
    }

    /// Updates the vector clock with received values, taking the maximum of local and received values
    pub fn update_vector(&mut self, received_vc: &std::collections::HashMap<String, i64>) {
        for (site_id, clock_value) in received_vc {
            let current_value = self.vector_clock.entry(site_id.clone()).or_insert(0);
            *current_value = (*current_value).max(*clock_value);
        }
    }

    /// Returns the vector clock as a list of values
    pub fn get_vector_clock(&self) -> Vec<i64> {
        let mut vc: Vec<i64> = vec![0; self.vector_clock.len()];
        for (i, clock_value) in self.vector_clock.values().enumerate() {
            vc[i] = *clock_value;
        }
        vc
    }

    /// Sets the clock value for a specific site ID
    pub fn set_site_id(&mut self, site_id: &str) {
        if !self.vector_clock.contains_key(site_id) {
            self.vector_clock.insert(site_id.to_string(), 0);
        }
    }

    #[allow(unused)]
    /// Updates the site ID in the vector clock while preserving its clock value
    pub fn change_current_site_id(&mut self, old_site_id: &str, new_site_id: &str) {
        if let Some(value) = self.vector_clock.remove(old_site_id) {
            self.vector_clock.insert(new_site_id.to_string(), value);
        }
    }

    #[allow(unused)]
    /// Updates the Lamport clock with a received value, taking the maximum
    pub fn update_lamport(&mut self, received_lamport: i64) {
        self.lamport_clock = self.lamport_clock.max(received_lamport);
    }
}

#[cfg(test)]
#[cfg(feature = "server")]
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
        let mut received_vc = std::collections::HashMap::new();
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
