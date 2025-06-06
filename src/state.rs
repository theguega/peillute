//! Application state management for Peillute
//!
//! This module handles the global application state, including site information,
//! peer management, and logical clock synchronization.

#[cfg(feature = "server")]
/// Represents the global state of a Peillute node
pub struct AppState {
    // --- Site Info ---
    /// Unique identifier for this site
    site_id: String,
    /// Unique address for this site
    site_addr: std::net::SocketAddr,
    /// List of peer addresses given in arguments at the launch of the application
    cli_peer_addrs: Vec<std::net::SocketAddr>,
    /// List of deg(1) neighbours connected addresses
    connected_neighbours_addrs: Vec<std::net::SocketAddr>,
    /// Hashmap of sockets for each deg(1) neighbours
    neighbours_socket: std::collections::HashMap<std::net::SocketAddr, std::net::SocketAddr>,

    // --- Message Diffusion Info ---
    /// Adress of the parent (deg(1) neighbour for this site) for a specific wave from initiator id
    pub parent_addr_for_transaction_wave: std::collections::HashMap<String, std::net::SocketAddr>,
    /// Number of response expected from our direct neighbours (deg(1) neighbours for this site) = nb of connected neighbours - 1 (parent) for a specific wave initiator id
    pub attended_neighbours_nb_for_transaction_wave: std::collections::HashMap<String, i64>,

    // --- Logical Clocks ---
    /// Logical clock implementation for distributed synchronization
    clocks: crate::clock::Clock,
}

#[cfg(feature = "server")]
impl AppState {
    /// Creates a new AppState instance with the given configuration
    pub fn new(
        site_id: String,
        peer_addrs: Vec<std::net::SocketAddr>,
        local_addr: std::net::SocketAddr,
    ) -> Self {
        let clocks = crate::clock::Clock::new();
        let parent_addr = std::collections::HashMap::new();
        let nb_of_attended_neighbors = std::collections::HashMap::new();
        let in_use_neighbors = Vec::new();
        let sockets_for_connected_peers = std::collections::HashMap::new();

        Self {
            site_id,
            cli_peer_addrs: peer_addrs,
            neighbours_socket: sockets_for_connected_peers,
            site_addr: local_addr,
            parent_addr_for_transaction_wave: parent_addr,
            attended_neighbours_nb_for_transaction_wave: nb_of_attended_neighbors,
            connected_neighbours_addrs: in_use_neighbors,
            clocks,
        }
    }

    /// Sets the site ID at initialization
    pub fn init_site_id(&mut self, site_id: String) {
        self.site_id = site_id;
    }

    /// Sets the site address at initialization
    pub fn init_site_addr(&mut self, site_addr: std::net::SocketAddr) {
        self.site_addr = site_addr;
    }

    /// Sets the list of CLI peer addresses at initialization
    pub fn init_cli_peer_addrs(&mut self, cli_peer_addrs: Vec<std::net::SocketAddr>) {
        self.cli_peer_addrs = cli_peer_addrs;
    }

    /// Set the clock at initialization
    pub fn init_clock(&mut self, clock: crate::clock::Clock) {
        self.clocks = clock;
    }

    /// Initialize the parent of the current site as self for the wave protocol
    pub fn init_parent_addr_for_transaction_wave(&mut self) {
        self.parent_addr_for_transaction_wave
            .insert(self.site_id.clone(), self.site_addr.clone());
    }

    /// Adds a new peer to the network and updates the logical clock
    ///
    /// This function should be safe to call multiple times
    ///
    /// If a new site appear on the netword, every peers will launch a wave diffusion to announce the presence of this new site
    pub fn add_connected_peer(
        &mut self,
        new_site_id: &str,
        new_addr: std::net::SocketAddr,
        new_socket: std::net::SocketAddr,
        received_clock: crate::clock::Clock,
    ) {
        if !self.connected_neighbours_addrs.contains(&new_addr) {
            self.connected_neighbours_addrs.push(new_addr);
            self.clocks
                .update_clock(self.site_id.clone().as_str(), Some(&received_clock));
            self.attended_neighbours_nb_for_transaction_wave
                .insert(new_site_id.to_string(), self.cli_peer_addrs.len() as i64);
            self.parent_addr_for_transaction_wave
                .insert(new_site_id.to_string(), "0.0.0.0:0".parse().unwrap());
            self.neighbours_socket.insert(new_socket, new_addr);
        }
    }

    /// Removes a peer from the network
    ///
    /// This function should be safe to call multiple times
    ///
    /// If a site disappear from the network, every neighbours will detected the closing of the tcp connection and will launch a wave diffusion to announce the disappearance of this site
    ///
    /// If a site is closed properly, it will send a disconnect message to all its neighbours
    pub fn remove_peer(&mut self, addr_to_remove: std::net::SocketAddr) {
        if let Some(pos) = self
            .connected_neighbours_addrs
            .iter()
            .position(|x| *x == addr_to_remove)
        {
            self.connected_neighbours_addrs.remove(pos);

            // TODO: what happend if it occur during a wave diffusion ?
            // self.attended_neighbours_nb_for_transaction_wave
            //     .remove(&site_id_to_remove);
            // self.parent_addr_for_transaction_wave
            //     .remove(&site_id_to_remove);

            // We can keep the clock value for the site we want to remove
            // if the site re-appears, it will be updated with the new clock value
        }
    }

    /// Removes a peer from the network with only an address
    ///
    /// This function should be safe to call multiple times
    ///
    /// If a site disappear from the network, every neighbours will detected the closing of the tcp connection and will launch a wave diffusion to announce the disappearance of this site
    ///
    /// If a site is closed properly, it will send a disconnect message to all its neighbours
    pub fn remove_peer_from_socket_closed(&mut self, socket_to_remove: std::net::SocketAddr) {
        // Find the site adress based on the socket
        let Some(addr_to_remove) = self.neighbours_socket.get(&socket_to_remove) else {
            return;
        };

        // Remove the site from the list of connected neighbours
        self.connected_neighbours_addrs
            .retain(|x| x != addr_to_remove);

        // TODO: what happend if it occur during a wave diffusion ?

        // We can keep the clock value for the site we want to remove
        // if the site re-appears, it will be updated with the new clock value
    }

    /// Returns the local address as a string
    pub fn get_site_addr(&self) -> std::net::SocketAddr {
        self.site_addr.clone()
    }

    /// Returns a reference to the local address as &str
    pub fn get_site_id(&self) -> String {
        self.site_id.clone()
    }

    /// Returns a list of all peer addresses
    pub fn get_peers_addrs(&self) -> Vec<std::net::SocketAddr> {
        self.cli_peer_addrs.clone()
    }

    /// Returns a list of conncted neibhours
    pub fn get_connected_neighbours_addrs(&self) -> Vec<std::net::SocketAddr> {
        self.connected_neighbours_addrs.clone()
    }

    /// Add a connected neighbour to the list of connected neighbours
    pub fn add_connected_neighbour(&mut self, addr: std::net::SocketAddr) {
        self.connected_neighbours_addrs.push(addr);
    }

    /// Returns the clock of the site
    pub fn get_clock(&self) -> crate::clock::Clock {
        self.clocks.clone()
    }

    /// Set the number of attended neighbors for the wave from initiator_id
    pub fn set_number_of_attended_neighbors(&mut self, initiator_id: String, n: i64) {
        self.attended_neighbours_nb_for_transaction_wave
            .insert(initiator_id, n);
    }

    /// Get the list of attended neighbors for the wave from initiator_id
    pub fn get_parent_addr_for_transaction_wave(
        &self,
    ) -> std::collections::HashMap<String, std::net::SocketAddr> {
        self.parent_addr_for_transaction_wave.clone()
    }

    /// Get the list of attended neighbors for the wave from initiator_id
    pub fn get_attended_neighbours_nb_for_transaction_wave(
        &self,
    ) -> std::collections::HashMap<String, i64> {
        self.attended_neighbours_nb_for_transaction_wave.clone()
    }

    /// Get the parent (neighbour deg(1)) address for the wave from initiator_id
    pub fn get_the_parent_addr_for_wave(&self, initiator_id: String) -> std::net::SocketAddr {
        self.parent_addr_for_transaction_wave
            .get(&initiator_id)
            .copied()
            .unwrap_or("0.0.0.0:0".parse().unwrap())
    }

    /// Set the parent (neighbour deg(1)) address for the wave from initiator_id
    pub fn set_parent_addr(&mut self, initiator_id: String, peer_adr: std::net::SocketAddr) {
        self.parent_addr_for_transaction_wave
            .insert(initiator_id, peer_adr);
    }

    /// Returns the number of deg(1) neighbors connected
    pub fn get_nb_connected_neighbours(&self) -> i64 {
        self.connected_neighbours_addrs.len() as i64
    }

    /// Update the clock of the site
    pub async fn update_clock(&mut self, received_vc: Option<&crate::clock::Clock>) {
        // this wrapper is needed to ensure that the clock is saved
        // each time it is updated
        // please DO NOT call the `update_clock` method directly from the clock
        self.clocks.update_clock(&self.site_id, received_vc);
        self.save_local_state().await;
    }

    pub async fn save_local_state(&self) {
        // this is likely to be called whenever the clocks are updated
        let _ = crate::db::update_local_state(&self.site_id, self.clocks.clone());
    }
}

// Singleton
#[cfg(feature = "server")]
lazy_static::lazy_static! {
    pub static ref LOCAL_APP_STATE: std::sync::Arc<tokio::sync::Mutex<AppState>> =
        std::sync::Arc::new(tokio::sync::Mutex::new(AppState::new(
            "".to_string(), // empty site id at start
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
        let cli_site_id = "A".to_string();
        let num_sites = 2;
        let peer_addrs = vec![
            "127.0.0.1:8081".parse().unwrap(),
            "127.0.0.1:8082".parse().unwrap(),
        ];
        let local_addr: std::net::SocketAddr = format!("127.0.0.1:{}", 8080).parse().unwrap();
        let shared_state = AppState::new(cli_site_id.clone(), peer_addrs.clone(), local_addr);

        assert_eq!(shared_state.site_id, cli_site_id);
        assert_eq!(shared_state.cli_peer_addrs.len() as i64, num_sites);
        assert_eq!(shared_state.cli_peer_addrs, peer_addrs);
        assert_eq!(shared_state.clocks.get_vector_clock_map().len(), 0); // Initially empty
    }
}
