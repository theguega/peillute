use std::net::SocketAddr;
use std::sync::atomic::AtomicU64;
// pour singleton
use lazy_static::lazy_static;
use std::sync::Arc;

#[allow(unused)]
#[derive(Debug)]
pub struct AppState {
    // --- Site Info ---
    pub site_id: usize,
    pub num_sites: usize,
    pub peer_addrs: Vec<SocketAddr>,
    pub local_addr: SocketAddr, // this includes the port if present
    // --- Application Data ---
    // infos such as transactions, etc.

    // --- Logical Clocks ---
    pub vector_clock: Vec<AtomicU64>,
    pub lamport_clock: AtomicU64,
    // --- Snapshot ---
    // snapshot of the state
}

impl AppState {
    #[allow(unused)]
    #[allow(dead_code)]
    pub fn new(
        site_id: usize,
        local_addr: SocketAddr,
        num_sites: usize,
        peer_addrs: Vec<SocketAddr>,
    ) -> Self {
        let vector_clock: Vec<AtomicU64> = (0..num_sites).map(|_| AtomicU64::new(0)).collect();

        Self {
            site_id,
            num_sites,
            local_addr,
            peer_addrs,
            vector_clock,
            lamport_clock: AtomicU64::new(0),
        }
    }

    pub fn add_peer(&mut self, addr: &str) {
        if let Ok(socket_addr) = addr.parse::<SocketAddr>() {
            if !self.peer_addrs.contains(&socket_addr) {
                self.peer_addrs.push(socket_addr);
                self.num_sites += 1;
                self.vector_clock.push(AtomicU64::new(0));
            }
        }
    }

    pub fn remove_peer(&mut self, addr: &str) {
        if let Ok(socket_addr) = addr.parse::<SocketAddr>() {
            if let Some(pos) = self.peer_addrs.iter().position(|x| *x == socket_addr) {
                self.peer_addrs.remove(pos);
                self.num_sites -= 1;
                self.vector_clock.remove(pos);
            }
        }
    }
    pub fn get_local_addr(&self) -> String {
        self.local_addr.to_string()
    }

    pub fn get_site_id(&self) -> usize {
        self.site_id
    }

    #[allow(unused)]
    #[allow(dead_code)]
    pub fn get_site(&self) -> usize {
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
        Arc::new(tokio::sync::Mutex::new(AppState {
            site_id: 0,
            num_sites: 0,
            local_addr: "0.0.0.0:0".parse().unwrap(),
            peer_addrs: Vec::new(),
            vector_clock: Vec::new(),
            lamport_clock: AtomicU64::new(0),
        }));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shared_state_new() {
        let site_id = 1;
        let num_sites = 3;
        let peer_addrs = vec![
            "127.0.0.1:8081".parse().unwrap(),
            "127.0.0.1:8082".parse().unwrap(),
        ];
        let local_addr = "127.0.0.1:8080".parse().unwrap();
        let shared_state = AppState::new(site_id, local_addr, num_sites, peer_addrs.clone());

        assert_eq!(shared_state.site_id, site_id);
        assert_eq!(shared_state.num_sites, num_sites);
        assert_eq!(shared_state.peer_addrs, peer_addrs);
        assert_eq!(shared_state.vector_clock.len(), num_sites);
    }
}
