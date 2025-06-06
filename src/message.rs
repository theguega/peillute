//! Message handling for distributed transactions
//!
//! This module defines the message types and structures used for communication
//! between nodes in the distributed system, including transaction messages,
//! network control messages, and various financial operations.

#[cfg(feature = "server")]
/// Represents a financial transaction in the system
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq)]
pub struct Transaction {
    /// Unique identifier for the transaction
    pub id: u64,
    /// ID of the user performing the transaction
    pub user_id: String,
    /// Transaction amount
    pub amount: f64,
    /// Description of the transaction
    pub description: String,
}

#[cfg(feature = "server")]
/// Types of network messages used for communication between nodes
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq)]
pub enum NetworkMessageCode {
    /// Message for discovering other nodes in the network
    Discovery,
    /// Message containing a financial transaction
    Transaction,
    /// Message acknowledging receipt of a previous transaction
    TransactionAcknowledgement,
    /// Message acknowledging receipt of a previous message
    Acknowledgment,
    /// Message indicating an error condition
    Error,
    /// Message for gracefully disconnecting from the network
    Disconnect,
    /// Message for synchronizing state between nodes
    Sync,
    /// Message requesting a state snapshot
    SnapshotRequest,
    /// Message containing a state snapshot
    SnapshotResponse,

    /// Request to acquire the global mutex
    AcquireMutex,
    // / Request to release the global mutex
    ReleaseGlobalMutex,
    /// Acknowledgment of the global mutex acquisition
    AckGlobalMutex,
}

#[cfg(feature = "server")]
impl NetworkMessageCode {
    #[allow(unused)]
    /// Returns the string representation of the message code
    pub fn code(&self) -> &'static str {
        match self {
            NetworkMessageCode::Discovery => "discovery",
            NetworkMessageCode::Transaction => "transaction",
            NetworkMessageCode::TransactionAcknowledgement => "transaction_acknowledgement",
            NetworkMessageCode::Acknowledgment => "acknowledgment",
            NetworkMessageCode::Error => "error",
            NetworkMessageCode::Disconnect => "disconnect",
            NetworkMessageCode::Sync => "sync",
            NetworkMessageCode::SnapshotRequest => "snapshot_request",
            NetworkMessageCode::SnapshotResponse => "snapshot_response",
            NetworkMessageCode::AcquireMutex => "acquire_mutex",
            NetworkMessageCode::ReleaseGlobalMutex => "release_global_mutex",
            NetworkMessageCode::AckGlobalMutex => "ack_global_mutex",
        }
    }

    #[allow(unused)]
    /// Converts a string code to a NetworkMessageCode variant
    pub fn from_code(code: &str) -> Option<Self> {
        match code {
            "discovery" => Some(NetworkMessageCode::Discovery),
            "transaction" => Some(NetworkMessageCode::Transaction),
            "transaction_acknowledgement" => Some(NetworkMessageCode::TransactionAcknowledgement),
            "acknowledgment" => Some(NetworkMessageCode::Acknowledgment),
            "error" => Some(NetworkMessageCode::Error),
            "disconnect" => Some(NetworkMessageCode::Disconnect),
            "sync" => Some(NetworkMessageCode::Sync),
            "snapshot_request" => Some(NetworkMessageCode::SnapshotRequest),
            "snapshot_response" => Some(NetworkMessageCode::SnapshotResponse),
            "acquire_mutex" => Some(NetworkMessageCode::AcquireMutex),
            "release_global_mutex" => Some(NetworkMessageCode::ReleaseGlobalMutex),
            "ack_global_mutex" => Some(NetworkMessageCode::AckGlobalMutex),
            _ => None,
        }
    }
}

#[cfg(feature = "server")]
/// Represents a message exchanged between nodes in the network
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct Message {
    /// ID of the sending node
    pub sender_id: String,
    /// ID of the node that initiated the message
    pub message_initiator_id: String,
    /// Network address of the node that initiated the message
    pub message_initiator_addr: std::net::SocketAddr,
    /// Network address of the sending node
    pub sender_addr: std::net::SocketAddr,
    /// Logical clock state of the sending node
    pub clock: crate::clock::Clock,
    /// Optional command to be executed
    pub command: Option<crate::control::Command>,
    /// Message payload containing the actual data
    pub info: MessageInfo,
    /// Type of the message
    pub code: NetworkMessageCode,
}

#[cfg(feature = "server")]
/// Types of message payloads for different operations
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub enum MessageInfo {
    /// Create a new user
    CreateUser(CreateUser),
    /// Deposit money into an account
    Deposit(Deposit),
    /// Withdraw money from an account
    Withdraw(Withdraw),
    /// Transfer money between accounts
    Transfer(Transfer),
    /// Make a payment
    Pay(Pay),
    /// Process a refund
    Refund(Refund),
    /// Response to a snapshot request
    SnapshotResponse(SnapshotResponse),
    AcquireMutex(AcquireMutexPayload),
    ReleaseMutex(ReleaseMutexPayload),
    AckMutex(AckMutexPayload),
    /// No payload
    None,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct AcquireMutexPayload;
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct ReleaseMutexPayload;
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct AckMutexPayload {
    pub clock: i64,
}

#[cfg(feature = "server")]
/// Response to a state snapshot request
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct SnapshotResponse {
    /// ID of the responding node
    pub site_id: String,
    /// Logical clock state of the responding node
    pub clock: crate::clock::Clock,
    /// Transaction log summary
    pub tx_log: Vec<crate::snapshot::TxSummary>,
}

/// Request to create a new user
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct CreateUser {
    /// Name of the user to create
    pub name: String,
}

#[cfg(feature = "server")]
impl CreateUser {
    /// Creates a new CreateUser request
    pub fn new(name: String) -> Self {
        Self { name }
    }
}

#[cfg(feature = "server")]
/// Request to deposit money into an account
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct Deposit {
    /// Name of the account
    pub name: String,
    /// Amount to deposit
    pub amount: f64,
}

#[cfg(feature = "server")]
impl Deposit {
    /// Creates a new Deposit request
    pub fn new(name: String, amount: f64) -> Self {
        Self { name, amount }
    }
}

#[cfg(feature = "server")]
/// Request to withdraw money from an account
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct Withdraw {
    /// Name of the account
    pub name: String,
    /// Amount to withdraw
    pub amount: f64,
}

#[cfg(feature = "server")]
impl Withdraw {
    /// Creates a new Withdraw request
    pub fn new(name: String, amount: f64) -> Self {
        Self { name, amount }
    }
}

#[cfg(feature = "server")]
/// Request to transfer money between accounts
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct Transfer {
    /// Name of the source account
    pub name: String,
    /// Name of the destination account
    pub beneficiary: String,
    /// Amount to transfer
    pub amount: f64,
}

#[cfg(feature = "server")]
impl Transfer {
    /// Creates a new Transfer request
    pub fn new(name: String, beneficiary: String, amount: f64) -> Self {
        Self {
            name,
            beneficiary,
            amount,
        }
    }
}

#[cfg(feature = "server")]
/// Request to make a payment
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct Pay {
    /// Name of the account
    pub name: String,
    /// Amount to pay
    pub amount: f64,
}

#[cfg(feature = "server")]
impl Pay {
    /// Creates a new Pay request
    pub fn new(name: String, amount: f64) -> Self {
        Self { name, amount }
    }
}

#[cfg(feature = "server")]
/// Request to process a refund
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct Refund {
    /// Name of the account
    pub name: String,
    /// Timestamp of the original transaction
    pub transac_time: i64,
    /// ID of the node that processed the original transaction
    pub transac_node: String,
}

#[cfg(feature = "server")]
impl Refund {
    /// Creates a new Refund request
    pub fn new(name: String, transac_time: i64, transac_node: String) -> Self {
        Self {
            name,
            transac_time,
            transac_node,
        }
    }
}

#[cfg(test)]
#[cfg(feature = "server")]
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
            message_initiator_id: "A".to_string(),
            message_initiator_addr: "127.0.0.1:8080".parse().unwrap(),
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
