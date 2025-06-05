pub fn get_mac_address() -> Option<String> {
    use pnet::datalink;

    let interfaces = datalink::interfaces();
    for iface in interfaces {
        // Ignore loopback et interfaces sans MAC
        if iface.is_up() && !iface.is_loopback() {
            if let Some(mac) = iface.mac {
                if mac.octets() != [0, 0, 0, 0, 0, 0] {
                    return Some(mac.to_string().replace(":", ""));
                }
            }
        }
    }
    None
}

pub async fn reload_existing_site(
    peer_interaction_addr: std::net::SocketAddr,
    peers_addrs: Vec<std::net::SocketAddr>,
) -> bool {
    use crate::state::LOCAL_APP_STATE;
    use log::info;

    let (site_id, clock) = match crate::db::get_local_state() {
        Ok((site_id, clock)) => (site_id.clone(), clock),
        Err(_) => {
            info!("No existing site state found, creating a new one.");
            return false;
        }
    };

    {
        let mut state = LOCAL_APP_STATE.lock().await;
        state.site_id = site_id.clone();
        state.clocks = clock.clone();
        state.site_addr = peer_interaction_addr;
        state
            .parent_addr_for_transaction_wave
            .insert(site_id.clone(), peer_interaction_addr);
        state.peer_addrs = peers_addrs.clone();
    }

    info!("Existing site state reloaded");
    true
}
