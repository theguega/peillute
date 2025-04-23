use super::db;
use rusqlite::{Connection, Result};
use std::io::{self as std_io, Write};
use tokio::io::{self as tokio_io, AsyncBufReadExt, BufReader};
use tokio::select;

#[allow(unused)]
#[allow(dead_code)]
pub async fn main_loop() -> Result<()> {
    let conn: Connection = Connection::open("peillute.db")?;
    db::drop_table(&conn);
    db::init_db(&conn);
    let noeud = "A";
    let mut local_lamport_time: i64 = 0;

    let stdin = tokio_io::stdin();
    let reader = BufReader::new(stdin);
    let mut lines = reader.lines();

    println!("Welcome on peillute, write /help to get the command list.");
    print!("> ");
    std_io::stdout().flush().unwrap();

    loop {
        select! {
            line = lines.next_line() => {
                match line {
                    Ok(Some(cmd)) => {
                        handle_command(&conn, &mut local_lamport_time,&noeud, cmd);
                        log::info!("> ");
                        std_io::stdout().flush().unwrap();
                    }
                    Ok(None) => {
                        break Ok(());
                    }
                    Err(e) => {
                        log::error!("Erreur de lecture stdin : {}", e);
                    }
                }
            }
        }
    }
}

#[allow(unused)]
#[allow(dead_code)]
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
            db::deposit_user(&conn, name, amount, lamport_time, noeud).unwrap();
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
            db::withdraw_user(&conn, name, amount, lamport_time, noeud).unwrap();
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

            db::print_users(&conn);

            let mut input = String::new();
            print!("Beneficiary > ");
            std_io::stdout().flush().unwrap();
            std_io::stdin().read_line(&mut input).unwrap();
            let beneficiary = input.trim();

            db::create_tsx(&conn, name, beneficiary, amount, lamport_time, noeud, "").unwrap();
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

            db::create_tsx(&conn, name, "NULL", amount, lamport_time, noeud, "").unwrap();
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

            db::refund(&conn, transac_time, transac_node, lamport_time, noeud).unwrap();
        }

        "/help" => {
            println!("Command list : ");
            println!("/create_user : create the user personnal account");
            println!("/user_accounts : list all users");
            println!("/print_user_tsx : print the user's transactions");
            println!("/print_tsx : print the system's transactions time");
            println!("/deposit : make a deposit on your personnal account.");
            println!("/withdraw : make a withdraw on your personnal account.");
            println!(
                "/transfer : make a transfer from your personnal account to an other user account."
            );
            println!("/pay : make a pay from your personnal account.");
            println!("/refund : get a refund on your personnal account.");
        }

        "/quit" => {
            println!("👋 Bye !");
            std::process::exit(0);
        }
        _ => println!("❓ Unknown command  : {}", cmd),
    }
}
