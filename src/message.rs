#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq)]
pub struct Transaction {
    pub id: u64,
    pub user_id: String,
    pub amount: f64,
    pub description: String,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq)]
pub enum NetworkMessageCode {
    Discovery,
    Transaction,
    Acknowledgment,
    Error,
    Disconnect,
    Sync,
    SnapshotRequest,
    SnapshotResponse,
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
            NetworkMessageCode::SnapshotRequest => "snapshot_request",
            NetworkMessageCode::SnapshotResponse => "snapshot_response",
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
            "snapshot_request" => Some(NetworkMessageCode::SnapshotRequest),
            "snapshot_response" => Some(NetworkMessageCode::SnapshotResponse),
            _ => None,
        }
    }
}

// TODO : add message status (failed, success, etc.)

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct Message {
    pub sender_id: String,
    pub sender_addr: std::net::SocketAddr,
    pub clock: crate::clock::Clock,
    pub command: Option<crate::control::Command>,
    pub info: MessageInfo,
    pub code: NetworkMessageCode,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub enum MessageInfo {
    CreateUser(CreateUser),
    Deposit(Deposit),
    Withdraw(Withdraw),
    Transfer(Transfer),
    Pay(Pay),
    Refund(Refund),
    SnapshotResponse(SnapshotResponse),
    None,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct SnapshotResponse {
    pub site_id: String,
    pub clock: crate::clock::Clock,
    pub user_balances: std::collections::HashMap<String, f64>,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct CreateUser {
    pub name: String,
}
impl CreateUser {
    pub fn new(name: String) -> Self {
        Self { name }
    }
}
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct Deposit {
    pub name: String,
    pub amount: f64,
}
impl Deposit {
    pub fn new(name: String, amount: f64) -> Self {
        Self { name, amount }
    }
}
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct Withdraw {
    pub name: String,
    pub amount: f64,
}
impl Withdraw {
    pub fn new(name: String, amount: f64) -> Self {
        Self { name, amount }
    }
}
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct Transfer {
    pub name: String,
    pub beneficiary: String,
    pub amount: f64,
}
impl Transfer {
    pub fn new(name: String, beneficiary: String, amount: f64) -> Self {
        Self {
            name,
            beneficiary,
            amount,
        }
    }
}
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct Pay {
    pub name: String,
    pub amount: f64,
}
impl Pay {
    pub fn new(name: String, amount: f64) -> Self {
        Self { name, amount }
    }
}
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct Refund {
    pub name: String,
    pub transac_time: i64,
    pub transac_node: String,
}
impl Refund {
    #[allow(unused)]
    #[allow(dead_code)]
    pub fn new(name: String, transac_time: i64, transac_node: String) -> Self {
        Self {
            name,
            transac_time,
            transac_node,
        }
    }
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
        let clock = crate::clock::Clock::new();

        let message = Message {
            sender_id: "A".to_string(),
            sender_addr: "127.0.0.1:8080".parse().unwrap(),
            clock: clock,
            command: None,
            info: MessageInfo::None,
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
