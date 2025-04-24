use lazy_static::lazy_static;
use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::atomic::AtomicU64;

pub struct AppState {
    // --- Site Info ---
    pub site_id: u64,
    pub nb_sites_on_network: usize,
    pub peer_addrs: Vec<SocketAddr>,
    pub local_addr: SocketAddr,

    // --- Logical Clocks ---
    pub vector_clock: Vec<AtomicU64>,
    pub lamport_clock: AtomicU64,
}

impl AppState {
    #[allow(unused)]
    pub fn new(
        site_id: u64,
        nb_sites_on_network: usize,
        local_addr: SocketAddr,
        peer_addrs: Vec<SocketAddr>,
    ) -> Self {
        let vector_clock: Vec<AtomicU64> = (0..nb_sites_on_network)
            .map(|_| AtomicU64::new(site_id))
            .collect();

        Self {
            site_id,
            nb_sites_on_network,
            local_addr,
            peer_addrs,
            vector_clock,
            lamport_clock: AtomicU64::new(site_id),
        }
    }

    pub fn add_peer(&mut self, addr: SocketAddr) {
        if !self.peer_addrs.contains(&addr) {
            self.peer_addrs.push(addr);
            self.nb_sites_on_network += 1;
            self.vector_clock.push(AtomicU64::new(0));
        }
    }

    pub fn remove_peer(&mut self, addr: SocketAddr) {
        if let Some(pos) = self.peer_addrs.iter().position(|x| *x == addr) {
            self.peer_addrs.remove(pos);
            self.nb_sites_on_network -= 1;
            self.vector_clock.remove(pos);
        }
    }
    pub fn get_local_addr(&self) -> String {
        self.local_addr.to_string()
    }

    pub fn get_site_id(&self) -> u64 {
        self.site_id
    }

    pub fn get_peers(&self) -> Vec<SocketAddr> {
        self.peer_addrs.clone()
    }

    pub fn get_vector_clock(&self) -> Vec<u64> {
        self.vector_clock
            .iter()
            .map(|vc| vc.load(std::sync::atomic::Ordering::SeqCst))
            .collect()
    }
}

// Singleton
lazy_static! {
    pub static ref GLOBAL_APP_STATE: Arc<tokio::sync::Mutex<AppState>> =
        Arc::new(tokio::sync::Mutex::new(AppState::new(
            0,
            0,
            "0.0.0.0:0".parse().unwrap(),
            Vec::new()
        )));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_state() {
        let site_id = 1;
        let num_sites = 3;
        let peer_addrs = vec![
            "127.0.0.1:8081".parse().unwrap(),
            "127.0.0.1:8082".parse().unwrap(),
        ];
        let local_addr: SocketAddr = format!("127.0.0.1:{}", 8080).parse().unwrap();
        let shared_state = AppState::new(site_id, num_sites, local_addr, peer_addrs.clone());

        assert_eq!(shared_state.site_id, site_id);
        assert_eq!(shared_state.nb_sites_on_network, num_sites);
        assert_eq!(shared_state.peer_addrs, peer_addrs);
        assert_eq!(shared_state.vector_clock.len(), num_sites);
    }

    #[test]
    fn test_add_peer() {
        let site_id = 1;
        let num_sites = 3;
        let peer_addrs = vec![
            "127.0.0.1:8081".parse().unwrap(),
            "127.0.0.1:8082".parse().unwrap(),
        ];
        let local_addr = "127.0.0.1:8080".parse().unwrap();
        let mut shared_state = AppState::new(site_id, num_sites, local_addr, peer_addrs.clone());

        shared_state.add_peer("127.0.0.1:8083".parse().unwrap());

        assert_eq!(shared_state.peer_addrs.len(), 3);
        assert_eq!(shared_state.nb_sites_on_network, 4);
        assert_eq!(shared_state.vector_clock.len(), 4);
    }

    #[test]
    fn test_remove_peer() {
        let site_id = 1;
        let num_sites = 3;
        let peer_addrs = vec![
            "127.0.0.1:8081".parse().unwrap(),
            "127.0.0.1:8082".parse().unwrap(),
        ];
        let local_addr = "127.0.0.1:8080".parse().unwrap();
        let mut shared_state = AppState::new(site_id, num_sites, local_addr, peer_addrs.clone());

        shared_state.remove_peer("127.0.0.1:8081".parse().unwrap());

        assert_eq!(shared_state.peer_addrs.len(), 1);
        assert_eq!(shared_state.nb_sites_on_network, 2);
        assert_eq!(shared_state.vector_clock.len(), 2);
    }
}
