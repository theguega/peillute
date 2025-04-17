use std::ffi::c_int;
use std::io::{self as std_io, Write};
use tokio::io::{self as tokio_io, AsyncBufReadExt, BufReader};
use tokio::select;
use rusqlite::{Connection, Result};
use rusqlite::types::Type::Null;
use super::db;
#[allow(unused)]
#[allow(dead_code)]
async fn main_loop()->Result<()>{

    let conn: Connection = Connection::open("database.db")?;
    db::drop_table();
    db::init_db();
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
                        print!("> ");
                        std_io::stdout().flush().unwrap();
                    }
                    Ok(None) => {
                        break Ok(());
                    }
                    Err(e) => {
                        eprintln!("Erreur de lecture stdin : {}", e);
                        break Ok(());
                    }
                }
            }
        }
    }
}

#[allow(unused)]
#[allow(dead_code)]
fn handle_command(conn : &Connection, lamport_time: &mut i64, noeud : &str, cmd: String) -> (){
    match cmd.as_str() {
        "/create_user" => {
            // To do, création de l'utilisateur en bdd
            let mut input = String::new();
            print!("Username > ");
            std_io::stdout().flush().unwrap();
            std_io::stdin().read_line(&mut input).unwrap();
            let name = input.trim();
            db::create_user(&conn,name).unwrap()
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
            db::deposit_user(&conn,name,amount,lamport_time,noeud).unwrap();
        }

        // Retirer de l'argent
        "/withdraw" => {
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
            // to do : verifications
            db::withdraw_user(&conn,name,amount,lamport_time,noeud).unwrap();
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
            let amount : f64 = input.trim().parse::<f64>().unwrap();

            db::print_users(&conn);

            let mut input = String::new();
            print!("Beneficiary > ");
            std_io::stdout().flush().unwrap();
            std_io::stdin().read_line(&mut input).unwrap();
            let beneficiary = input.trim();

            db::create_tsx(&conn,name,beneficiary,amount,lamport_time,noeud,"").unwrap();

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
            let amount : f64 = input.trim().parse::<f64>().unwrap();

            db::create_tsx(&conn,name,Null,amount,lamport_time,noeud,"").unwrap();
        }

        // Se faire rembourser
        "/refund" => {
            let mut input = String::new();
            print!("Username > ");
            std_io::stdout().flush().unwrap();
            std_io::stdin().read_line(&mut input).unwrap();
            let name = input.trim();

            db::print_tsx(&conn).unwrap();

            let mut input = String::new();
            print!("Lamport time > ");
            std_io::stdout().flush().unwrap();
            std_io::stdin().read_line(&mut input).unwrap();
            let lamport_time = input.trim().parse::<i64>().unwrap();

            let mut input = String::new();
            print!("Node > ");
            std_io::stdout().flush().unwrap();
            std_io::stdin().read_line(&mut input).unwrap();
            let noeud = input.trim();

            // To do : revert_tsx(lamport_time,node)


        }

        "/help" => {
            println!("Command list : ");
            println!("/create_user : create the user personnal account");
            println!("/deposit : make a deposit on your personnal account.");
            println!("/withdraw : make a withdraw on your personnal account.");
            println!("/transfer : make a transfer from your personnal account to an other user account.");
            println!("/pay : make a pay from your personnal account.");
            println!("/refund : get a refund on your personnal account.");
        }

        "/quit" => {
            println!("👋 Bye !");
            std::process::exit(0);
        }
        _ => println!("❓ Unknown command  : {}", cmd)
    }
}

#[allow(unused)]
#[allow(dead_code)]
async fn main() -> Result<()> {

    let _ = main_loop().await;

    Ok(())
}