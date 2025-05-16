pub struct AppState {
    // --- Site Info ---
    pub site_id: String,
    pub nb_neighbors: usize,
    pub peer_addrs: Vec<std::net::SocketAddr>,
    pub local_addr: std::net::SocketAddr,

    // --- Message Diffusion Info ---
    pub nb_of_attended_neighbors: usize,
    pub parent_address: std::net::SocketAddr,

    // --- Logical Clocks ---
    pub clocks: crate::clock::Clock,
}

impl AppState {
    #[allow(unused)]
    pub fn new(
        site_id: String,
        nb_neighbors: usize,
        local_addr: std::net::SocketAddr,
        peer_addrs: Vec<std::net::SocketAddr>,
        nb_of_attended_neighbors: usize,
        parent_address: std::net::SocketAddr
    ) -> Self {
        let clocks = crate::clock::Clock::new();

        Self {
            site_id,
            nb_neighbors,
            local_addr,
            peer_addrs,
            nb_of_attended_neighbors,
            parent_address,
            clocks
        }
    }
    #[allow(unused)]
    pub fn change_site_id(&mut self, site_id: &str) {
        self.clocks.change_current_site_id(&self.site_id, site_id);
        self.site_id = site_id.to_string();
    }

    pub fn add_peer(&mut self, site_id: &str, addr: std::net::SocketAddr) {
        if !self.peer_addrs.contains(&addr) {
            self.peer_addrs.push(addr);
            self.nb_neighbors += 1;
            self.clocks.add_peer(site_id);
        }
    }

    pub fn remove_peer(&mut self, addr: std::net::SocketAddr) {
        if let Some(pos) = self.peer_addrs.iter().position(|x| *x == addr) {
            self.peer_addrs.remove(pos);
            self.nb_neighbors -= 1;
            // TODO : decide what to do with the vector clock
            // self.vector_clock.remove(&addr); ?
        }
    }
    pub fn get_local_addr(&self) -> String {
        self.local_addr.to_string()
    }

    pub fn get_site_id(&self) -> &str {
        self.site_id.as_str()
    }

    pub fn get_peers(&self) -> Vec<std::net::SocketAddr> {
        self.peer_addrs.clone()
    }

    pub fn increment_lamport(&mut self) -> i64 {
        self.clocks.increment_lamport()
    }

    #[allow(unused)]
    #[allow(dead_code)]
    pub fn increment_vector(&mut self, site_id: &str) -> i64 {
        self.clocks.increment_vector(site_id)
    }

    pub fn increment_vector_current(&mut self) -> i64 {
        self.clocks.increment_vector(self.site_id.as_str())
    }

    pub fn get_lamport(&self) -> i64 {
        self.clocks.get_lamport()
    }
    #[allow(unused)]
    #[allow(dead_code)]
    pub fn get_vector(&self) -> &std::collections::HashMap<String, i64> {
        self.clocks.get_vector()
    }

    pub fn update_vector(&mut self, received_vc: &std::collections::HashMap<String, i64>) {
        self.clocks.update_vector(received_vc);
    }
    #[allow(unused)]
    #[allow(dead_code)]
    pub fn get_vector_clock(&self) -> Vec<i64> {
        self.clocks.get_vector_clock()
    }

    pub fn get_clock(&self) -> &crate::clock::Clock {
        &self.clocks
    }
    #[allow(unused)]
    #[allow(dead_code)]
    pub fn update_lamport(&mut self, received_lamport: i64) {
        self.clocks.update_lamport(received_lamport);
    }
    pub fn get_number_of_attended_neighbors(&self) -> usize {
        self.nb_of_attended_neighbors
    }
    pub fn set_number_of_attended_neighbors(&mut self, n: usize) {
        self.nb_of_attended_neighbors = n;
    }
    pub fn get_parent_address(&self) -> std::net::SocketAddr {
        self.parent_address
    }

    pub fn set_parent_address(&mut self, addr: std::net::SocketAddr) {
        self.parent_address = addr;
    }

    pub fn get_nb_sites_on_network(&self) -> usize {
        self.nb_neighbors
    }
}

// Singleton
lazy_static::lazy_static! {
    pub static ref LOCAL_APP_STATE: std::sync::Arc<tokio::sync::Mutex<AppState>> =
        std::sync::Arc::new(tokio::sync::Mutex::new(AppState::new(
            "".to_string(), // empty site id at start
            0,
            "0.0.0.0:0".parse().unwrap(),
            Vec::new(),
            0,
            "0.0.0.0:0".parse().unwrap(),
        )));
}

#[cfg(test)]
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
            AppState::new(site_id.clone(), num_sites, local_addr, peer_addrs.clone(),num_sites, "0.0.0.0:0".parse().unwrap(),);

        assert_eq!(shared_state.site_id, site_id);
        assert_eq!(shared_state.nb_neighbors, num_sites);
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
            AppState::new(site_id.clone(), num_sites, local_addr, peer_addrs.clone(),num_sites, "0.0.0.0:0".parse().unwrap(),);

        shared_state.add_peer("B", "127.0.0.1:8083".parse().unwrap());

        assert_eq!(shared_state.peer_addrs.len(), 3);
        assert_eq!(shared_state.nb_neighbors, 3);
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
            AppState::new(site_id.clone(), num_sites, local_addr, peer_addrs.clone(),num_sites, "0.0.0.0:0".parse().unwrap(),);

        shared_state.add_peer("B", "127.0.0.1:8083".parse().unwrap());
        shared_state.remove_peer("127.0.0.1:8081".parse().unwrap());

        assert_eq!(shared_state.peer_addrs.len(), 2);
        assert_eq!(shared_state.nb_neighbors, 2);

        // Ensure the vector clock is updated correctly ??
        // Do we want the clock to remove the site when it is removed from the peer list?
        // assert!(!shared_state.clocks.get_vector().contains_key("B"));
    }
}
