use rusqlite::{Connection, Result, params};

#[allow(unused)]
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

pub fn init_db(conn: &Connection) -> Result<()> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS User (
            unique_name TEXT PRIMARY KEY,
            solde FLOAT NOT NULL
        )",
        [],
    )?;

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
            PRIMARY KEY(lamport_time, source_node)
        )",
        [],
    )?;

    log::debug!("Database initialized successfully.");
    Ok(())
}

pub fn is_database_initialized(conn: &Connection) -> Result<bool> {
    let mut stmt = conn.prepare(
        "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'Transactions')",
    )?;
    let exists: bool = stmt.query_row([], |row| row.get(0))?;
    Ok(exists)
}

pub fn drop_tables(conn: &Connection) -> Result<()> {
    conn.execute("DROP TABLE IF EXISTS Transactions;", [])?;
    conn.execute("DROP TABLE IF EXISTS User;", [])?;
    log::debug!("Tables dropped successfully.");
    Ok(())
}

pub fn user_exists(conn: &Connection, name: &str) -> Result<bool> {
    let mut stmt = conn.prepare("SELECT EXISTS(SELECT 1 FROM User WHERE unique_name = ?1)")?;
    let exists: bool = stmt.query_row(params![name], |row| row.get(0))?;
    Ok(exists)
}

pub fn create_user(conn: &Connection, unique_name: &str) -> Result<()> {
    if user_exists(conn, unique_name)? {
        log::warn!("User '{}' already exists.", unique_name);
        return Ok(());
    }
    conn.execute(
        "INSERT INTO User (unique_name, solde) VALUES (?1, 0)",
        params![unique_name],
    )?;
    Ok(())
}

pub fn calculate_solde(name: &str) -> Result<f64> {
    let conn = Connection::open("peillute.db")?;
    let mut stmt = conn.prepare(
        "SELECT
            IFNULL((SELECT SUM(amount) FROM Transactions WHERE to_user = ?1), 0) -
            IFNULL((SELECT SUM(amount) FROM Transactions WHERE from_user = ?1), 0)
        AS balance",
    )?;
    stmt.query_row(params![name], |row| row.get(0))
}

pub fn update_solde(conn: &Connection, name: &str) -> Result<()> {
    if !user_exists(conn, name)? {
        log::error!("User '{}' does not exist.", name);
        return Ok(());
    }
    let solde = calculate_solde(name)?;
    conn.execute(
        "UPDATE User SET solde = ?1 WHERE unique_name = ?2",
        params![solde, name],
    )?;
    Ok(())
}

pub fn ensure_user(conn: &Connection, name: &str) -> Result<()> {
    if name != NULL && !user_exists(conn, name)? {
        create_user(conn, name)?;
    }
    Ok(())
}

pub fn create_transaction(
    conn: &Connection,
    from_user: &str,
    to_user: &str,
    amount: f64,
    lamport_time: &mut i64,
    source_node: &str,
    optional_msg: &str,
) -> Result<()> {
    if from_user != NULL && calculate_solde(from_user)? < amount {
        log::error!(
            "Insufficient funds: '{}' has less than {}.",
            from_user,
            amount
        );
        return Err(rusqlite::Error::InvalidQuery);
    }

    ensure_user(conn, from_user)?;
    ensure_user(conn, to_user)?;

    conn.execute(
        "INSERT INTO Transactions (from_user, to_user, amount, lamport_time, source_node, optional_msg)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![from_user, to_user, amount, *lamport_time, source_node, optional_msg],
    )?;

    *lamport_time += 1;

    if from_user != NULL {
        update_solde(conn, from_user)?;
    }
    if to_user != NULL {
        update_solde(conn, to_user)?;
    }

    Ok(())
}

pub fn deposit(
    conn: &Connection,
    user: &str,
    amount: f64,
    lamport_time: &mut i64,
    source_node: &str,
) -> Result<()> {
    if amount < 0.0 {
        log::error!("Negative deposit amount: {}", amount);
        return Err(rusqlite::Error::InvalidQuery);
    }
    create_transaction(
        conn,
        NULL,
        user,
        amount,
        lamport_time,
        source_node,
        "Deposit",
    )
}

pub fn withdraw(
    conn: &Connection,
    user: &str,
    amount: f64,
    lamport_time: &mut i64,
    source_node: &str,
) -> Result<()> {
    if amount < 0.0 {
        log::error!("Negative withdrawal amount: {}", amount);
        return Err(rusqlite::Error::InvalidQuery);
    }
    create_transaction(
        conn,
        user,
        NULL,
        amount,
        lamport_time,
        source_node,
        "Withdraw",
    )
}

#[allow(unused)]
pub fn create_user_with_solde(
    conn: &Connection,
    unique_name: &str,
    solde: f64,
    lamport_time: &mut i64,
    source_node: &str,
) -> Result<()> {
    create_user(conn, unique_name)?;
    create_transaction(
        conn,
        NULL,
        unique_name,
        solde,
        lamport_time,
        source_node,
        "Initial deposit",
    )
}

pub fn get_transaction(
    conn: &Connection,
    transac_time: i64,
    node: &str,
) -> Result<Option<Transaction>> {
    let mut stmt = conn.prepare(
        "SELECT from_user, to_user, amount, lamport_time, source_node, optional_msg
        FROM Transactions WHERE lamport_time = ?1 AND source_node = ?2",
    )?;

    match stmt.query_row(params![transac_time, node], |row| {
        Ok(Transaction {
            from_user: row.get(0)?,
            to_user: row.get(1)?,
            amount: row.get(2)?,
            lamport_time: row.get(3)?,
            source_node: row.get(4)?,
            optional_msg: row.get(5)?,
        })
    }) {
        Ok(tx) => Ok(Some(tx)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e),
    }
}

pub fn refund_transaction(
    conn: &Connection,
    transac_time: i64,
    node: &str,
    lamport_time: &mut i64,
    source_node: &str,
) -> Result<()> {
    if let Some(tx) = get_transaction(conn, transac_time, node)? {
        create_transaction(
            conn,
            &tx.to_user,
            &tx.from_user,
            tx.amount,
            lamport_time,
            source_node,
            "Refund",
        )?;
    } else {
        log::error!(
            "No transaction found at time {} from node {}",
            transac_time,
            node
        );
    }
    Ok(())
}

pub fn print_users(conn: &Connection) -> Result<()> {
    let mut stmt = conn.prepare("SELECT unique_name, solde FROM User")?;
    let users = stmt.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, f64>(1)?))
    })?;

    log::info!("-- Users --");
    for user in users {
        let (name, solde) = user?;
        log::info!("{}: {:.2}", name, solde);
    }
    Ok(())
}

pub fn print_transactions(conn: &Connection) -> Result<()> {
    let mut stmt = conn.prepare(
        "SELECT from_user, to_user, amount, lamport_time, source_node, optional_msg FROM Transactions",
    )?;
    let txs = stmt.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, f64>(2)?,
            row.get::<_, i64>(3)?,
            row.get::<_, String>(4)?,
            row.get::<_, Option<String>>(5)?,
        ))
    })?;

    log::info!("-- Transactions --");
    for tx in txs {
        let (from, to, amount, time, node, msg) = tx?;
        log::info!(
            "{} -> {} | {:.2} | time: {} | node: {} | msg: {:?}",
            from,
            to,
            amount,
            time,
            node,
            msg
        );
    }
    Ok(())
}

pub fn print_transaction_for_user(conn: &Connection, name: &str) -> Result<()> {
    let mut stmt = conn.prepare(
        "SELECT from_user, to_user, amount, lamport_time, source_node, optional_msg
        FROM Transactions WHERE from_user = ?1",
    )?;

    let txs = stmt.query_map(params![name], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, f64>(2)?,
            row.get::<_, i64>(3)?,
            row.get::<_, String>(4)?,
            row.get::<_, Option<String>>(5)?,
        ))
    })?;

    log::info!("-- Transactions for user {} --", name);
    for tx in txs {
        let (from, to, amount, time, node, msg) = tx?;
        log::info!(
            "{} -> {} | {:.2} | time: {} | node: {} | msg: {:?}",
            from,
            to,
            amount,
            time,
            node,
            msg
        );
    }
    Ok(())
}
