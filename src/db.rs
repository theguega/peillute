#![allow(unused)]

#[derive(Debug)]
struct Transaction {
    from_user: String,
    to_user: String,
    amount: f64,
    lamport_time: i64,
    source_node: String,
    optional_msg: Option<String>,
}

use std::io;
use std::io::{Write, BufReader, BufRead, ErrorKind};
use std::fs::File;
use std::cmp::Ordering;
use rusqlite::ffi::SQLITE_NULL;
use rusqlite::{params, Connection, Result};

fn main() -> Result<()> {

    test();

    Ok(())
}

fn test() -> Result<()> {

    // initialisation du noeud et de l'horloge
    let noeud = "A"; // noeud non mutable
    // let mut local_lamport_time: &mut i64 =&mut 0; // temps de lamport local qui est incrémenté à chaque action
    let mut local_lamport_time = 0;

    // initialisation de la connexion
    let conn: Connection = rusqlite::Connection::open("database.db")?;
    // on supprime la db déjà existante pour des tests clean
    drop_table();
    // initialisation de la db (create table) 
    init_db();
    // création des users Alice et Bob
    create_user(&conn, "Alice")?;
    create_user(&conn, "Bob")?;
    // verification
    print_users(&conn)?;

    // premier dépot des utilisateurs, horodatés par l'heure véctorielle de lamport
    deposit_user(&conn, "Alice", 150.0, &mut local_lamport_time, noeud,);
    deposit_user(&conn, "Bob", 250.0, &mut local_lamport_time, noeud,);

    // transactions entre les utilisateurs
    create_tsx(&conn, "Alice", "Bob", 100.0, &mut local_lamport_time, noeud, "Cookie");
    create_tsx(&conn, "Bob", "Alice", 79.0, &mut local_lamport_time, noeud, "Pizza party");

    // retrait de Bob
    withdraw_user(&conn, "Bob", 100.0, &mut local_lamport_time, noeud);

    // remboursement de la transaction A3 : Alice-> Bob 79 pour les pizzas
    refund(&conn, 3 , "A", &mut local_lamport_time, noeud);

    // print la table user et la table transaction
    println!("");
    print_users(&conn);
    print_tsx(&conn);

    Ok(())

}

fn drop_table() -> rusqlite::Result<()> {
    let conn: Connection = rusqlite::Connection::open("database.db")?;
    conn.execute("DROP TABLE IF EXISTS Transactions;", [])?;
    conn.execute("DROP TABLE IF EXISTS User;", [])?;
    println!("Tables dropped successfully.");
    Ok(())
}

fn init_db() -> Result<()> {
    // création/connection à la db
    let conn = Connection::open("database.db")?;

    // Création de la table User
    conn.execute(
        "CREATE TABLE IF NOT EXISTS User (
            unique_name TEXT PRIMARY KEY,
            solde FLOAT NOT NULL
        )",
        [],
    )?;

    // Création de la table Transactions 
    // Attention : le mot Transaction sans S est reservé dans SQL donc on en peut pas l'utiliser (comme Select par exemple)
    conn.execute(
        "CREATE TABLE IF NOT EXISTS Transactions (
            from_user TEXT,
            to_user TEXT NOT NULL,
            amount FLOAT NOT NULL,
            lamport_time INTEGER NOT NULL,
            source_node TEXT NOT NULL,
            optionnal_msg TEXT,
            FOREIGN KEY(from_user) REFERENCES User(unique_name),
            FOREIGN KEY(to_user) REFERENCES User(unique_name),
            PRIMARY KEY(lamport_time,source_node)
        )",
        [],
    )?;

    println!("Database initialized successfully.");
    Ok(())
}

fn create_user(conn: &rusqlite::Connection, unique_name: &str) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO User (unique_name, solde) VALUES (?1,0)",
        rusqlite::params![unique_name],
    )?;
    println!("User '{}' added with solde 0", unique_name);
    Ok(())
}

fn create_tsx(conn: &rusqlite::Connection, from_user: &str, to_user: &str, amount: f64, lamport_time: &mut i64, source_node: &str, optionnal_msg: &str) -> rusqlite::Result<()> {
    
    if from_user!="NULL"{
        let from_solde=calculate_solde(from_user)?;
        if from_solde<amount{
            // not enought money #broke
            return Err(rusqlite::Error::InvalidQuery);
        }
    }
    
    conn.execute(
        "INSERT INTO Transactions (from_user, to_user, amount, lamport_time, source_node, optionnal_msg) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        rusqlite::params![from_user, to_user, amount, *lamport_time, source_node, optionnal_msg],
    )?;
    println!("from user: {}, to user: {}, amount: {}, lamport time: {}, source node: {}, optionnal msg: {}", from_user, to_user, amount, lamport_time, source_node, optionnal_msg);
    *lamport_time+=1;
    update_solde(from_user);
    update_solde(to_user);

    Ok(())
}

fn deposit_user(conn: &rusqlite::Connection, unique_name: &str, amount: f64, lamport_time: &mut i64, source_node: &str) -> rusqlite::Result<()> {
    if amount<0.0{
        // amount should be >0
        return Err(rusqlite::Error::InvalidQuery);
    }
    create_tsx(&conn, "NULL", unique_name, amount, lamport_time, source_node, "Deposit");

    update_solde(unique_name);
    
    // println!("User '{}' deposed {}€ in User", unique_name, amount);

    Ok(())
}

fn withdraw_user(conn: &rusqlite::Connection, unique_name: &str, amount: f64, lamport_time: &mut i64, source_node: &str) -> rusqlite::Result<()> {
    if amount<0.0{
        // amount should be >0
        return Err(rusqlite::Error::InvalidQuery);
    }

    let solde = calculate_solde(unique_name)?;
    if solde<amount{
        // not enought money #broke
        return Err(rusqlite::Error::InvalidQuery);
    }

    create_tsx(&conn, unique_name, "NULL", amount, lamport_time, source_node, "Withdraw");

    update_solde(unique_name);
    
    // println!("User '{}' withdrawed {}€", unique_name, amount);

    Ok(())
}

fn calculate_solde(name: &str) -> Result<f64> {
    let conn: Connection = Connection::open("database.db")?;

    let mut stmt = conn.prepare(
        "SELECT
            IFNULL((SELECT SUM(amount) FROM Transactions WHERE to_user = ?1), 0) -
            IFNULL((SELECT SUM(amount) FROM Transactions WHERE from_user = ?1), 0)
        AS difference;"
    )?;

    let solde: f64 = stmt.query_row(params![name], |row| row.get(0))?;
    Ok(solde)
}

fn update_solde(name: &str) -> Result<()> {
    let conn = Connection::open("database.db")?;

    let solde = calculate_solde(name)?; // get the f64 value or return error
    conn.execute(
        "UPDATE User SET solde = ?1 WHERE unique_name = ?2",
        params![solde, name],
    )?;

    Ok(())
}

fn create_user_solde(conn: &rusqlite::Connection, unique_name: &str, solde: f64, lamport_time: &mut i64, source_node: &str ) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO User (unique_name, solde) VALUES (?1, ?2)",
        rusqlite::params![unique_name, solde],
    )?;

    create_tsx(&conn, "", unique_name, solde, lamport_time, source_node, "Deposit");
    update_solde(unique_name);

    println!("User '{}' added with solde {}", unique_name, solde);
    Ok(())
}

fn get_transac(
    conn: &rusqlite::Connection,
    transac_time: i64,
    node: &str
) -> rusqlite::Result<Option<Transaction>> {
    let mut stmt = conn.prepare(
        "SELECT from_user, to_user, amount, lamport_time, source_node, optionnal_msg 
         FROM Transactions 
         WHERE lamport_time = ?1 AND source_node = ?2",
    )?;

    let transaction = stmt.query_row(params![transac_time, node], |row| {
        Ok(Transaction {
            from_user: row.get(0)?,
            to_user: row.get(1)?,
            amount: row.get(2)?,
            lamport_time: row.get(3)?,
            source_node: row.get(4)?,
            optional_msg: row.get(5)?,
        })
    });

    match transaction {
        Ok(t) => Ok(Some(t)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e),
    }
}

fn refund(conn: &rusqlite::Connection, transac_time:i64 , node: &str, lamport_time: &mut i64, source_node: &str) -> rusqlite::Result<()> {
    
    match get_transac(&conn, transac_time, node)? {
        Some(tx) => {
            create_tsx(&conn, &tx.to_user, &tx.from_user, tx.amount, lamport_time, source_node, "Refunding");

            /* println!(
                "Refunded transaction from {} to {} of {}€ at time {} from node {}. Message: {:?}",
                tx.from_user, tx.to_user, tx.amount, tx.lamport_time, tx.source_node, tx.optional_msg
            ); */

            update_solde(&tx.from_user);
            update_solde(&tx.to_user);
        
        }
        None => {
            println!("No transaction found for the specified time and node. Can not refund");
        }
    }
    Ok(())
}

fn print_users(conn: &rusqlite::Connection) -> rusqlite::Result<()> {
    let mut stmt = conn.prepare("SELECT unique_name, solde FROM User")?;

    // récupère les users dans user_iter, la String nom puis le f64 solde
    let user_iter = stmt.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, f64>(1)?))
    })?;

    println!("------ Users ------");
    // boucle d'affichage
    for user in user_iter {
        let (name, solde) = user?;
        println!("Name: {}, Solde: {}", name, solde);
    }
    println!("-------------------");

    Ok(())
}

fn print_tsx(conn: &rusqlite::Connection) -> rusqlite::Result<()> {
    let mut stmt = conn.prepare(
        "SELECT from_user, to_user, amount, lamport_time, source_node, optionnal_msg FROM Transactions"
    )?;

    // récupère les transactions dans tsx_iter
    let tsx_iter = stmt.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,     // from_user
            row.get::<_, String>(1)?,     // to_user
            row.get::<_, f64>(2)?,        // amount
            row.get::<_, i64>(3)?,        // lamport_time
            row.get::<_, String>(4)?,     // source_node
            row.get::<_, Option<String>>(5)?, // optionnal_msg (peut être NULL)
        ))
    })?;

    println!("--- Transactions ---");
    for tsx in tsx_iter {

        let (from_user, to_user, amount, lamport_time, source_node, optionnal_msg) = tsx?;

        println!(
            "from_user: {}, to_user: {}, amount: {}, lamport_time: {}, source_node: {}, optionnal_msg: {}",
            from_user,
            to_user,
            amount,
            lamport_time,
            source_node,
            optionnal_msg.unwrap_or_else(|| "None".to_string())
        );
    }
    println!("-------------------");

    Ok(())
}
