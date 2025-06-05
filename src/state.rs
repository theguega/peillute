//! Application state management for Peillute
//!
//! This module handles the global application state, including site information,
//! peer management, and logical clock synchronization.

#[cfg(feature = "server")]
/// Represents the global state of a Peillute node
pub struct AppState {
    // --- Site Info ---
    /// Unique identifier for this site
    pub site_id: String,
    /// Unique address for this site
    pub site_addr: std::net::SocketAddr,
    /// Number of deg(1) neighbours connected
    pub nb_connected_neighbours: i64,
    /// List of peer addresses given in arguments at the launch of the application
    pub peer_addrs: Vec<std::net::SocketAddr>,
    /// List of deg(1) neighbours connected addresses
    pub connected_neighbours_addrs: Vec<std::net::SocketAddr>,

    // --- Message Diffusion Info ---
    /// Adress of the parent (deg(1) neighbour for this site) for a specific wave from initiator id
    pub parent_addr_for_transaction_wave: std::collections::HashMap<String, std::net::SocketAddr>,
    /// Number of response expected from our direct neighbours (deg(1) neighbours for this site) = nb of connected neighbours - 1 (parent) for a specific wave initiator id
    pub attended_neighbours_nb_for_transaction_wave: std::collections::HashMap<String, i64>,

    // --- Logical Clocks ---
    /// Logical clock implementation for distributed synchronization
    pub clocks: crate::clock::Clock,
}

#[cfg(feature = "server")]
impl AppState {
    /// Creates a new AppState instance with the given configuration
    pub fn new(
        site_id: String,
        nb_neighbors: i64,
        peer_addrs: Vec<std::net::SocketAddr>,
        local_addr: std::net::SocketAddr,
    ) -> Self {
        let clocks = crate::clock::Clock::new();
        let parent_addr = std::collections::HashMap::new();
        let nb_of_attended_neighbors = std::collections::HashMap::new();
        let in_use_neighbors = Vec::new();

        Self {
            site_id,
            nb_connected_neighbours: nb_neighbors,
            peer_addrs,
            site_addr: local_addr,
            parent_addr_for_transaction_wave: parent_addr,
            attended_neighbours_nb_for_transaction_wave: nb_of_attended_neighbors,
            connected_neighbours_addrs: in_use_neighbors,
            clocks,
        }
    }

    #[allow(unused)]
    /// Updates the site ID and adjusts the logical clock accordingly
    pub fn set_site_id(&mut self, site_id: &str) {
        self.clocks.change_current_site_id(&self.site_id, site_id);
        self.site_id = site_id.to_string();
    }

    /// Adds a new peer to the network and updates the logical clock
    #[allow(unused)]
    pub fn add_peer(&mut self, site_id: &str, addr: std::net::SocketAddr) {
        if !self.peer_addrs.contains(&addr) {
            self.peer_addrs.push(addr);
            self.clocks.add_peer(site_id);
            self.attended_neighbours_nb_for_transaction_wave
                .insert(site_id.to_string(), self.peer_addrs.len() as i64);
            self.parent_addr_for_transaction_wave
                .insert(site_id.to_string(), "0.0.0.0:0".parse().unwrap());
        }
    }

    /// Removes a peer from the network
    #[allow(unused)]
    pub fn remove_peer(&mut self, site_id: &str, addr: std::net::SocketAddr) {
        if let Some(pos) = self.peer_addrs.iter().position(|x| *x == addr) {
            self.peer_addrs.remove(pos);
            self.nb_connected_neighbours -= 1;
            self.attended_neighbours_nb_for_transaction_wave
                .insert(site_id.to_string(), self.peer_addrs.len() as i64);
            self.parent_addr_for_transaction_wave
                .insert(site_id.to_string(), "0.0.0.0:0".parse().unwrap());
            // TODO : decide what to do with the vector clock
            // self.vector_clock.remove(&addr); ?
        }
    }

    /// Returns the local address as a string
    pub fn get_site_addr(&self) -> String {
        self.site_addr.to_string()
    }

    /// Returns the current site ID
    pub fn get_site_id(&self) -> &str {
        self.site_id.as_str()
    }

    /// Returns a list of all peer addresses
    pub fn get_peers_addrs(&self) -> Vec<std::net::SocketAddr> {
        self.peer_addrs.clone()
    }

    /// Returns a list of peer addresses as strings
    pub fn get_peers_addrs_string(&self) -> Vec<String> {
        self.peer_addrs.iter().map(|x| x.to_string()).collect()
    }

    /// Increments the Lamport clock and returns the new value
    pub fn increment_lamport(&mut self) -> i64 {
        self.clocks.increment_lamport()
    }

    #[allow(unused)]
    /// Increments the vector clock for a specific site
    pub fn increment_vector(&mut self, site_id: &str) -> i64 {
        self.clocks.increment_vector(site_id)
    }

    /// Increments the vector clock for the current site
    pub fn increment_vector_current(&mut self) -> i64 {
        self.clocks.increment_vector(self.site_id.as_str())
    }

    /// Returns the current Lamport clock value
    pub fn get_lamport(&self) -> i64 {
        self.clocks.get_lamport()
    }

    /// Returns the current vector clock state
    pub fn get_vector(&self) -> &std::collections::HashMap<String, i64> {
        self.clocks.get_vector()
    }

    /// Updates the vector clock with received values
    pub fn update_vector(&mut self, received_vc: &std::collections::HashMap<String, i64>) {
        self.clocks.update_vector(received_vc);
    }

    #[allow(unused)]
    /// Returns the current vector clock as a list
    pub fn get_vector_clock(&self) -> Vec<i64> {
        self.clocks.get_vector_clock()
    }

    /// Returns a reference to the clock implementation
    pub fn get_clock(&self) -> &crate::clock::Clock {
        &self.clocks
    }

    /// Set the number of attended neighbors for the wave from initiator_id
    pub fn set_number_of_attended_neighbors(&mut self, initiator_id: String, n: i64) {
        self.attended_neighbours_nb_for_transaction_wave
            .insert(initiator_id, n);
    }

    /// Get the parent address for a node id
    pub fn get_parent_addr(&self, initiator_id: String) -> std::net::SocketAddr {
        self.parent_addr_for_transaction_wave
            .get(&initiator_id)
            .copied()
            .unwrap_or("0.0.0.0:0".parse().unwrap())
    }

    /// Sets the parent address for a node id
    pub fn set_parent_addr(&mut self, initiator_id: String, peer_adr: std::net::SocketAddr) {
        self.parent_addr_for_transaction_wave
            .insert(initiator_id, peer_adr);
    }

    /// Returns the number of deg(1) neighbors connected
    #[allow(unused)]
    pub fn get_nb_sites_on_network(&self) -> i64 {
        self.nb_connected_neighbours
    }
}

// Singleton
#[cfg(feature = "server")]
lazy_static::lazy_static! {
    pub static ref LOCAL_APP_STATE: std::sync::Arc<tokio::sync::Mutex<AppState>> =
        std::sync::Arc::new(tokio::sync::Mutex::new(AppState::new(
            "".to_string(), // empty site id at start
            0,
            Vec::new(),
            "0.0.0.0:0".parse().unwrap(),
        )));
}

#[cfg(test)]
#[cfg(feature = "server")]
mod tests {
    use super::*;

    #[test]
    fn test_new_state() {
        let site_id = "A".to_string();
        let num_sites = 2;
        let peer_addrs = vec![
            "127.0.0.1:8081".parse().unwrap(),
            "127.0.0.1:8082".parse().unwrap(),
        ];
        let local_addr: std::net::SocketAddr = format!("127.0.0.1:{}", 8080).parse().unwrap();
        let shared_state =
            AppState::new(site_id.clone(), num_sites, peer_addrs.clone(), local_addr);

        assert_eq!(shared_state.site_id, site_id);
        assert_eq!(shared_state.nb_connected_neighbours, num_sites);
        assert_eq!(shared_state.peer_addrs, peer_addrs);
        assert_eq!(shared_state.clocks.get_vector().len(), 0); // Initially empty
    }
}
