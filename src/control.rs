//! Command handling and CLI interface
//!
//! This module provides the command-line interface and command handling functionality
//! for the Peillute application, including both local and network command processing.

#[cfg(feature = "server")]
/// Parse a line of input from the CLI and converts it to a Command
pub fn parse_command(line: Result<Option<String>, std::io::Error>) -> Command {
    use log;
    match line {
        Ok(Some(cmd)) => {
            let command = match cmd.trim() {
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
                other => Command::Unknown(other.to_string()),
            };
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
}

#[cfg(feature = "server")]
/// Process commands received from the CLI
/// Update the clock of the site
/// Interact with the database
/// Implement our wave diffusion protocol
pub async fn process_cli_command(cmd: Command) -> Result<(), Box<dyn std::error::Error>> {
    use crate::state::LOCAL_APP_STATE;

    let (clock, site_addr, site_id) = {
        let mut state = LOCAL_APP_STATE.lock().await;
        let local_addr = state.get_site_addr();
        let node = state.get_site_id();
        let _ = state.update_clock(None);
        let clock = state.get_clock();
        (clock, local_addr, node)
    };

    match cmd {
        Command::CreateUser => {
            let name = prompt("Username");
            super::db::create_user(&name)?;

            use crate::message::{CreateUser, Message, MessageInfo, NetworkMessageCode};
            use crate::network::diffuse_message;

            let msg = Message {
                command: Some(Command::CreateUser),
                info: MessageInfo::CreateUser(CreateUser::new(name)),
                code: NetworkMessageCode::Transaction,
                clock: clock,
                sender_addr: site_addr,
                sender_id: site_id.to_string(),
                message_initiator_id: site_id.to_string(),
                message_initiator_addr: site_addr,
            };

            {
                // initialisation des paramètres avant la diffusion d'un message
                let mut state = LOCAL_APP_STATE.lock().await;
                let nb_neigh = state.get_nb_connected_neighbours();
                state.set_parent_addr(site_id.to_string(), site_addr);
                state.set_number_of_attended_neighbors(site_id.to_string(), nb_neigh);
            }

            diffuse_message(&msg).await?;
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
                clock.get_lamport(),
                site_id.as_str(),
                clock.get_vector_clock_map(),
            )?;

            use crate::message::{Deposit, MessageInfo, NetworkMessageCode};

            use crate::message::Message;
            use crate::network::diffuse_message;

            let msg = Message {
                command: Some(Command::Deposit),
                info: MessageInfo::Deposit(Deposit::new(name, amount)),
                code: NetworkMessageCode::Transaction,
                clock: clock,
                sender_addr: site_addr,
                sender_id: site_id.to_string(),
                message_initiator_id: site_id.to_string(),
                message_initiator_addr: site_addr,
            };
            {
                // initialisation des paramètres avant la diffusion d'un message
                let mut state = LOCAL_APP_STATE.lock().await;
                let nb_neigh = state.get_nb_connected_neighbours();
                state.set_parent_addr(site_id.to_string(), site_addr);
                state.set_number_of_attended_neighbors(site_id.to_string(), nb_neigh);
            }

            diffuse_message(&msg).await?;
        }

        Command::Withdraw => {
            let name = prompt("Username");

            let amount = prompt_parse::<f64>("Withdraw amount");
            if amount < 0.0 {}
            super::db::withdraw(
                &name,
                amount,
                clock.get_lamport(),
                site_id.as_str(),
                clock.get_vector_clock_map(),
            )?;

            use crate::message::Message;
            use crate::message::{MessageInfo, NetworkMessageCode, Withdraw};
            use crate::network::diffuse_message;

            let msg = Message {
                command: Some(Command::Withdraw),
                info: MessageInfo::Withdraw(Withdraw::new(name, amount)),
                code: NetworkMessageCode::Transaction,
                clock: clock,
                sender_addr: site_addr,
                sender_id: site_id.to_string(),
                message_initiator_id: site_id.to_string(),
                message_initiator_addr: site_addr,
            };

            {
                // initialisation des paramètres avant la diffusion d'un message
                let mut state = LOCAL_APP_STATE.lock().await;
                let nb_neigh = state.get_nb_connected_neighbours();
                state.set_parent_addr(site_id.to_string(), site_addr);
                state.set_number_of_attended_neighbors(site_id.to_string(), nb_neigh);
            }

            diffuse_message(&msg).await?;
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
                clock.get_lamport(),
                site_id.as_str(),
                "",
                clock.get_vector_clock_map(),
            )?;

            use crate::message::Message;
            use crate::message::{MessageInfo, NetworkMessageCode, Transfer};
            use crate::network::diffuse_message;

            let msg = Message {
                command: Some(Command::Transfer),
                info: MessageInfo::Transfer(Transfer::new(name, beneficiary, amount)),
                code: NetworkMessageCode::Transaction,
                clock: clock,
                sender_addr: site_addr,
                sender_id: site_id.to_string(),
                message_initiator_id: site_id.to_string(),
                message_initiator_addr: site_addr,
            };

            {
                // initialisation des paramètres avant la diffusion d'un message
                let mut state = LOCAL_APP_STATE.lock().await;
                let nb_neigh = state.get_nb_connected_neighbours();
                state.set_parent_addr(site_id.to_string(), site_addr);
                state.set_number_of_attended_neighbors(site_id.to_string(), nb_neigh);
            }

            diffuse_message(&msg).await?;
        }

        Command::Pay => {
            let name = prompt("Username");
            let amount = prompt_parse::<f64>("Payment amount");
            super::db::create_transaction(
                &name,
                "NULL",
                amount,
                clock.get_lamport(),
                site_id.as_str(),
                "",
                clock.get_vector_clock_map(),
            )?;

            use crate::message::Message;
            use crate::message::{MessageInfo, NetworkMessageCode, Pay};
            use crate::network::diffuse_message;

            let msg = Message {
                command: Some(Command::Pay),
                info: MessageInfo::Pay(Pay::new(name, amount)),
                code: NetworkMessageCode::Transaction,
                clock: clock,
                sender_addr: site_addr,
                sender_id: site_id.to_string(),
                message_initiator_id: site_id.to_string(),
                message_initiator_addr: site_addr,
            };

            {
                // initialisation des paramètres avant la diffusion d'un message
                let mut state = LOCAL_APP_STATE.lock().await;
                let nb_neigh = state.get_nb_connected_neighbours();
                state.set_parent_addr(site_id.to_string(), site_addr);
                state.set_number_of_attended_neighbors(site_id.to_string(), nb_neigh);
            }

            diffuse_message(&msg).await?;
        }

        Command::Refund => {
            let name = prompt("Username");
            super::db::print_transaction_for_user(&name).unwrap();

            let transac_time = prompt_parse::<i64>("Lamport time");
            let transac_node = prompt("Node");
            super::db::refund_transaction(
                transac_time,
                &transac_node.as_str(),
                clock.get_lamport(),
                site_id.as_str(),
                clock.get_vector_clock_map(),
            )?;

            use crate::message::Message;
            use crate::message::{MessageInfo, NetworkMessageCode, Refund};
            use crate::network::diffuse_message;

            let msg = Message {
                command: Some(Command::Refund),
                info: MessageInfo::Refund(Refund::new(name, transac_time, transac_node)),
                code: NetworkMessageCode::Transaction,
                clock: clock,
                sender_addr: site_addr,
                sender_id: site_id.to_string(),
                message_initiator_id: site_id.to_string(),
                message_initiator_addr: site_addr,
            };

            {
                // initialisation des paramètres avant la diffusion d'un message
                let mut state = LOCAL_APP_STATE.lock().await;
                let nb_neigh = state.get_nb_connected_neighbours();
                state.set_parent_addr(site_id.to_string(), site_addr);
                state.set_number_of_attended_neighbors(site_addr.to_string(), nb_neigh);
            }

            diffuse_message(&msg).await?;
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
            println!("/help             - Show this help message");
            println!("----------------------------------------");
        }

        Command::Snapshot => {
            println!("📸 Starting snapshot...");
            super::snapshot::start_snapshot().await?;
        }

        Command::Info => {
            let (
                site_addr,
                site_id,
                peer_addrs,
                clock,
                nb_connected_neighbours,
                connected_neighbours_addrs,
                parent_addr_for_transaction_wave,
                attended_neighbours_nb_for_transaction_wave,
            ) = {
                let state = LOCAL_APP_STATE.lock().await;
                (
                    state.get_site_addr(),
                    state.get_site_id().to_string(),
                    state.get_peers_addrs(),
                    state.get_clock(),
                    state.get_nb_connected_neighbours(),
                    state.get_connected_neighbours_addrs(),
                    state.get_parent_addr_for_transaction_wave(),
                    state.get_attended_neighbours_nb_for_transaction_wave(),
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
            println!("Local Address: {}", site_addr);
            println!("Site ID: {}", site_id);
            println!("Number of peers: {}", peer_addrs.len());
            println!("Peers: {:?}", peer_addrs);
            println!("Number of connected neighbors: {}", nb_connected_neighbours);
            println!(
                "Number of connected neighbors: {:?}",
                connected_neighbours_addrs
            );
            println!("Vector Clock: {:?}", clock.get_vector_clock_map());
            println!("Lamport Clock: {}", clock.get_lamport());
            println!("--------- Wave diffusion info ------------");
            println!(
                "Parent addresses for wave (if any): {:?}",
                parent_addr_for_transaction_wave
            );
            println!(
                "Attended neighbours for wave (if any): {:?}",
                attended_neighbours_nb_for_transaction_wave
            );
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
/// Process commands received from the network
/// Update the clock of the site
/// Interact with the database
pub async fn process_network_command(
    msg: crate::message::MessageInfo,
    received_clock: crate::clock::Clock,
    site_id: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    use crate::message::MessageInfo;
    use log;

    let message_lamport_time = received_clock.get_lamport();
    let message_vc_clock = received_clock.get_vector_clock_map();

    if crate::db::transaction_exists(*message_lamport_time, site_id)? {
        log::info!("Transaction allready exists, skipping");
        return Ok(());
    }

    match msg {
        crate::message::MessageInfo::CreateUser(create_user) => {
            super::db::create_user(&create_user.name)?;
        }
        crate::message::MessageInfo::Deposit(deposit) => {
            super::db::deposit(
                &deposit.name,
                deposit.amount,
                &message_lamport_time,
                site_id,
                &message_vc_clock,
            )?;
        }

        MessageInfo::Withdraw(withdraw) => {
            super::db::withdraw(
                &withdraw.name,
                withdraw.amount,
                &message_lamport_time,
                site_id,
                &message_vc_clock,
            )?;
        }

        MessageInfo::Transfer(transfer) => {
            super::db::create_transaction(
                &transfer.name,
                &transfer.beneficiary,
                transfer.amount,
                &message_lamport_time,
                site_id,
                "",
                &message_vc_clock,
            )?;
        }

        MessageInfo::Pay(pay) => {
            super::db::create_transaction(
                &pay.name,
                "NULL",
                pay.amount,
                &message_lamport_time,
                site_id,
                "",
                &message_vc_clock,
            )?;
        }

        MessageInfo::Refund(refund) => {
            super::db::refund_transaction(
                refund.transac_time,
                &refund.transac_node,
                &message_lamport_time,
                site_id,
                &message_vc_clock,
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
