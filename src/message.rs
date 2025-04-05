use serde::{Deserialize, Serialize};
use std::net::SocketAddr;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Transaction {
    pub id: u64,
    pub user_id: String,
    pub amount: f64,
    pub description: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Message {
    pub sender_id: usize,
    pub sender_addr: SocketAddr,
    pub sender_vc: Vec<u64>,
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
        let message = Message {
            sender_id: 1,
            sender_addr: "127.0.0.1:8080".parse().unwrap(),
            sender_vc: vec![1, 2, 3],
        };
        assert_eq!(
            format!("{:?}", message).split(",").collect::<Vec<&str>>()[0],
            "Message { sender_id: 1"
        );
    }
}
