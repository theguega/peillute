
pub struct AppState {
    // --- Site Info ---
    pub site_id: String,
    pub nb_sites_on_network : i64,
    pub nb_neighbors: i64,
    pub peer_addrs: Vec<std::net::SocketAddr>,
    pub local_addr: std::net::SocketAddr,

    // --- Message Diffusion Info ---
    pub parent_addr: std::collections::HashMap<String, std::net::SocketAddr>,
    // message_initiator_id, parent_for_this_id
    pub nb_of_attended_neighbors: std::collections::HashMap<String, i64>,
    // message_initiator_id, number of attended for this id

    // --- Logical Clocks ---
    pub clocks: crate::clock::Clock,
}

impl AppState {

    #[allow(unused)]
    pub fn new(
        site_id: String,
        nb_neighbors: i64,
        nb_sites_on_network: i64,
        peer_addrs: Vec<std::net::SocketAddr>,
        local_addr: std::net::SocketAddr,
    ) -> Self {
        let clocks = crate::clock::Clock::new();
        let parent_addr = std::collections::HashMap::new();
        let nb_of_attended_neighbors = std::collections::HashMap::new();

        Self {
            site_id,
            nb_sites_on_network,
            nb_neighbors,
            peer_addrs,
            local_addr,
            parent_addr,
            nb_of_attended_neighbors,
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
            self.nb_sites_on_network+=1;
            self.clocks.add_peer(site_id);
            self.nb_of_attended_neighbors.insert(site_id.to_string(), self.peer_addrs.len() as i64);
            self.parent_addr.insert(site_id.to_string(), "0.0.0.0:0".parse().unwrap());
        }
    }

    pub fn remove_peer(&mut self, site_id: &str,addr: std::net::SocketAddr) {
        if let Some(pos) = self.peer_addrs.iter().position(|x| *x == addr) {
            self.peer_addrs.remove(pos);
            self.nb_neighbors -= 1;
            self.nb_of_attended_neighbors.insert(site_id.to_string(), self.peer_addrs.len() as i64);
            self.parent_addr.insert(site_id.to_string(), "0.0.0.0:0".parse().unwrap());
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
    #[allow(unused)]
    pub fn get_number_of_attended_neighbors(&self,initiator_id : String) -> i64 {
        self.nb_of_attended_neighbors.get(&initiator_id).copied().unwrap_or(0)
    }
    #[allow(unused)]
    pub fn set_number_of_attended_neighbors(&mut self,initiator_id : String, n: i64) {
        self.nb_of_attended_neighbors.insert(initiator_id, n);
    }
    pub fn get_parent_addr(&self, initiator_id : String) -> std::net::SocketAddr {
        self.parent_addr.get(&initiator_id).copied().unwrap_or("0.0.0.0".parse().unwrap())
    }

    pub fn set_parent_addr(&mut self, initiator_id : String, peer_adr : std::net::SocketAddr) {
        self.parent_addr.insert(initiator_id,peer_adr);
    }

    pub fn get_nb_sites_on_network(&self) -> i64 {
        self.nb_neighbors
    }
}

// Singleton
lazy_static::lazy_static! {
    pub static ref LOCAL_APP_STATE: std::sync::Arc<tokio::sync::Mutex<AppState>> =
        std::sync::Arc::new(tokio::sync::Mutex::new(AppState::new(
            "".to_string(), // empty site id at start
            0,
            0,
            Vec::new(),
            "0.0.0.0:0".parse().unwrap(),
        )));
}

#[cfg(test)]
mod tests {
    use log::__private_api::loc;
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
            AppState::new(site_id.clone(), num_sites, num_sites,peer_addrs.clone(), local_addr,);

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
            AppState::new(site_id.clone(), num_sites,num_sites, peer_addrs.clone(), local_addr,);

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
            AppState::new(site_id.clone(), num_sites, num_sites, peer_addrs.clone(), local_addr,);

        shared_state.add_peer("B", "127.0.0.1:8083".parse().unwrap());
        shared_state.remove_peer("127.0.0.1:8081".parse().unwrap());

        assert_eq!(shared_state.peer_addrs.len(), 2);
        assert_eq!(shared_state.nb_neighbors, 2);

        // Ensure the vector clock is updated correctly ??
        // Do we want the clock to remove the site when it is removed from the peer list?
        // assert!(!shared_state.clocks.get_vector().contains_key("B"));
    }
}
