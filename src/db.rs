#![allow(unused)]

#[derive(Debug)]
pub struct Transaction {
    from_user: String,
    to_user: String,
    amount: f64,
    lamport_time: i64,
    source_node: String,
    optional_msg: Option<String>,
}

const NULL: &str = "NULL";

use rusqlite::ffi::SQLITE_NULL;
use rusqlite::{Connection, Result, params};
use std::cmp::Ordering;
use std::fs::File;
use std::io;
use std::io::{BufRead, BufReader, ErrorKind, Write};

pub fn init_db(conn: &rusqlite::Connection) -> Result<()> {
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
            optional_msg TEXT,
            FOREIGN KEY(from_user) REFERENCES User(unique_name),
            FOREIGN KEY(to_user) REFERENCES User(unique_name),
            PRIMARY KEY(lamport_time,source_node)
        )",
        [],
    )?;

    log::debug!("Database initialized successfully.");
    Ok(())
}

pub fn drop_table(conn: &rusqlite::Connection) -> rusqlite::Result<()> {
    conn.execute("DROP TABLE IF EXISTS Transactions;", [])?;
    conn.execute("DROP TABLE IF EXISTS User;", [])?;
    log::debug!("Tables dropped successfully.");
    Ok(())
}

pub fn create_user(conn: &rusqlite::Connection, unique_name: &str) -> rusqlite::Result<()> {
    // vérifie si l'utilisateur existe déjà
    let mut stmt = conn.prepare("SELECT COUNT(*) FROM User WHERE unique_name = ?1")?;
    let user_exists: i64 = stmt.query_row(rusqlite::params![unique_name], |row| row.get(0))?;

    if user_exists > 0 {
        log::error!("User '{}' {}", unique_name, "already exists.");
        return Ok(());
    }

    conn.execute(
        "INSERT INTO User (unique_name, solde) VALUES (?1, 0)",
        rusqlite::params![unique_name],
    )?;
    log::debug!("User '{}' added with solde 0", unique_name);
    Ok(())
}

pub fn create_tsx(
    conn: &rusqlite::Connection,
    from_user: &str,
    to_user: &str,
    amount: f64,
    lamport_time: &mut i64,
    source_node: &str,
    optional_msg: &str,
) -> rusqlite::Result<()> {
    if from_user != NULL {
        let from_solde = calculate_solde(from_user)?;
        if from_solde < amount {
            // not enought money #broke
            log::error!(
                "Error : Solde '{}' is lower than amount {}, {}",
                from_solde,
                amount,
                "can't make this transaction"
            );
            return Err(rusqlite::Error::InvalidQuery);
        }
    }

    if from_user != NULL {
        // vérifie si l'utilisateur existe
        let mut stmt = conn.prepare("SELECT COUNT(*) FROM User WHERE unique_name = ?1")?;
        let user_exists: i64 = stmt.query_row(rusqlite::params![from_user], |row| row.get(0))?;
        // si non, on le crée
        if user_exists == 0 {
            create_user(&conn, from_user);
        }
    }

    if to_user != NULL {
        // vérifie si l'utilisateur existe
        let mut stmt = conn.prepare("SELECT COUNT(*) FROM User WHERE unique_name = ?1")?;
        let user_exists: i64 = stmt.query_row(rusqlite::params![to_user], |row| row.get(0))?;
        // si non, on le crée
        if user_exists == 0 {
            create_user(&conn, to_user);
        }
    }

    conn.execute(
        "INSERT INTO Transactions (from_user, to_user, amount, lamport_time, source_node, optional_msg) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        rusqlite::params![from_user, to_user, amount, *lamport_time, source_node, optional_msg],
    )?;
    log::debug!(
        "from user: {}, to user: {}, amount: {}, lamport time: {}, source node: {}, optionnal msg: {}",
        from_user,
        to_user,
        amount,
        lamport_time,
        source_node,
        optional_msg
    );
    *lamport_time += 1;
    if from_user != NULL {
        update_solde(&conn, from_user);
    }

    if to_user != NULL {
        update_solde(&conn, to_user);
    }

    Ok(())
}

pub fn deposit_user(
    conn: &rusqlite::Connection,
    unique_name: &str,
    amount: f64,
    lamport_time: &mut i64,
    source_node: &str,
) -> rusqlite::Result<()> {
    if amount < 0.0 {
        // amount should be >0
        log::error!(
            "Amount '{}' {}",
            amount,
            "is negative, can't make this deposit"
        );
        return Err(rusqlite::Error::InvalidQuery);
    }

    create_tsx(
        &conn,
        NULL,
        unique_name,
        amount,
        lamport_time,
        source_node,
        "Deposit",
    );

    update_solde(&conn, unique_name);

    // println!("User '{}' deposed {}€ in User", unique_name, amount);

    Ok(())
}

pub fn withdraw_user(
    conn: &rusqlite::Connection,
    unique_name: &str,
    amount: f64,
    lamport_time: &mut i64,
    source_node: &str,
) -> rusqlite::Result<()> {
    if amount < 0.0 {
        // amount should be >0
        log::error!(
            "Amount '{}' {}",
            amount,
            "is negative, can't make this withdraw"
        );
        return Err(rusqlite::Error::InvalidQuery);
    }

    let solde = calculate_solde(unique_name)?;
    if solde < amount {
        // not enought money #broke
        log::error!(
            "Solde '{}' is lower than amount {}, {}",
            solde,
            amount,
            "can't make this withdraw"
        );
        return Err(rusqlite::Error::InvalidQuery);
    }

    create_tsx(
        &conn,
        unique_name,
        NULL,
        amount,
        lamport_time,
        source_node,
        "Withdraw",
    );

    update_solde(&conn, unique_name);

    // println!("User '{}' withdrawed {}€", unique_name, amount);

    Ok(())
}

pub fn calculate_solde(name: &str) -> Result<f64> {
    let conn: Connection = Connection::open("peillute.db")?;

    // if user don't exist, default return 0

    // somme des transactions positives - négatives
    let mut stmt = conn.prepare(
        "SELECT
            IFNULL((SELECT SUM(amount) FROM Transactions WHERE to_user = ?1), 0) -
            IFNULL((SELECT SUM(amount) FROM Transactions WHERE from_user = ?1), 0)
        AS difference;",
    )?;

    let solde: f64 = stmt.query_row(params![name], |row| row.get(0))?;
    Ok(solde)
}

pub fn update_solde(conn: &rusqlite::Connection, name: &str) -> Result<()> {
    // vérifie si l'utilisateur existe
    let mut stmt = conn.prepare("SELECT COUNT(*) FROM User WHERE unique_name = ?1")?;
    let user_exists: i64 = stmt.query_row(rusqlite::params![name], |row| row.get(0))?;
    // si non, on return
    if user_exists == 0 {
        log::error!("User '{}' {}", name, "doesn't exists, can't update solde.");
        return Ok(());
    }

    let solde = calculate_solde(name)?; // get the f64 value or return error
    conn.execute(
        "UPDATE User SET solde = ?1 WHERE unique_name = ?2",
        params![solde, name],
    )?;

    Ok(())
}

pub fn create_user_solde(
    conn: &rusqlite::Connection,
    unique_name: &str,
    solde: f64,
    lamport_time: &mut i64,
    source_node: &str,
) -> rusqlite::Result<()> {
    // vérifie si l'utilisateur existe déjà
    let mut stmt = conn.prepare("SELECT COUNT(*) FROM User WHERE unique_name = ?1")?;
    let user_exists: i64 = stmt.query_row(rusqlite::params![unique_name], |row| row.get(0))?;

    if user_exists > 0 {
        log::error!("User '{}' {}", unique_name, "already exists.");
        return Ok(());
    }

    create_user(&conn, unique_name); // optionel car create tsx crée aussi les users qui n'existent pas

    create_tsx(
        &conn,
        NULL,
        unique_name,
        solde,
        lamport_time,
        source_node,
        "Deposit",
    );
    update_solde(&conn, unique_name);

    log::debug!("User '{}' added with solde {}", unique_name, solde);
    Ok(())
}

pub fn get_transac(
    conn: &rusqlite::Connection,
    transac_time: i64,
    node: &str,
) -> rusqlite::Result<Option<Transaction>> {
    let mut stmt = conn.prepare(
        "SELECT from_user, to_user, amount, lamport_time, source_node, optional_msg
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

pub fn refund(
    conn: &rusqlite::Connection,
    transac_time: i64,
    node: &str,
    lamport_time: &mut i64,
    source_node: &str,
) -> rusqlite::Result<()> {
    match get_transac(&conn, transac_time, node)? {
        Some(tx) => {
            create_tsx(
                &conn,
                &tx.to_user,
                &tx.from_user,
                tx.amount,
                lamport_time,
                source_node,
                "Refunding",
            );

            /* println!(
                "Refunded transaction from {} to {} of {}€ at time {} from node {}. Message: {:?}",
                tx.from_user, tx.to_user, tx.amount, tx.lamport_time, tx.source_node, tx.optional_msg
            ); */

            update_solde(&conn, &tx.from_user);
            update_solde(&conn, &tx.to_user);
        }
        None => {
            log::error!(
                "{} time:{}, node:{}",
                "No transaction found for the specified time and node. Can not refund",
                transac_time,
                node
            );
        }
    }
    Ok(())
}

pub fn print_users(conn: &rusqlite::Connection) -> rusqlite::Result<()> {
    let mut stmt = conn.prepare("SELECT unique_name, solde FROM User")?;

    // récupère les users dans user_iter, la String nom puis le f64 solde
    let user_iter = stmt.query_map([], |row| {
        let name: String = row.get(0)?;
        let solde: f64 = row.get(1)?;
        Ok((name, solde))
    })?;

    log::debug!("------ Users ------");
    // boucle d'affichage
    for user in user_iter {
        let (name, solde) = user?;
        log::debug!("Name: {}, Solde: {}", name, solde);
    }
    log::debug!("-------------------");

    Ok(())
}

pub fn print_tsx(conn: &rusqlite::Connection) -> rusqlite::Result<()> {
    let mut stmt = conn.prepare(
        "SELECT from_user, to_user, amount, lamport_time, source_node, optional_msg FROM Transactions"
    )?;

    // récupère les transactions dans tsx_iter
    let tsx_iter = stmt.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,         // from_user
            row.get::<_, String>(1)?,         // to_user
            row.get::<_, f64>(2)?,            // amount
            row.get::<_, i64>(3)?,            // lamport_time
            row.get::<_, String>(4)?,         // source_node
            row.get::<_, Option<String>>(5)?, // optional_msg (peut être NULL)
        ))
    })?;

    log::debug!("--- Transactions ---");
    for tsx in tsx_iter {
        let (from_user, to_user, amount, lamport_time, source_node, optional_msg) = tsx?;

        log::debug!(
            "from_user: {}, to_user: {}, amount: {}, lamport_time: {}, source_node: {}, optional_msg: {}",
            from_user,
            to_user,
            amount,
            lamport_time,
            source_node,
            optional_msg.unwrap_or_else(|| "None".to_string())
        );
    }
    log::debug!("-------------------");

    Ok(())
}

pub fn print_tsx_user(conn: &rusqlite::Connection, name: &str) -> rusqlite::Result<()> {
    let mut stmt = conn.prepare(
        "SELECT from_user, to_user, amount, lamport_time, source_node, optional_msg FROM Transactions WHERE from_user=?1 OR to_user=?1"
    )?;

    // récupère les transactions dans tsx_iter
    let tsx_iter = stmt.query_map([name], |row| {
        Ok((
            row.get::<_, String>(0)?,         // from_user
            row.get::<_, String>(1)?,         // to_user
            row.get::<_, f64>(2)?,            // amount
            row.get::<_, i64>(3)?,            // lamport_time
            row.get::<_, String>(4)?,         // source_node
            row.get::<_, Option<String>>(5)?, // optional_msg (peut être NULL)
        ))
    })?;

    log::debug!("--- Transactions of {} ---", name);
    for tsx in tsx_iter {
        let (from_user, to_user, amount, lamport_time, source_node, optional_msg) = tsx?;

        log::debug!(
            "from_user: {}, to_user: {}, amount: {}, lamport_time: {}, source_node: {}, optional_msg: {}",
            from_user,
            to_user,
            amount,
            lamport_time,
            source_node,
            optional_msg.unwrap_or_else(|| "None".to_string())
        );
    }
    log::debug!("-------------------");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    #[test]
    fn test_create_user() -> rusqlite::Result<()> {
        let conn = Connection::open_in_memory()?;
        init_db(&conn)?;

        create_user(&conn, "Charlie")?;

        let mut stmt = conn.prepare("SELECT COUNT(*) FROM User WHERE unique_name = ?1")?;
        let user_exists: i64 = stmt.query_row(rusqlite::params!["Charlie"], |row| row.get(0))?;

        assert_eq!(user_exists, 1);

        Ok(())
    }
}
