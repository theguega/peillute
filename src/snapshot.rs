#[derive(Clone, serde::Serialize, serde::Deserialize, Debug, Eq, PartialEq, Hash)]
pub struct TxSummary {
    pub lamport_time: i64,
    pub source_node: String,
}

impl From<&crate::db::Transaction> for TxSummary {
    fn from(tx: &crate::db::Transaction) -> Self {
        Self {
            lamport_time: tx.lamport_time,
            source_node: tx.source_node.clone(),
        }
    }
}

#[derive(Clone)]
pub struct LocalSnapshot {
    pub site_id: String,
    pub vector_clock: std::collections::HashMap<String, i64>,
    pub tx_log: std::collections::HashSet<TxSummary>,
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct GlobalSnapshot {
    pub all_transactions: std::collections::HashSet<TxSummary>,
    pub missing: std::collections::HashMap<String, std::collections::HashSet<TxSummary>>,
}

impl GlobalSnapshot {
    pub fn is_consistent(snaps: &[LocalSnapshot]) -> bool {
        for si in snaps {
            for sj in snaps {
                if let (Some(&cij), Some(&cjj)) = (
                    si.vector_clock.get(&sj.site_id),
                    sj.vector_clock.get(&sj.site_id),
                ) {
                    if cij > cjj {
                        return false;
                    }
                }
            }
        }
        true
    }
}

pub struct SnapshotManager {
    pub expected: i64,
    pub received: Vec<LocalSnapshot>,
}

impl SnapshotManager {
    pub fn new(expected: i64) -> Self {
        Self {
            expected,
            received: Vec::new(),
        }
    }

    pub fn push(&mut self, resp: crate::message::SnapshotResponse) -> Option<GlobalSnapshot> {
        self.received.push(LocalSnapshot {
            site_id: resp.site_id.clone(),
            vector_clock: resp.clock.get_vector().clone(),
            tx_log: resp.tx_log.into_iter().collect(),
        });

        if (self.received.len() as i64) < self.expected {
            return None;
        }

        if GlobalSnapshot::is_consistent(&self.received) {
            return Some(self.build_snapshot(&self.received));
        }

        // Back-track to the last consistent snapshot by computing the minimum vector clock
        // V_j = min_i Ci[j], where Ci[j] is the clock value for site j in snapshot i.
        // This ensures that we only consider transactions that are consistent across all snapshots.

        // Compute the minimum vector clock (vmin) across all received snapshots.
        let mut vmin: std::collections::HashMap<String, i64> = std::collections::HashMap::new();
        for snap in &self.received {
            for (site, &val) in &snap.vector_clock {
                // Update vmin for each site to the minimum value observed across all snapshots.
                vmin.entry(site.clone())
                    .and_modify(|m| *m = (*m).min(val))
                    .or_insert(val);
            }
        }

        // Create a new list of snapshots with trimmed vector clocks and transaction logs.
        let mut trimmed: Vec<LocalSnapshot> = Vec::new();
        for mut s in self.received.clone() {
            // Limit the vector clock for the current site to the minimum value in vmin.
            let lim = *vmin.get(&s.site_id).unwrap_or(&0);
            s.vector_clock.insert(s.site_id.clone(), lim);

            // Filter the transaction log to only include transactions that are consistent
            // with the minimum vector clock for their source node.
            let tx_keep: std::collections::HashSet<_> = s
                .tx_log
                .into_iter()
                .filter(|t| t.lamport_time <= *vmin.get(&t.source_node).unwrap_or(&0))
                .collect();
            s.tx_log = tx_keep;

            trimmed.push(s);
        }

        Some(self.build_snapshot(&trimmed))
    }

    fn build_snapshot(&self, snaps: &[LocalSnapshot]) -> GlobalSnapshot {
        let mut union: std::collections::HashSet<TxSummary> = std::collections::HashSet::new();
        for s in snaps {
            union.extend(s.tx_log.iter().cloned());
        }

        let mut miss: std::collections::HashMap<String, std::collections::HashSet<TxSummary>> =
            std::collections::HashMap::new();
        for s in snaps {
            let diff: std::collections::HashSet<_> = union.difference(&s.tx_log).cloned().collect();
            if !diff.is_empty() {
                miss.insert(s.site_id.clone(), diff);
            }
        }
        GlobalSnapshot {
            all_transactions: union,
            missing: miss,
        }
    }
}

pub async fn start_snapshot() -> Result<(), Box<dyn std::error::Error>> {
    let local_txs = crate::db::get_local_transaction_log()?;
    let summaries: Vec<TxSummary> = local_txs.iter().map(|t| t.into()).collect();

    let (site_id, clock, expected) = {
        let st = crate::state::LOCAL_APP_STATE.lock().await;
        (
            st.get_site_id().to_string(),
            st.get_clock().clone(),
            st.nb_neighbors + 1,
        )
    };

    {
        let mut mgr = LOCAL_SNAPSHOT_MANAGER.lock().await;
        mgr.expected = expected;
        mgr.received.clear();
        mgr.push(crate::message::SnapshotResponse {
            site_id: site_id.clone(),
            clock: clock.clone(),
            tx_log: summaries.clone(),
        });
    }

    crate::network::send_message_to_all(
        None,
        crate::message::NetworkMessageCode::SnapshotRequest,
        crate::message::MessageInfo::None,
    )
    .await?;

    Ok(())
}

pub async fn persist(snapshot: &GlobalSnapshot) -> std::io::Result<()> {
    use std::io::Write;

    let site_id = {
        let st = crate::state::LOCAL_APP_STATE.lock().await;
        st.get_site_id().to_string()
    };

    let ts = chrono::Local::now().format("%Y%m%d_%H%M%S");
    let filename = format!("snapshot_{}_{}.json", site_id, ts);

    let mut file = std::fs::File::create(&filename)?;
    let json = serde_json::to_string_pretty(snapshot).unwrap();
    file.write_all(json.as_bytes())?;
    log::info!("Snapshot saved in {}", filename);
    Ok(())
}

lazy_static::lazy_static! {
    pub static ref LOCAL_SNAPSHOT_MANAGER: tokio::sync::Mutex<SnapshotManager> =
        tokio::sync::Mutex::new(SnapshotManager::new(0));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mk_clock(pairs: &[(&str, i64)]) -> crate::clock::Clock {
        let mut c = crate::clock::Clock::new();
        let mut m = std::collections::HashMap::new();
        for (id, v) in pairs {
            m.insert((*id).to_string(), *v);
        }
        c.update_vector(&m);
        c
    }

    fn resp(site: &str, vc: &[(&str, i64)], txs: &[TxSummary]) -> crate::message::SnapshotResponse {
        crate::message::SnapshotResponse {
            site_id: site.to_string(),
            clock: mk_clock(vc),
            tx_log: txs.to_vec(),
        }
    }

    #[test]
    fn consistency_ok() {
        let s1 = LocalSnapshot {
            site_id: "A".into(),
            vector_clock: std::collections::HashMap::from_iter([("A".into(), 1), ("B".into(), 0)]),
            tx_log: std::collections::HashSet::new(),
        };
        let s2 = LocalSnapshot {
            site_id: "B".into(),
            vector_clock: std::collections::HashMap::from_iter([("A".into(), 1), ("B".into(), 1)]),
            tx_log: std::collections::HashSet::new(),
        };
        assert!(GlobalSnapshot::is_consistent(&[s1, s2]));
    }

    #[test]
    fn consistency_violation() {
        let s1 = LocalSnapshot {
            site_id: "A".into(),
            vector_clock: std::collections::HashMap::from_iter([("A".into(), 2), ("B".into(), 2)]),
            tx_log: std::collections::HashSet::new(),
        };
        let s2 = LocalSnapshot {
            site_id: "B".into(),
            vector_clock: std::collections::HashMap::from_iter([("A".into(), 1), ("B".into(), 1)]),
            tx_log: std::collections::HashSet::new(),
        };
        assert!(!GlobalSnapshot::is_consistent(&[s1, s2]));
    }

    #[test]
    fn push_waits_for_expected() {
        let mut mgr = SnapshotManager::new(2);
        let tx = TxSummary {
            lamport_time: 1,
            source_node: "A".into(),
        };
        let r1 = resp("A", &[("A", 1)], &[tx.clone()]);
        assert!(mgr.push(r1).is_none());
        assert_eq!(mgr.received.len(), 1);
    }

    #[test]
    fn push_detects_incoherence() {
        let mut mgr = SnapshotManager::new(2);
        let bad_r1 = resp("A", &[("A", 2), ("B", 2)], &[]);
        let bad_r2 = resp("B", &[("A", 1), ("B", 1)], &[]);
        assert!(mgr.push(bad_r1).is_none());
        let snap = mgr.push(bad_r2).expect("back-tracked snapshot");
        assert!(GlobalSnapshot::is_consistent(&[LocalSnapshot {
            site_id: "dummy".into(),
            vector_clock: std::collections::HashMap::new(),
            tx_log: snap.all_transactions.clone()
        }]));
        assert!(snap.missing.is_empty() || !snap.missing.contains_key("A"));
    }

    #[test]
    fn push_computes_missing_and_dedup() {
        let mut mgr = SnapshotManager::new(2);
        let t1 = TxSummary {
            lamport_time: 10,
            source_node: "A".into(),
        };
        let t2 = TxSummary {
            lamport_time: 11,
            source_node: "B".into(),
        };

        let r1 = resp("A", &[("A", 1)], &[t1.clone()]);
        let r2 = resp("B", &[("B", 1)], &[t1.clone(), t2.clone()]);

        let _ = mgr.push(r1);
        let gs = mgr.push(r2).expect("snapshot ready");

        assert_eq!(gs.all_transactions.len(), 2);
        assert_eq!(
            gs.missing["A"],
            std::collections::HashSet::from_iter([t2.clone()])
        );
        assert!(!gs.missing.contains_key("B"));
    }

    #[test]
    fn consistency_handles_missing_columns() {
        let a = LocalSnapshot {
            site_id: "A".into(),
            vector_clock: std::collections::HashMap::from_iter([("A".into(), 3)]),
            tx_log: std::collections::HashSet::new(),
        };
        let b = LocalSnapshot {
            site_id: "B".into(),
            vector_clock: std::collections::HashMap::from_iter([("B".into(), 1)]),
            tx_log: std::collections::HashSet::new(),
        };
        assert!(GlobalSnapshot::is_consistent(&[a, b]));
    }

    #[test]
    fn backtrack_trims_future_transactions() {
        let mut mgr = SnapshotManager::new(2);

        let t1 = TxSummary {
            lamport_time: 1,
            source_node: "A".into(),
        };
        let t3 = TxSummary {
            lamport_time: 3,
            source_node: "A".into(),
        };
        let t5 = TxSummary {
            lamport_time: 5,
            source_node: "A".into(),
        };

        let r_a = resp(
            "A",
            &[("A", 5), ("B", 2)], // ← incohérence ici
            &[t1.clone(), t3.clone(), t5.clone()],
        );

        let r_b = resp("B", &[("A", 2), ("B", 1)], &[]);

        let _ = mgr.push(r_a);
        let snap = mgr.push(r_b).expect("snapshot after back-track");

        assert!(snap.all_transactions.contains(&t1));
        assert!(!snap.all_transactions.contains(&t5));
    }

    #[test]
    fn union_is_deduplicated() {
        let mut mgr = SnapshotManager::new(2);
        let tx = TxSummary {
            lamport_time: 7,
            source_node: "A".into(),
        };

        let r1 = resp("A", &[("A", 1)], &[tx.clone()]);
        let r2 = resp("B", &[("B", 1)], &[tx.clone()]);

        let _ = mgr.push(r1);
        let gs = mgr.push(r2).expect("snapshot ready");
        assert_eq!(gs.all_transactions.len(), 1);
    }
}
