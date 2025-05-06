use serde::{Deserialize, Serialize};
use std::net::SocketAddr;

use crate::clock::Clock;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Transaction {
    pub id: u64,
    pub user_id: String,
    pub amount: f64,
    pub description: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum NetworkMessageCode {
    Discovery,
    Transaction,
    Acknowledgment,
    Error,
    Disconnect,
    Sync,
}

impl NetworkMessageCode {
    #[allow(unused)]
    #[allow(dead_code)]
    pub fn code(&self) -> &'static str {
        match self {
            NetworkMessageCode::Discovery => "discovery",
            NetworkMessageCode::Transaction => "transaction",
            NetworkMessageCode::Acknowledgment => "acknowledgment",
            NetworkMessageCode::Error => "error",
            NetworkMessageCode::Disconnect => "disconnect",
            NetworkMessageCode::Sync => "sync",
        }
    }
    #[allow(unused)]
    #[allow(dead_code)]
    pub fn from_code(code: &str) -> Option<Self> {
        match code {
            "discovery" => Some(NetworkMessageCode::Discovery),
            "transaction" => Some(NetworkMessageCode::Transaction),
            "acknowledgment" => Some(NetworkMessageCode::Acknowledgment),
            "error" => Some(NetworkMessageCode::Error),
            "disconnect" => Some(NetworkMessageCode::Disconnect),
            "sync" => Some(NetworkMessageCode::Sync),
            _ => None,
        }
    }
}

// TODO : add message status (failed, success, etc.)

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Message {
    pub sender_id: String,
    pub sender_addr: SocketAddr,
    pub clock: Clock,
    pub message: String,
    pub code: NetworkMessageCode,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transaction_debug() {
        let transaction = Transaction {
            id: 1,
            user_id: "test_user".to_string(),
            amount: 100.0,
            description: "Test transaction".to_string(),
        };
        assert_eq!(
            format!("{:?}", transaction),
            "Transaction { id: 1, user_id: \"test_user\", amount: 100.0, description: \"Test transaction\" }"
        );
    }

    #[test]
    fn test_message_debug() {
        let clock = Clock::new();

        let message = Message {
            sender_id: "A".to_string(),
            sender_addr: "127.0.0.1:8080".parse().unwrap(),
            clock: clock,
            message: "Test message".to_string(),
            code: NetworkMessageCode::Transaction,
        };
        assert!(format!("{:?}", message).contains("Message { sender_id: \"A\""));
    }

    #[test]
    fn test_network_message_code_conversion() {
        let code = NetworkMessageCode::Transaction;
        assert_eq!(code.code(), "transaction");

        let from_code = NetworkMessageCode::from_code("transaction");
        assert_eq!(from_code, Some(NetworkMessageCode::Transaction));

        let invalid_code = NetworkMessageCode::from_code("invalid");
        assert_eq!(invalid_code, None);
    }
}
