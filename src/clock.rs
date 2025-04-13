use crate::state::AppState;
use log::trace;
use std::sync::atomic::Ordering;

// --- Vector Clock ---
#[allow(unused)]
#[allow(dead_code)]
pub fn increment_vector(state: &mut AppState) -> Vec<u64> {
    let site_id = state.site_id;
    if site_id < state.vector_clock.len() {
        state.vector_clock[site_id].fetch_add(1, Ordering::SeqCst);
        trace!(
            "Site {}: Vector clock incremented at index {}",
            site_id, site_id
        );
    } else {
        log::error!(
            "Site {}: Invalid site_id for vector clock increment",
            site_id
        );
    }
    get_vector_clock(state)
}

#[allow(unused)]
#[allow(dead_code)]
pub fn update_vector_on_receive(state: &mut AppState, received_vc: &[u64]) -> Vec<u64> {
    let site_id = state.site_id;
    trace!(
        "Site {}: Updating vector clock on receive. Received VC: {:?}. Current VC: {:?}",
        site_id,
        received_vc,
        get_vector_clock(state)
    );

    if received_vc.len() != state.vector_clock.len() {
        log::warn!(
            "Site {}: Received vector clock of different size ({} vs {})",
            site_id,
            received_vc.len(),
            state.vector_clock.len()
        );
    } else {
        for i in 0..state.num_sites {
            if i != site_id {
                let current_val = state.vector_clock[i].load(Ordering::SeqCst);
                let received_val = received_vc[i];
                let max_val = current_val.max(received_val);
                state.vector_clock[i].store(max_val, Ordering::SeqCst);
            }
        }
    }

    // Increment own clock for the receive event AFTER updating from received vector
    if site_id < state.vector_clock.len() {
        state.vector_clock[site_id].fetch_add(1, Ordering::SeqCst);
        trace!(
            "Site {}: Vector clock incremented at index {} for receive event",
            site_id, site_id
        );
    } else {
        log::error!(
            "Site {}: Invalid site_id for vector clock increment post-receive",
            site_id
        );
    }
    get_vector_clock(state)
}

pub fn get_vector_clock(state: &AppState) -> Vec<u64> {
    state
        .vector_clock
        .iter()
        .map(|a| a.load(Ordering::SeqCst))
        .collect()
}

// --- Lamport Clock ---
#[allow(unused)]
#[allow(dead_code)]
pub fn increment_lamport_clock(state: &mut AppState) -> u64 {
    state.lamport_clock.fetch_add(1, Ordering::SeqCst);
    state.lamport_clock.load(Ordering::SeqCst)
}
#[allow(unused)]
#[allow(dead_code)]
pub fn get_lamport_clock(state: &AppState) -> u64 {
    state.lamport_clock.load(Ordering::SeqCst)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::AppState;
    use std::net::SocketAddr;

    #[test]
    fn test_increment_vector() {
        let site_id = 1;
        let num_sites = 3;
        let peer_addrs: Vec<SocketAddr> = Vec::new();
        let mut shared_state = AppState::new(site_id, num_sites, peer_addrs.clone());

        let initial_clock = get_vector_clock(&shared_state);
        let updated_clock = increment_vector(&mut shared_state);

        assert_eq!(updated_clock[site_id], initial_clock[site_id] + 1);
    }

    #[test]
    fn test_update_vector_on_receive() {
        let site_id = 1;
        let num_sites = 3;
        let peer_addrs: Vec<SocketAddr> = Vec::new();
        let mut shared_state = AppState::new(site_id, num_sites, peer_addrs.clone());

        let mut received_vc = vec![0; num_sites];
        received_vc[0] = 2;

        let initial_clock = get_vector_clock(&shared_state);
        let updated_clock = update_vector_on_receive(&mut shared_state, &received_vc);

        assert_eq!(updated_clock[site_id], initial_clock[site_id] + 1);
        assert_eq!(updated_clock[0], 2);
    }

    #[test]
    fn test_increment_lamport_clock() {
        let site_id = 1;
        let num_sites = 3;
        let peer_addrs: Vec<SocketAddr> = Vec::new();
        let mut shared_state = AppState::new(site_id, num_sites, peer_addrs.clone());

        let initial_clock = get_lamport_clock(&shared_state);
        let updated_clock = increment_lamport_clock(&mut shared_state);

        assert_eq!(updated_clock, initial_clock + 1);
    }

    #[test]
    fn test_get_lamport_clock() {
        let site_id = 1;
        let num_sites = 3;
        let peer_addrs: Vec<SocketAddr> = Vec::new();
        let shared_state = AppState::new(site_id, num_sites, peer_addrs.clone());

        assert_eq!(get_lamport_clock(&shared_state), 0);
    }
}
