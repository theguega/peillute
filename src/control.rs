#[cfg(feature = "server")]
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
    Snapshot,
}

#[cfg(feature = "server")]
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
pub async fn handle_command_from_cli(
    cmd: Command,
    node: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    use crate::state::LOCAL_APP_STATE;
    use log;

    let (local_vc_clock,local_lamport_time)={
        let mut state = LOCAL_APP_STATE.lock().await;
        state.increment_vector_current();
        state.increment_lamport();
        let local_lamport_time = state.get_lamport();
        let local_vc_clock = state.get_vector().clone();
        (local_vc_clock, local_lamport_time)
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
            super::db::deposit(&name, amount, &local_lamport_time, node, &local_vc_clock)?;

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
            if amount < 0.0 {

            }
            super::db::withdraw(&name, amount, &local_lamport_time, node, &local_vc_clock)?;

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
                node,
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
                node,
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
                &transac_node,
                &local_lamport_time,
                node,
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
            println!("/create_user      - Create a personal account");
            println!("/user_accounts    - List all users");
            println!("/print_user_tsx   - Show a user’s transactions");
            println!("/print_tsx        - Show all system transactions");
            println!("/deposit          - Deposit money to an account");
            println!("/withdraw         - Withdraw money from an account");
            println!("/transfer         - Transfer money to another user");
            println!("/pay              - Make a payment (to NULL)");
            println!("/refund           - Refund a transaction");
            println!("/info             - Show system information");
            println!("/start_snapshot   - Start a snapshot");
        }

        Command::Snapshot => {
            super::snapshot::start_snapshot().await?;
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

            println!("ℹ️  Version: {}", env!("CARGO_PKG_VERSION"));
            println!("ℹ️  Authors: {}", env!("CARGO_PKG_AUTHORS"));
            println!("ℹ️  License: MIT");
            println!("ℹ️  Local address: {}", local_addr);
            println!("ℹ️  Site ID: {}", site_id);
            println!("ℹ️  Peers: {:?}", peer_addrs);
            println!("ℹ️  Number of sites on network: {}", nb_sites);
            println!("ℹ️  Lamport clock: {:?}", clock.get_lamport());
            println!("ℹ️  Vector clock: {:?}", clock.get_vector_clock());
        }

        Command::Unknown(cmd) => {
            println!("❓ Unknown command: {}", cmd);
        }

        Command::Error(err) => {
            log::error!("❌ Error: {}", err);
        }
    }
    Ok(())
}

#[cfg(feature = "server")]
pub async fn handle_command_from_network(
    msg: crate::message::MessageInfo,
    node: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    use crate::message::MessageInfo;
    use log;

    let (local_vc_clock,local_lamport_time)={
        let mut state = LOCAL_APP_STATE.lock().await;
        state.increment_vector_current();
        state.increment_lamport();
        let local_lamport_time = state.get_lamport();
        let local_vc_clock = state.get_vector().clone();
        (local_vc_clock, local_lamport_time)
    };

    match msg {
        MessageInfo::CreateUser(create_user) => {
            super::db::create_user(&create_user.name)?;
        }

        MessageInfo::Deposit(deposit) => {
            super::db::deposit(
                &deposit.name,
                deposit.amount,
                &local_lamport_time,
                node,
                &local_vc_clock,
            )?;
        }

        MessageInfo::Withdraw(withdraw) => {
            super::db::withdraw(
                &withdraw.name,
                withdraw.amount,
                &local_lamport_time,
                node,
                &local_vc_clock,
            )?;
        }

        MessageInfo::Transfer(transfer) => {
            super::db::create_transaction(
                &transfer.name,
                &transfer.beneficiary,
                transfer.amount,
                &local_lamport_time,
                node,
                "",
                &local_vc_clock,
            )?;
        }

        MessageInfo::Pay(pay) => {
            super::db::create_transaction(
                &pay.name,
                "NULL",
                pay.amount,
                &local_lamport_time,
                node,
                "",
                &local_vc_clock,
            )?;
        }

        MessageInfo::Refund(refund) => {
            super::db::refund_transaction(
                refund.transac_time,
                &refund.transac_node,
                &local_lamport_time,
                node,
                &local_vc_clock,
            )?;
        }

        MessageInfo::SnapshotResponse(data) => {
            //do nothing
            log::info!("Snapshot response: {:?}", data);
        }

        MessageInfo::None => {
            log::info!("❓ Received None message");
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

#[cfg(feature = "server")]
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
