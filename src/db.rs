#![allow(unused)]

use std::io;
use std::io::{Write, BufReader, BufRead, ErrorKind};
use std::fs::File;
use std::cmp::Ordering;
use rusqlite::{params, Connection, Result};


fn main() -> Result<()> {
    // connection à la DB dans le main
    let conn = rusqlite::Connection::open("database.db")?;
    // initialisation de la DB
    let res = init_db();
    println!("{}","ok");
    // création d'un user
    create_user(&conn, "Clem", 150.0)?;
    println!("Users : ");
    print_users(&conn)?;
    println!("Transactions : ");
    print_tsx(&conn)?;
    Ok(())
}

fn create_user(conn: &rusqlite::Connection, unique_name: &str, solde: f64) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO User (unique_name, solde) VALUES (?1, ?2)",
        rusqlite::params![unique_name, solde],
    )?;
    println!("User '{}' added with solde {}", unique_name, solde);
    Ok(())
}

fn print_users(conn: &rusqlite::Connection) -> rusqlite::Result<()> {
    let mut stmt = conn.prepare("SELECT unique_name, solde FROM User")?;

    // récupère les users dans user_iter, la String nom puis le f64 solde
    let user_iter = stmt.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, f64>(1)?))
    })?;

    println!("--- Users ---");
    // boucle d'affichage
    for user in user_iter {
        let (name, solde) = user?;
        println!("Name: {}, Solde: {}", name, solde);
    }
    Ok(())
}

fn print_tsx(conn: &rusqlite::Connection) -> rusqlite::Result<()> {
    let mut stmt = conn.prepare(
        "SELECT tx_id, from_user, to_user, amount, lamport_time, source_node, optionnal_msg FROM Transactions"
    )?;
    
    // récupère les transactions dans tsx_iter
    let tsx_iter = stmt.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,     // tx_id
            row.get::<_, String>(1)?,     // from_user
            row.get::<_, String>(2)?,     // to_user
            row.get::<_, f64>(3)?,        // amount
            row.get::<_, i64>(4)?,        // lamport_time
            row.get::<_, String>(5)?,     // source_node
            row.get::<_, Option<String>>(6)?, // optionnal_msg (peut être NULL)
        ))
    })?;

    println!("--- Transactions ---");
    for tsx in tsx_iter {
        let (tx_id, from_user, to_user, amount, lamport_time, source_node, optionnal_msg) = tsx?;
        println!(
            "tx_id: {}, from_user: {}, to_user: {}, amount: {}, lamport_time: {}, source_node: {}, optionnal_msg: {}",
            tx_id,
            from_user,
            to_user,
            amount,
            lamport_time,
            source_node,
            optionnal_msg.unwrap_or_else(|| "None".to_string())
        );
    }
    Ok(())
}


fn init_db() -> Result<()> {
    // création/connection à la db
    let conn = Connection::open("database.db")?;

    // Création de la table User
    conn.execute(
        "CREATE TABLE IF NOT EXISTS User (
            unique_name TEXT PRIMARY KEY,
            solde REAL NOT NULL
        )",
        [],
    )?;

    // Création de la table Transactions 
    // Attention : le mot Transaction sans S est reservé dans SQL donc on en peut pas l'utiliser (comme Select par exemple)
    conn.execute(
        "CREATE TABLE IF NOT EXISTS Transactions (
            tx_id TEXT PRIMARY KEY,
            from_user TEXT NOT NULL,
            to_user TEXT NOT NULL,
            amount REAL NOT NULL,
            lamport_time INTEGER NOT NULL,
            source_node TEXT NOT NULL,
            optionnal_msg TEXT,
            FOREIGN KEY(from_user) REFERENCES User(unique_name),
            FOREIGN KEY(to_user) REFERENCES User(unique_name)
        )",
        [],
    )?;

    println!("Database initialized successfully.");
    Ok(())
}

