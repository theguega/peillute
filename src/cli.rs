use std::io::{self as std_io, Write};
use tokio::io::{self as tokio_io, AsyncBufReadExt, BufReader};
use tokio::select;


#[allow(unused)]
#[allow(dead_code)]
async fn main_loop(){

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
                        handle_command(cmd);
                        print!("> ");
                        std_io::stdout().flush().unwrap();
                    }
                    Ok(None) => {
                        break;
                    }
                    Err(e) => {
                        eprintln!("Erreur de lecture stdin : {}", e);
                        break;
                    }
                }
            }
        }
    }
}

#[allow(unused)]
#[allow(dead_code)]
async fn handle_command(cmd: String) -> (){
    match cmd.as_str() {
        "/create_user" => {
            // To do, création de l'utilisateur en bdd
            let mut input = String::new();
            print!("Username > ");
            std_io::stdout().flush().unwrap();
            std_io::stdin().read_line(&mut input).unwrap();
            let name = input.trim();
            // create_user(name);
        }

        // Déposer de l'argent
        "/deposit" => {
            let mut input = String::new();
            print!("Deposit amount > ");
            std_io::stdout().flush().unwrap();
            std_io::stdin().read_line(&mut input).unwrap();
            let amount = input.trim().parse::<f32>().unwrap();
            // make_deposit(amount);
        }

        // Retirer de l'argent
        "/withdraw" => {
            let mut input = String::new();
            print!("Withdraw amount > ");
            std_io::stdout().flush().unwrap();
            std_io::stdin().read_line(&mut input).unwrap();
            let amount = input.trim().parse::<f32>().unwrap();
            // to do : verifications
            // make_withdraw(amount);
        }

        // Faire un virement à qqn d'autre
        "/transfer" => {
            let mut input = String::new();
            print!("Transfer amount > ");
            std_io::stdout().flush().unwrap();
            std_io::stdin().read_line(&mut input).unwrap();
            let amount = input.trim().parse::<f32>().unwrap();
            // to do : vérifications
            // make_withdraw(amount);
        }

        // Payer
        "/pay" => {
            let mut input = String::new();
            print!("Payment amount > ");
            std_io::stdout().flush().unwrap();
            std_io::stdin().read_line(&mut input).unwrap();
            let amount = input.trim().parse::<f32>().unwrap();
            // to do : vérifications
            // make_withdraw(amount);
        }

        // Se faire rembourser
        "/refund" => {
            // Affichage de l'historique et choix du remboursement à avoir
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