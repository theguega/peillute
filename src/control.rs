pub fn run_cli(line: Result<Option<String>, std::io::Error>) -> Command {
    use log;
    match line {
        Ok(Some(cmd)) => {
            let command = parse_command(&cmd);
            command
        }
        Ok(None) => {
            log::info!("Aucun input");
            Command::Unknown("Aucun input".to_string())
        }
        Err(e) => {
            log::error!("Erreur de lecture stdin : {}", e);
            Command::Error("Erreur de lecture stdin".to_string())
        }
    }
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq)]
pub enum Command {
    CreateUser,
    UserAccounts,
    PrintUserTransactions,
    PrintTransactions,
    Deposit,
    Withdraw,
    Transfer,
    Pay,
    Refund,
    Help,
    Info,
    Unknown(String),
    Error(String),
}

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
        other => Command::Unknown(other.to_string()),
    }
}

pub async fn handle_command(
    cmd: Command,
    lamport_time: &mut i64,
    node: &str,
    from_network: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    use crate::state::LOCAL_APP_STATE;
    use log;

    match cmd {
        Command::CreateUser => {
            let name = prompt("Username");
            super::db::create_user(&name).unwrap();

            if !from_network {
                use crate::message::{CreateUser, MessageInfo, NetworkMessageCode};
                use crate::network::send_message_to_all;

                let _ = send_message_to_all(
                    Some(Command::CreateUser),
                    NetworkMessageCode::Transaction,
                    MessageInfo::CreateUser(CreateUser::new(name.clone())),
                )
                .await?;
            }
        }

        Command::UserAccounts => {
            super::db::print_users().unwrap();
        }

        Command::PrintUserTransactions => {
            let name = prompt("Username");
            super::db::print_transaction_for_user(&name).unwrap();
        }

        Command::PrintTransactions => {
            super::db::print_transactions().unwrap();
        }

        Command::Deposit => {
            let name = prompt("Username");
            let amount = prompt_parse::<f64>("Deposit amount");
            super::db::deposit(&name, amount, lamport_time, node).unwrap();

            if !from_network {
                use crate::message::{Deposit, MessageInfo, NetworkMessageCode};
                use crate::network::send_message_to_all;

                let _ = send_message_to_all(
                    Some(Command::Deposit),
                    NetworkMessageCode::Transaction,
                    MessageInfo::Deposit(Deposit::new(name.clone(), amount)),
                )
                .await?;
            }
        }

        Command::Withdraw => {
            let name = prompt("Username");
            let amount = prompt_parse::<f64>("Withdraw amount");
            super::db::withdraw(&name, amount, lamport_time, node).unwrap();

            if !from_network {
                use crate::message::{MessageInfo, NetworkMessageCode, Withdraw};
                use crate::network::send_message_to_all;

                let _ = send_message_to_all(
                    Some(Command::Withdraw),
                    NetworkMessageCode::Transaction,
                    MessageInfo::Withdraw(Withdraw::new(name.clone(), amount)),
                )
                .await?;
            }
        }

        Command::Transfer => {
            let name = prompt("Username");
            let amount = prompt_parse::<f64>("Transfer amount");
            let _ = super::db::print_users();
            let beneficiary = prompt("Beneficiary");

            super::db::create_transaction(&name, &beneficiary, amount, lamport_time, node, "")
                .unwrap();

            if !from_network {
                use crate::message::{MessageInfo, NetworkMessageCode, Transfer};
                use crate::network::send_message_to_all;

                let _ = send_message_to_all(
                    Some(Command::Transfer),
                    NetworkMessageCode::Transaction,
                    MessageInfo::Transfer(Transfer::new(name.clone(), beneficiary.clone(), amount)),
                )
                .await?;
            }
        }

        Command::Pay => {
            let name = prompt("Username");
            let amount = prompt_parse::<f64>("Payment amount");
            super::db::create_transaction(&name, "NULL", amount, lamport_time, node, "").unwrap();

            if !from_network {
                use crate::message::{MessageInfo, NetworkMessageCode, Pay};
                use crate::network::send_message_to_all;

                let _ = send_message_to_all(
                    Some(Command::Pay),
                    NetworkMessageCode::Transaction,
                    MessageInfo::Pay(Pay::new(name.clone(), amount)),
                )
                .await?;
            }
        }

        Command::Refund => {
            let name = prompt("Username");
            super::db::print_transaction_for_user(&name).unwrap();

            let transac_time = prompt_parse::<i64>("Lamport time");
            let transac_node = prompt("Node");
            super::db::refund_transaction(transac_time, &transac_node, lamport_time, node).unwrap();

            if !from_network {
                // TODO : send message
            }
        }

        Command::Help => {
            log::info!("📜 Command list:");
            log::info!("/create_user      - Create a personal account");
            log::info!("/user_accounts    - List all users");
            log::info!("/print_user_tsx   - Show a user’s transactions");
            log::info!("/print_tsx        - Show all system transactions");
            log::info!("/deposit          - Deposit money to an account");
            log::info!("/withdraw         - Withdraw money from an account");
            log::info!("/transfer         - Transfer money to another user");
            log::info!("/pay              - Make a payment (to NULL)");
            log::info!("/refund           - Refund a transaction");
            log::info!("/info             - Show system information");
        }

        Command::Info => {
            let (local_addr, site_id, peer_addrs, clock, nb_sites) = {
                let state = LOCAL_APP_STATE.lock().await;
                (
                    state.get_local_addr().to_string(),
                    state.get_site_id().to_string(),
                    state.get_peers(),
                    state.get_clock().clone(),
                    state.nb_sites_on_network,
                )
            };

            log::info!("ℹ️  Info: This is a distributed banking system.");
            log::info!("ℹ️  Version: 0.0.1");
            log::info!(
                "ℹ️  Authors: Aubin Vert, Théo Guegan, Alexandre Eberhardt, Léopold Chappuis"
            );
            log::info!("ℹ️  License: MIT");
            log::info!("ℹ️  Local address: {}", local_addr);
            log::info!("ℹ️  Site ID: {}", site_id);
            log::info!("ℹ️  Peers: {:?}", peer_addrs);
            log::info!("ℹ️  Number of sites on network: {}", nb_sites);
            log::info!("ℹ️  Lamport clock: {:?}", clock.get_lamport());
            log::info!("ℹ️  Vector clock: {:?}", clock.get_vector_clock());
        }

        Command::Unknown(cmd) => {
            log::info!("❓ Unknown command: {}", cmd);
        }

        Command::Error(err) => {
            log::error!("❌ Error: {}", err);
        }
    }
    Ok(())
}

fn prompt(label: &str) -> String {
    use std::io::{self, Write};
    let mut input = String::new();
    print!("{label} > ");
    io::stdout().flush().unwrap();
    io::stdin().read_line(&mut input).unwrap();
    input.trim().to_string()
}

fn prompt_parse<T: std::str::FromStr>(label: &str) -> T
where
    T::Err: std::fmt::Debug,
{
    loop {
        let input = prompt(label);
        match input.parse::<T>() {
            Ok(value) => break value,
            Err(_) => println!("Invalid input. Try again."),
        }
    }
}
