use super::db;
use rusqlite::Connection;
use std::io::{self as std_io, Write};

pub fn run_cli(line: Result<Option<String>, std::io::Error>, conn: &Connection, mut local_lamport_time: &mut i64, noeud: &str) -> u8 {
    match line {
        Ok(Some(cmd)) => {
            handle_command(&conn, &mut local_lamport_time,&noeud, cmd);
            print!("> ");
            std_io::stdout().flush().unwrap();
            return 0;
        }
        Ok(None) => {
            log::info!("Aucun input");
            return 0;
        }
        Err(e) => {
            log::error!("Erreur de lecture stdin : {}", e);
            return 1; // code d'erreur
        }
    }
}

fn handle_command(conn: &Connection, lamport_time: &mut i64, noeud: &str, cmd: String) -> () {
    match cmd.as_str() {
        "/create_user" => {
            let mut input = String::new();
            print!("Username > ");
            std_io::stdout().flush().unwrap();
            std_io::stdin().read_line(&mut input).unwrap();
            let name = input.trim();
            db::create_user(&conn, name).unwrap()
        }

        "/user_accounts" => {
            db::print_users(&conn).unwrap();
        }

        "/print_user_tsx" => {
            let mut input = String::new();
            print!("Username > ");
            std_io::stdout().flush().unwrap();
            std_io::stdin().read_line(&mut input).unwrap();
            let name = input.trim();
            db::print_tsx_user(&conn, name).unwrap();
        }

        "/print_tsx" => {
            db::print_tsx(&conn).unwrap();
        }

        // Déposer de l'argent
        "/deposit" => {
            let mut input = String::new();
            print!("Username > ");
            std_io::stdout().flush().unwrap();
            std_io::stdin().read_line(&mut input).unwrap();
            let name = input.trim();

            let mut input = String::new();
            print!("Deposit amount > ");
            std_io::stdout().flush().unwrap();
            std_io::stdin().read_line(&mut input).unwrap();
            let amount = input.trim().parse::<f64>().unwrap();
            db::deposit_user(
                &conn,
                name,
                amount,
                lamport_time,
                noeud,
            )
            .unwrap();
        }

        // Retirer de l'argent
        "/withdraw" => {
            let mut input = String::new();
            print!("Username > ");
            std_io::stdout().flush().unwrap();
            std_io::stdin().read_line(&mut input).unwrap();
            let name = input.trim();

            let mut input = String::new();
            print!("Withdraw amount > ");
            std_io::stdout().flush().unwrap();
            std_io::stdin().read_line(&mut input).unwrap();
            let amount = input.trim().parse::<f64>().unwrap();
            // to do : verifications
            db::withdraw_user(
                &conn,
                name,
                amount,
                lamport_time,
                noeud,
            )
            .unwrap();
        }

        // Faire un virement à qqn d'autre
        "/transfer" => {
            let mut input = String::new();
            print!("Username > ");
            std_io::stdout().flush().unwrap();
            std_io::stdin().read_line(&mut input).unwrap();
            let name = input.trim();

            let mut input = String::new();
            print!("Transfer amount > ");
            std_io::stdout().flush().unwrap();
            std_io::stdin().read_line(&mut input).unwrap();
            let amount: f64 = input.trim().parse::<f64>().unwrap();

            let _ = db::print_users(&conn);

            let mut input = String::new();
            print!("Beneficiary > ");
            std_io::stdout().flush().unwrap();
            std_io::stdin().read_line(&mut input).unwrap();
            let beneficiary = input.trim();

            db::create_tsx(
                &conn,
                name,
                beneficiary,
                amount,
                lamport_time,
                noeud,
                "",
            )
            .unwrap();
        }

        // Payer
        "/pay" => {
            let mut input = String::new();
            print!("Username > ");
            std_io::stdout().flush().unwrap();
            std_io::stdin().read_line(&mut input).unwrap();
            let name = input.trim();

            let mut input = String::new();
            print!("Payment amount > ");
            std_io::stdout().flush().unwrap();
            std_io::stdin().read_line(&mut input).unwrap();
            let amount: f64 = input.trim().parse::<f64>().unwrap();

            db::create_tsx(
                &conn,
                name,
                "NULL",
                amount,
                lamport_time,
                noeud,
                "",
            )
            .unwrap();
        }

        // Se faire rembourser
        "/refund" => {
            let mut input = String::new();
            print!("Username > ");
            std_io::stdout().flush().unwrap();
            std_io::stdin().read_line(&mut input).unwrap();
            let name = input.trim();

            db::print_tsx_user(&conn, name).unwrap();

            let mut input = String::new();
            print!("Lamport time > ");
            std_io::stdout().flush().unwrap();
            std_io::stdin().read_line(&mut input).unwrap();
            let transac_time = input.trim().parse::<i64>().unwrap();

            let mut input = String::new();
            print!("Node > ");
            std_io::stdout().flush().unwrap();
            std_io::stdin().read_line(&mut input).unwrap();
            let transac_node = input.trim();

            db::refund(
                &conn,
                transac_time,
                transac_node,
                lamport_time,
                noeud,
            )
            .unwrap();
        }

        "/help" => {
            log::info!("Command list : ");
            log::info!(
                "/create_user : create the user personnal account"
            );
            log::info!("/user_accounts : list all users");
            log::info!(
                "/print_user_tsx : print the user's transactions"
            );
            log::info!(
                "/print_tsx : print the system's transactions time"
            );
            log::info!("/deposit : make a deposit on your personnal account.");
            log::info!("/withdraw : make a withdraw on your personnal account.");
            log::info!(
                "/transfer : make a transfer from your personnal account to an other user account."
            );
            log::info!(
                "/pay : make a pay from your personnal account."
            );
            log::info!(
                "/refund : get a refund on your personnal account."
            );
        }

        "/quit" => {
            log::info!("👋 Bye !");
            std::process::exit(0);
        }
        _ => log::info!("❓ Unknown command  : {}", cmd),
    }
}
