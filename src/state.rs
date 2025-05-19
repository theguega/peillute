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
    /// Total number of sites in the network
    pub nb_sites_on_network: usize,
    /// List of peer addresses
    pub peer_addrs: Vec<std::net::SocketAddr>,
    /// Local address for this site
    pub local_addr: std::net::SocketAddr,

    /// Half-wave propagation state
    pub half_wave: HalfWaveState,

    // --- Logical Clocks ---
    /// Logical clock implementation for distributed synchronization
    pub clocks: crate::clock::Clock,
}

#[cfg(feature = "server")]
impl AppState {
    /// Creates a new AppState instance with the given configuration
    pub fn new(
        site_id: String,
        nb_sites_on_network: usize,
        local_addr: std::net::SocketAddr,
        peer_addrs: Vec<std::net::SocketAddr>,
    ) -> Self {
        let clocks = crate::clock::Clock::new();

        Self {
            site_id,
            nb_sites_on_network,
            local_addr,
            peer_addrs,
            clocks,
            half_wave: HalfWaveState {
                pending: std::collections::HashMap::new(),
                received: std::collections::HashSet::new(),
            },
        }
    }

    #[allow(unused)]
    /// Updates the site ID and adjusts the logical clock accordingly
    pub fn change_site_id(&mut self, site_id: &str) {
        self.clocks.change_current_site_id(&self.site_id, site_id);
        self.site_id = site_id.to_string();
    }

    /// Adds a new peer to the network and updates the logical clock
    pub fn add_peer(&mut self, site_id: &str, addr: std::net::SocketAddr) {
        if !self.peer_addrs.contains(&addr) {
            self.peer_addrs.push(addr);
            self.nb_sites_on_network += 1;
            self.clocks.add_peer(site_id);
        }
    }

    /// Removes a peer from the network
    pub fn remove_peer(&mut self, addr: std::net::SocketAddr) {
        if let Some(pos) = self.peer_addrs.iter().position(|x| *x == addr) {
            self.peer_addrs.remove(pos);
            self.nb_sites_on_network -= 1;
            // TODO : decide what to do with the vector clock
            // self.vector_clock.remove(&addr); ?
        }
    }

    /// Returns the local address as a string
    pub fn get_local_addr(&self) -> String {
        self.local_addr.to_string()
    }

    /// Returns the current site ID
    pub fn get_site_id(&self) -> &str {
        self.site_id.as_str()
    }

    /// Returns the number of sites on the network
    pub fn get_nb_sites_on_network(&self) -> usize {
        self.nb_sites_on_network
    }

    /// Returns a list of all peer addresses
    pub fn get_peers(&self) -> Vec<std::net::SocketAddr> {
        self.peer_addrs.clone()
    }

    /// Returns a list of peer addresses as strings
    pub fn get_peers_string(&self) -> Vec<String> {
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

    #[allow(unused)]
    /// Updates the Lamport clock with a received value
    pub fn update_lamport(&mut self, received_lamport: i64) {
        self.clocks.update_lamport(received_lamport);
    }

    /// Returns the current state of the half-wave propagation
    pub fn mark_half_wave_sent(&mut self, msg_id: &str) {
        let mut set = std::collections::HashSet::new();
        for peer in &self.peer_addrs {
            set.insert(*peer);
        }
        self.half_wave.pending.insert(msg_id.to_string(), set);
    }

    /// Marks a half-wave message as acknowledged by a specific peer
    pub fn mark_half_wave_ack(&mut self, msg_id: &str, from: &std::net::SocketAddr) {
        if let Some(pending) = self.half_wave.pending.get_mut(msg_id) {
            pending.remove(from);
            if pending.is_empty() {
                log::info!("Half-wave message {} fully acknowledged!", msg_id);
                self.half_wave.pending.remove(msg_id);
            }
        }
    }

    pub fn has_already_received_half_wave(&self, msg_id: &str) -> bool {
    self.half_wave.received.contains(msg_id)
    }

    pub fn mark_half_wave_received(&mut self, msg_id: &str) {
        self.half_wave.received.insert(msg_id.to_string());
    }

    
}

// Singleton
#[cfg(feature = "server")]
lazy_static::lazy_static! {
    pub static ref LOCAL_APP_STATE: std::sync::Arc<tokio::sync::Mutex<AppState>> =
        std::sync::Arc::new(tokio::sync::Mutex::new(AppState::new(
            "".to_string(), // empty site id at start
            0,
            "0.0.0.0:0".parse().unwrap(),
            Vec::new()
        )));
}

#[derive(Debug)]
pub struct HalfWaveState {
    pub pending: std::collections::HashMap<String, std::collections::HashSet<std::net::SocketAddr>>,
    pub received: std::collections::HashSet<String>,
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
            AppState::new(site_id.clone(), num_sites, local_addr, peer_addrs.clone());

        assert_eq!(shared_state.site_id, site_id);
        assert_eq!(shared_state.nb_sites_on_network, num_sites);
        assert_eq!(shared_state.peer_addrs, peer_addrs);
        assert_eq!(shared_state.clocks.get_vector().len(), 0); // Initially empty
    }

    #[test]
    fn test_add_peer() {
        let site_id = "A".to_string();
        let num_sites = 2;
        let peer_addrs = vec![
            "127.0.0.1:8081".parse().unwrap(),
            "127.0.0.1:8082".parse().unwrap(),
        ];
        let local_addr = "127.0.0.1:8080".parse().unwrap();
        let mut shared_state =
            AppState::new(site_id.clone(), num_sites, local_addr, peer_addrs.clone());

        shared_state.add_peer("B", "127.0.0.1:8083".parse().unwrap());

        assert_eq!(shared_state.peer_addrs.len(), 3);
        assert_eq!(shared_state.nb_sites_on_network, 3);
        assert!(shared_state.clocks.get_vector().contains_key("B"));
    }

    #[test]
    fn test_remove_peer() {
        let site_id = "A".to_string();
        let num_sites = 2;
        let peer_addrs = vec![
            "127.0.0.1:8081".parse().unwrap(),
            "127.0.0.1:8082".parse().unwrap(),
        ];
        let local_addr = "127.0.0.1:8080".parse().unwrap();
        let mut shared_state =
            AppState::new(site_id.clone(), num_sites, local_addr, peer_addrs.clone());

        shared_state.add_peer("B", "127.0.0.1:8083".parse().unwrap());
        shared_state.remove_peer("127.0.0.1:8081".parse().unwrap());

        assert_eq!(shared_state.peer_addrs.len(), 2);
        assert_eq!(shared_state.nb_sites_on_network, 2);

        // Ensure the vector clock is updated correctly ??
        // Do we want the clock to remove the site when it is removed from the peer list?
        // assert!(!shared_state.clocks.get_vector().contains_key("B"));
    }
}
