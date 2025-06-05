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
        other => Command::Unknown(other.to_string()),
    }
}

#[cfg(feature = "server")]
/// Handles commands received from the CLI
pub async fn handle_command_from_cli(cmd: Command) -> Result<(), Box<dyn std::error::Error>> {
    use crate::state::LOCAL_APP_STATE;

    let (local_vc_clock, local_lamport_time, local_clk, local_addr, node) = {
        let mut state = LOCAL_APP_STATE.lock().await;
        state.increment_vector_current();
        state.increment_lamport();
        let local_lamport_time = state.get_lamport();
        let local_vc_clock = state.get_vector().clone();
        let local_clk = state.get_clock().clone();
        let local_addr = state.get_site_addr().clone();
        let node = state.get_site_id().to_string();
        (
            local_vc_clock,
            local_lamport_time,
            local_clk,
            local_addr,
            node,
        )
    };

    match cmd {
        Command::CreateUser => {
            let name = prompt("Username");
            super::db::create_user(&name)?;

            use crate::message::{CreateUser, Message, MessageInfo, NetworkMessageCode};
            use crate::network::diffuse_message;

            let msg = Message {
                command: Some(Command::CreateUser),
                info: MessageInfo::CreateUser(CreateUser::new(name.clone())),
                code: NetworkMessageCode::Transaction,
                clock: local_clk.clone(),
                sender_addr: local_addr.parse().unwrap(),
                sender_id: node.to_string(),
                message_initiator_id: node.to_string(),
                message_initiator_addr: local_addr.parse().unwrap(),
            };

            {
                // initialisation des paramètres avant la diffusion d'un message
                let mut state = LOCAL_APP_STATE.lock().await;
                let nb_neigh = state.nb_connected_neighbours;
                state.set_parent_addr(node.to_string(), local_addr.parse().unwrap());
                state.set_number_of_attended_neighbors(node.to_string(), nb_neigh);
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
                &local_lamport_time,
                node.as_str(),
                &local_vc_clock,
            )?;

            use crate::message::{Deposit, MessageInfo, NetworkMessageCode};

            use crate::message::Message;
            use crate::network::diffuse_message;

            let msg = Message {
                command: Some(Command::Deposit),
                info: MessageInfo::Deposit(Deposit::new(name.clone(), amount)),
                code: NetworkMessageCode::Transaction,
                clock: local_clk.clone(),
                sender_addr: local_addr.parse().unwrap(),
                sender_id: node.to_string(),
                message_initiator_id: node.to_string(),
                message_initiator_addr: local_addr.parse().unwrap(),
            };
            {
                // initialisation des paramètres avant la diffusion d'un message
                let mut state = LOCAL_APP_STATE.lock().await;
                let nb_neigh = state.nb_connected_neighbours;
                state.set_parent_addr(node.to_string(), local_addr.parse().unwrap());
                state.set_number_of_attended_neighbors(node.to_string(), nb_neigh);
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
                &local_lamport_time,
                node.as_str(),
                &local_vc_clock,
            )?;

            use crate::message::Message;
            use crate::message::{MessageInfo, NetworkMessageCode, Withdraw};
            use crate::network::diffuse_message;

            let msg = Message {
                command: Some(Command::Withdraw),
                info: MessageInfo::Withdraw(Withdraw::new(name.clone(), amount)),
                code: NetworkMessageCode::Transaction,
                clock: local_clk,
                sender_addr: local_addr.parse().unwrap(),
                sender_id: node.to_string(),
                message_initiator_id: node.to_string(),
                message_initiator_addr: local_addr.parse().unwrap(),
            };

            {
                // initialisation des paramètres avant la diffusion d'un message
                let mut state = LOCAL_APP_STATE.lock().await;
                let nb_neigh = state.nb_connected_neighbours;
                state.set_parent_addr(node.to_string(), local_addr.parse().unwrap());
                state.set_number_of_attended_neighbors(node.to_string(), nb_neigh);
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
                &local_lamport_time,
                node.as_str(),
                "",
                &local_vc_clock,
            )?;

            use crate::message::Message;
            use crate::message::{MessageInfo, NetworkMessageCode, Transfer};
            use crate::network::diffuse_message;

            let msg = Message {
                command: Some(Command::Transfer),
                info: MessageInfo::Transfer(Transfer::new(
                    name.clone(),
                    beneficiary.clone(),
                    amount,
                )),
                code: NetworkMessageCode::Transaction,
                clock: local_clk,
                sender_addr: local_addr.parse().unwrap(),
                sender_id: node.to_string(),
                message_initiator_id: node.to_string(),
                message_initiator_addr: local_addr.parse().unwrap(),
            };

            {
                // initialisation des paramètres avant la diffusion d'un message
                let mut state = LOCAL_APP_STATE.lock().await;
                let nb_neigh = state.nb_connected_neighbours;
                state.set_parent_addr(node.to_string(), local_addr.parse().unwrap());
                state.set_number_of_attended_neighbors(node.to_string(), nb_neigh);
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
                &local_lamport_time,
                node.as_str(),
                "",
                &local_vc_clock,
            )?;

            use crate::message::Message;
            use crate::message::{MessageInfo, NetworkMessageCode, Pay};
            use crate::network::diffuse_message;

            let msg = Message {
                command: Some(Command::Pay),
                info: MessageInfo::Pay(Pay::new(name.clone(), amount)),
                code: NetworkMessageCode::Transaction,
                clock: local_clk,
                sender_addr: local_addr.parse().unwrap(),
                sender_id: node.parse().unwrap(),
                message_initiator_id: node.to_string(),
                message_initiator_addr: local_addr.parse().unwrap(),
            };

            {
                // initialisation des paramètres avant la diffusion d'un message
                let mut state = LOCAL_APP_STATE.lock().await;
                let nb_neigh = state.nb_connected_neighbours;
                state.set_parent_addr(node.to_string(), local_addr.parse().unwrap());
                state.set_number_of_attended_neighbors(node.to_string(), nb_neigh);
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
                &local_lamport_time,
                node.as_str(),
                &local_vc_clock,
            )?;

            use crate::message::Message;
            use crate::message::{MessageInfo, NetworkMessageCode, Refund};
            use crate::network::diffuse_message;

            let msg = Message {
                command: Some(Command::Refund),
                info: MessageInfo::Refund(Refund::new(name, transac_time, transac_node)),
                code: NetworkMessageCode::Transaction,
                clock: local_clk,
                sender_addr: local_addr.parse().unwrap(),
                sender_id: node.parse().unwrap(),
                message_initiator_id: node.to_string(),
                message_initiator_addr: local_addr.parse().unwrap(),
            };

            {
                // initialisation des paramètres avant la diffusion d'un message
                let mut state = LOCAL_APP_STATE.lock().await;
                let nb_neigh = state.nb_connected_neighbours;
                state.set_parent_addr(node.to_string(), local_addr.parse().unwrap());
                state.set_number_of_attended_neighbors(node.to_string(), nb_neigh);
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
                local_addr,
                site_id,
                peer_addrs,
                clock,
                nb_neighbours,
                in_use_neighbours,
                parent_addr,
                nb_of_in_use_neig,
            ) = {
                let state = LOCAL_APP_STATE.lock().await;
                (
                    state.get_site_addr(),
                    state.get_site_id().to_string(),
                    state.get_peers_addrs_string(),
                    state.get_clock().clone(),
                    state.nb_connected_neighbours,
                    state.connected_neighbours_addrs.clone(),
                    state.parent_addr_for_transaction_wave.clone(),
                    state.attended_neighbours_nb_for_transaction_wave.clone(),
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
            println!("Number of connected neighbors: {}", nb_neighbours);
            println!("Peers: {:?}", peer_addrs);
            println!("Vector Clock: {:?}", clock.get_vector());
            println!("Lamport Clock: {}", clock.get_lamport());
            println!("----------------------------------------");
            log::info!("ℹ️  Info: This is a distributed banking system.");
            log::info!("ℹ️  Version: 0.0.1");
            log::info!(
                "ℹ️  Authors: Aubin Vert, Théo Guegan, Alexandre Eberhardt, Léopold Chappuis"
            );
            log::info!("ℹ️  License: MIT");
            log::info!("ℹ️  Local address: {}", local_addr);
            log::info!("ℹ️  Site ID: {}", site_id);
            log::info!("ℹ️  Peers: {:?}", peer_addrs);
            log::info!("ℹ️  Number of in use neighbours: {}", nb_neighbours);

            let mut msg: String = " ".to_string();
            for neig in in_use_neighbours {
                msg += neig.to_string().as_str();
                msg += "  ";
            }
            log::info!("ℹ️  In use neighbours: {}", msg);

            log::info!("ℹ️  Wave diffusion parameters\n");

            log::info!("ℹ️  Parent addresses\n");
            let mut msg_parent: String = " ".to_string();
            for (key, addr) in parent_addr {
                msg_parent += key.as_str();
                msg_parent += " ";
                msg_parent += addr.to_string().as_str();
                msg_parent += " \n";
            }

            log::info!("{}", msg_parent);

            log::info!("ℹ️  Nb_of_attended_neighbours\n");
            let mut msg_nb_a_i: String = " ".to_string();
            for (init_id, nb) in nb_of_in_use_neig {
                msg_nb_a_i += init_id.as_str();
                msg_nb_a_i += " ";
                msg_nb_a_i += nb.to_string().as_str();
                msg_nb_a_i += " \n";
            }

            log::info!("{}", msg_nb_a_i);

            log::info!("ℹ️  Lamport clock: {:?}", clock.get_lamport());
            log::info!("ℹ️  Vector clock: {:?}", clock.get_vector());
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
    received_clock: crate::clock::Clock,
    site_id: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    use crate::state::LOCAL_APP_STATE;
    // at reception, if clock of the site id is greater than our receiver clock, we have to prodcast the message
    // or if the vector clock for this site id is not present, we have to broadcast the message
    // else, this message have already been received, we can ignore it
    let local_clocks = {
        let state = LOCAL_APP_STATE.lock().await;
        state.get_clock().clone()
    };
    if let Some(local_version_of_received_clock) =
        local_clocks.get_vector().get(&site_id.to_string())
    {
        log::debug! {"local version of received clock: {local_version_of_received_clock}"};
        log::debug! {"received clock: {}", received_clock.get_lamport()};
        if local_version_of_received_clock >= &received_clock.get_lamport() {
            log::debug!("command already received, skipping");
            return Ok(());
        };
    }

    log::debug!("handle and broadcast to other nodes");
    use crate::message::MessageInfo;
    use log;

    let message_lamport_time = received_clock.get_lamport().clone();
    let message_vc_clock = received_clock.get_vector().clone();

    if crate::db::transaction_exists(message_lamport_time, site_id)? {
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
