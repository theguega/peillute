//! Command handling and CLI interface
//!
//! This module provides the command-line interface and command handling functionality
//! for the Peillute application, including both local and network command processing.

#[cfg(feature = "server")]
/// Processes a line of input from the CLI and converts it to a Command
pub fn run_cli(line: Result<Option<String>, std::io::Error>) -> Command {
    use log;
    match line {
        Ok(Some(cmd)) => {
            let command = parse_command(&cmd);
            command
        }
        Ok(None) => {
            println!("Aucun input");
            Command::Unknown("Aucun input".to_string())
        }
        Err(e) => {
            log::error!("Erreur de lecture stdin : {}", e);
            Command::Error("Erreur de lecture stdin".to_string())
        }
    }
}

#[cfg(feature = "server")]
/// Available commands in the system
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq)]
pub enum Command {
    /// Create a new user account
    CreateUser,
    /// List all user accounts
    UserAccounts,
    /// Display transactions for a specific user
    PrintUserTransactions,
    /// Display all system transactions
    PrintTransactions,
    /// Deposit money into an account
    Deposit,
    /// Withdraw money from an account
    Withdraw,
    /// Transfer money between accounts
    Transfer,
    /// Make a payment
    Pay,
    /// Process a refund
    Refund,
    /// Display help information
    Help,
    /// Display system information
    Info,
    /// Unknown command
    Unknown(String),
    /// Error command
    Error(String),
    /// Start a system snapshot
    Snapshot,
    /// Wave command
    Wave,
}

#[cfg(feature = "server")]
/// Parses a command string into a Command enum variant
fn parse_command(input: &str) -> Command {
    match input.trim() {
        "/create_user" => Command::CreateUser,
        "/user_accounts" => Command::UserAccounts,
        "/print_user_tsx" => Command::PrintUserTransactions,
        "/print_tsx" => Command::PrintTransactions,
        "/deposit" => Command::Deposit,
        "/withdraw" => Command::Withdraw,
        "/transfer" => Command::Transfer,
        "/pay" => Command::Pay,
        "/refund" => Command::Refund,
        "/help" => Command::Help,
        "/info" => Command::Info,
        "/start_snapshot" => Command::Snapshot,
        "/wave" => Command::Wave,
        other => Command::Unknown(other.to_string()),
    }
}

#[cfg(feature = "server")]
/// Handles commands received from the CLI
pub async fn handle_command_from_cli(cmd: Command) -> Result<(), Box<dyn std::error::Error>> {
    use crate::state::LOCAL_APP_STATE;

    let (local_vc_clock, local_lamport_time, node) = {
        let mut state = LOCAL_APP_STATE.lock().await;
        state.increment_vector_current();
        state.increment_lamport();
        let local_lamport_time = state.get_lamport();
        let local_vc_clock = state.get_vector().clone();
        let node = state.get_site_id().to_string();
        (local_vc_clock, local_lamport_time, node)
    };

    match cmd {
        Command::CreateUser => {
            let name = prompt("Username");
            super::db::create_user(&name)?;

            use crate::message::{CreateUser, MessageInfo, NetworkMessageCode};
            use crate::network::send_message_to_all;

            let _ = send_message_to_all(
                Some(Command::CreateUser),
                NetworkMessageCode::Transaction,
                MessageInfo::CreateUser(CreateUser::new(name.clone())),
            )
            .await?;
        }

        Command::UserAccounts => {
            super::db::print_users()?;
        }

        Command::PrintUserTransactions => {
            let name = prompt("Username");
            super::db::print_transaction_for_user(&name)?;
        }

        Command::PrintTransactions => {
            super::db::print_transactions()?;
        }

        Command::Deposit => {
            let name = prompt("Username");

            let amount = prompt_parse::<f64>("Deposit amount");
            super::db::deposit(
                &name,
                amount,
                &local_lamport_time,
                node.as_str(),
                &local_vc_clock,
            )?;

            use crate::message::{Deposit, MessageInfo, NetworkMessageCode};
            use crate::network::send_message_to_all;

            let _ = send_message_to_all(
                Some(Command::Deposit),
                NetworkMessageCode::Transaction,
                MessageInfo::Deposit(Deposit::new(name.clone(), amount)),
            )
            .await?;
        }

        Command::Withdraw => {
            let name = prompt("Username");

            let amount = prompt_parse::<f64>("Withdraw amount");
            if amount < 0.0 {}
            super::db::withdraw(
                &name,
                amount,
                &local_lamport_time,
                node.as_str(),
                &local_vc_clock,
            )?;

            use crate::message::{MessageInfo, NetworkMessageCode, Withdraw};
            use crate::network::send_message_to_all;

            let _ = send_message_to_all(
                Some(Command::Withdraw),
                NetworkMessageCode::Transaction,
                MessageInfo::Withdraw(Withdraw::new(name.clone(), amount)),
            )
            .await?;
        }

        Command::Transfer => {
            let name = prompt("Username");

            let amount = prompt_parse::<f64>("Transfer amount");
            let _ = super::db::print_users();
            let beneficiary = prompt("Beneficiary");

            super::db::create_transaction(
                &name,
                &beneficiary,
                amount,
                &local_lamport_time,
                node.as_str(),
                "",
                &local_vc_clock,
            )?;

            use crate::message::{MessageInfo, NetworkMessageCode, Transfer};
            use crate::network::send_message_to_all;

            let _ = send_message_to_all(
                Some(Command::Transfer),
                NetworkMessageCode::Transaction,
                MessageInfo::Transfer(Transfer::new(name.clone(), beneficiary.clone(), amount)),
            )
            .await?;
        }

        Command::Pay => {
            let name = prompt("Username");
            let amount = prompt_parse::<f64>("Payment amount");
            super::db::create_transaction(
                &name,
                "NULL",
                amount,
                &local_lamport_time,
                node.as_str(),
                "",
                &local_vc_clock,
            )?;

            use crate::message::{MessageInfo, NetworkMessageCode, Pay};
            use crate::network::send_message_to_all;

            let _ = send_message_to_all(
                Some(Command::Pay),
                NetworkMessageCode::Transaction,
                MessageInfo::Pay(Pay::new(name.clone(), amount)),
            )
            .await?;
        }

        Command::Refund => {
            let name = prompt("Username");
            super::db::print_transaction_for_user(&name).unwrap();

            let transac_time = prompt_parse::<i64>("Lamport time");
            let transac_node = prompt("Node");
            super::db::refund_transaction(
                transac_time,
                &transac_node.as_str(),
                &local_lamport_time,
                node.as_str(),
                &local_vc_clock,
            )?;

            use crate::message::{MessageInfo, NetworkMessageCode, Refund};
            use crate::network::send_message_to_all;

            let _ = send_message_to_all(
                Some(Command::Refund),
                NetworkMessageCode::Transaction,
                MessageInfo::Refund(Refund::new(name, transac_time, transac_node)),
            )
            .await?;
        }

        Command::Help => {
            println!("📜 Command list:");
            println!("----------------------------------------");
            println!("/create_user      - Create a personal account");
            println!("/user_accounts    - List all users");
            println!("/print_user_tsx   - Show a user's transactions");
            println!("/print_tsx        - Show all system transactions");
            println!("/deposit          - Deposit money to an account");
            println!("/withdraw         - Withdraw money from an account");
            println!("/transfer         - Transfer money to another user");
            println!("/pay              - Make a payment (to NULL)");
            println!("/refund           - Refund a transaction");
            println!("/info             - Show system information");
            println!("/start_snapshot   - Start a snapshot");
            println!("/wave             - Launch a Half-Wave broadcast");
            println!("/help             - Show this help message");
            println!("----------------------------------------");
        }

        Command::Snapshot => {
            println!("📸 Starting snapshot...");
            super::snapshot::start_snapshot().await?;
        }

        Command::Wave => {
            println!("Launching Half-Wave Broadcast...");
            crate::network::start_half_wave_broadcast().await;
        }
        

        Command::Info => {
            let (local_addr, site_id, peer_addrs, clock, nb_sites) = {
                let state = LOCAL_APP_STATE.lock().await;
                (
                    state.get_local_addr(),
                    state.get_site_id().to_string(),
                    state.get_peers_string(),
                    state.get_clock().clone(),
                    state.nb_sites_on_network,
                )
            };

            let db_path = {
                let conn = crate::db::DB_CONN.lock().unwrap();
                let path = conn.path().unwrap();
                // keep only the name of the file (after the last "/")
                path.to_string().split("/").last().unwrap().to_string()
            };

            println!("📊 System Information:");
            println!("----------------------------------------");
            println!("Database : {}", db_path);
            println!("Local Address: {}", local_addr);
            println!("Site ID: {}", site_id);
            println!("Number of Sites: {}", nb_sites);
            println!("Peers: {:?}", peer_addrs);
            println!("Vector Clock: {:?}", clock.get_vector());
            println!("Lamport Clock: {}", clock.get_lamport());
            println!("----------------------------------------");
        }

        Command::Unknown(msg) => {
            println!("❌ Unknown command: {}", msg);
        }

        Command::Error(msg) => {
            println!("❌ Error: {}", msg);
        }
    }

    Ok(())
}

#[cfg(feature = "server")]
/// Handles commands received from the network
pub async fn handle_command_from_network(
    msg: crate::message::MessageInfo,
    clock : crate::clock::Clock,
    site : String,
) -> Result<(), Box<dyn std::error::Error>> {
    match msg {
        crate::message::MessageInfo::HalfWave { .. } => {
            // Handle HalfWave message
        }
        crate::message::MessageInfo::HalfWaveAck { .. } => {
            // Handle HalfWaveAck message
        }
        crate::message::MessageInfo::CreateUser(create_user) => {
            super::db::create_user(&create_user.name)?;
        }
        crate::message::MessageInfo::Deposit(deposit) => {
 
            let lamport_time = clock.get_lamport();
            let vc_clock = clock.get_vector();
            super::db::deposit(
                &deposit.name,
                deposit.amount,
                &lamport_time,
                site.as_str(),
                &vc_clock,
            )?;
        }
        crate::message::MessageInfo::Withdraw(withdraw) => {
            let lamport_time = clock.get_lamport();
            let vc_clock = clock.get_vector();

            super::db::withdraw(
                &withdraw.name,
                withdraw.amount,
                &lamport_time,
                site.as_str(),
                &vc_clock,
            )?;
        }
        crate::message::MessageInfo::Transfer(transfer) => {
            let lamport_time = clock.get_lamport();
            let vc_clock = clock.get_vector();

            super::db::create_transaction(
                &transfer.name,
                &transfer.beneficiary,
                transfer.amount,
                &lamport_time,
                site.as_str(),
                "",
                &vc_clock,
            )?;
        }
        crate::message::MessageInfo::Pay(pay) => {
            let lamport_time = clock.get_lamport();
            let vc_clock = clock.get_vector();

            super::db::create_transaction(
                &pay.name,
                "NULL",
                pay.amount,
                &lamport_time,
                site.as_str(),
                "",
                &vc_clock,
            )?;
        }
        crate::message::MessageInfo::Refund(refund) => {
            let lamport_time = clock.get_lamport();
            let vc_clock = clock.get_vector();

            super::db::refund_transaction(
                refund.transac_time,
                &refund.transac_node.as_str(),
                &lamport_time,
                site.as_str(),
                &vc_clock,
            )?;
        }
        crate::message::MessageInfo::SnapshotResponse(_) => {
            // Handle snapshot response
        }
        crate::message::MessageInfo::None => {
            // No action needed
        }
    }

    Ok(())
}

#[cfg(feature = "server")]
/// Prompts the user for input with a label
fn prompt(label: &str) -> String {
    use std::io::{self, Write};
    print!("{}: ", label);
    io::stdout().flush().unwrap();
    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();
    input.trim().to_string()
}

#[cfg(feature = "server")]
/// Prompts the user for input and parses it to a specific type
fn prompt_parse<T: std::str::FromStr>(label: &str) -> T
where
    T::Err: std::fmt::Debug,
{
    use std::io::{self, Write};
    loop {
        print!("{}: ", label);
        io::stdout().flush().unwrap();
        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();
        match input.trim().parse() {
            Ok(value) => return value,
            Err(e) => println!("Invalid input: {:?}", e),
        }
    }
}
