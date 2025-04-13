use std::net::SocketAddr;
use std::sync::atomic::AtomicU64;

#[allow(unused)]
#[derive(Debug)]
pub struct AppState {
    // --- Site Info ---
    pub site_id: usize,
    pub num_sites: usize,
    pub peer_addrs: Vec<SocketAddr>,
    // --- Application Data ---
    // infos such as transactions, etc.

    // --- Logical Clocks ---
    pub vector_clock: Vec<AtomicU64>,
    pub lamport_clock: AtomicU64,
    // --- Snapshot ---
    // snapshot of the state
}

impl AppState {
    pub fn new(site_id: usize, num_sites: usize, peer_addrs: Vec<SocketAddr>) -> Self {
        let vector_clock: Vec<AtomicU64> = (0..num_sites).map(|_| AtomicU64::new(0)).collect();

        Self {
            site_id,
            num_sites,
            peer_addrs,
            vector_clock,
            lamport_clock: AtomicU64::new(0),
        }
    }
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
        let shared_state = AppState::new(site_id, num_sites, peer_addrs.clone());

        assert_eq!(shared_state.site_id, site_id);
        assert_eq!(shared_state.num_sites, num_sites);
        assert_eq!(shared_state.peer_addrs, peer_addrs);
        assert_eq!(shared_state.vector_clock.len(), num_sites);
    }
}
