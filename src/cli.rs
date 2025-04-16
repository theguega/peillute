use std::io::{self, Write};

/*
To do : rendre la boucle asynchrone avec les intérruptions
Pour l'instant, pas asynchrone, le but est d'avoir d'une première ébauche
 */
pub fn main_loop(){
    print!("Welcome on peillute, write /help to get the command list.");
    io::stdout().flush().unwrap();
    loop {
        print!(">");
        io::stdout().flush().unwrap();

        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();

        let mut commande = input.trim();
        match commande {
            "/create_user" => {

                // To do, création de l'utilisateur en bdd
            }

            // Déposer de l'argent
            "/deposit" => {
                print!("Deposit amount > ");
                io::stdout().flush().unwrap();
                io::stdin().read_line(&mut input).unwrap();
                let amount = input.trim().parse::<f32>().unwrap();
                // make_deposit(amount);
            }

            // Retirer de l'argent
            "/withdraw" => {
                print!("Withdraw amount > ");
                io::stdout().flush().unwrap();
                io::stdin().read_line(&mut input).unwrap();
                let amount = input.trim().parse::<f32>().unwrap();
                // to do : verifications
                // make_withdraw(amount);
            }

            // Faire un virement à qqn d'autre
            "/transfer" => {
                print!("Transfer amount > ");
                io::stdout().flush().unwrap();
                io::stdin().read_line(&mut input).unwrap();
                let amount = input.trim().parse::<f32>().unwrap();
                // to do : vérifications
                // make_withdraw(amount);
            }

            // Payer
            "/pay" => {
                print!("Payment amount > ");
                io::stdout().flush().unwrap();
                io::stdin().read_line(&mut input).unwrap();
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
                break;
            }
            _ => println!("❓ Unknown command  : {}", commande)
        }
    }

}
