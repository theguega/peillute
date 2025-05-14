#[allow(unused)]
#[derive(Debug)]
pub struct Transaction {
    from_user: String,
    to_user: String,
    amount: f64,
    lamport_time: i64,
    source_node: String,
    optional_msg: Option<String>,
    vector_clock: std::collections::HashMap<String, i64>,
}

lazy_static::lazy_static! {
    static ref DB_CONN: std::sync::Mutex<rusqlite::Connection> =
        std::sync::Mutex::new(rusqlite::Connection::open("peillute.db").unwrap());
}

const NULL: &str = "NULL";

pub fn init_db() -> rusqlite::Result<()> {
    {
        let conn = DB_CONN.lock().unwrap();

        conn.execute(
            "CREATE TABLE IF NOT EXISTS VectorClock (
            id INTEGER PRIMARY KEY AUTOINCREMENT
        );",
            [],
        )?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS VectorClockEntry (
                vector_clock_id INTEGER,
                site_id TEXT,
                value INTEGER NOT NULL,
                PRIMARY KEY(vector_clock_id, site_id),
                FOREIGN KEY(vector_clock_id) REFERENCES VectorClock(id) ON DELETE CASCADE
            );
            ",
            [],
        )?;

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
                vector_clock_id INTEGER NOT NULL,
                source_node TEXT NOT NULL,
                optional_msg TEXT,
                FOREIGN KEY(from_user) REFERENCES User(unique_name),
                FOREIGN KEY(to_user) REFERENCES User(unique_name),
                FOREIGN KEY(vector_clock_id) REFERENCES VectorClock(id), 
                PRIMARY KEY(lamport_time, source_node)
            );",
            [],
        )?;
    }

    log::debug!("Database initialized successfully.");
    Ok(())
}

pub fn is_database_initialized() -> rusqlite::Result<bool> {
    {
        let conn = DB_CONN.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'Transactions')",
        )?;
        let exists: bool = stmt.query_row([], |row| row.get(0))?;
        Ok(exists)
    }
}

#[allow(unused)]
pub fn drop_tables() -> rusqlite::Result<()> {
    {
        let conn = DB_CONN.lock().unwrap();
        conn.execute("DROP TABLE IF EXISTS Transactions;", [])?;
        conn.execute("DROP TABLE IF EXISTS User;", [])?;
    }
    log::debug!("Tables dropped successfully.");
    Ok(())
}

pub fn user_exists(name: &str) -> rusqlite::Result<bool> {
    {
        use rusqlite::params;
        let conn = DB_CONN.lock().unwrap();
        let mut stmt = conn.prepare("SELECT EXISTS(SELECT 1 FROM User WHERE unique_name = ?1)")?;
        let exists: bool = stmt.query_row(params![name], |row| row.get(0))?;
        Ok(exists)
    }
}

pub fn create_user(unique_name: &str) -> rusqlite::Result<()> {
    use rusqlite::params;
    if user_exists(unique_name)? {
        log::warn!("User '{}' already exists.", unique_name);
        return Ok(());
    }

    {
        let conn = DB_CONN.lock().unwrap();
        conn.execute(
            "INSERT INTO User (unique_name, solde) VALUES (?1, 0)",
            params![unique_name],
        )?;
        Ok(())
    }
}

pub fn calculate_solde(name: &str) -> rusqlite::Result<f64> {
    {
        use rusqlite::params;
        let conn = DB_CONN.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT
            IFNULL((SELECT SUM(amount) FROM Transactions WHERE to_user = ?1), 0) -
            IFNULL((SELECT SUM(amount) FROM Transactions WHERE from_user = ?1), 0)
        AS balance",
        )?;
        stmt.query_row(params![name], |row| row.get(0))
    }
}

pub fn update_solde(name: &str) -> rusqlite::Result<()> {
    use rusqlite::params;
    if !user_exists(name)? {
        log::error!("User '{}' does not exist.", name);
        return Ok(());
    }
    let solde = calculate_solde(name)?;
    {
        let conn = DB_CONN.lock().unwrap();
        conn.execute(
            "UPDATE User SET solde = ?1 WHERE unique_name = ?2",
            params![solde, name],
        )?;
        Ok(())
    }
}

pub fn ensure_user(name: &str) -> rusqlite::Result<()> {
    if name != NULL && !user_exists(name)? {
        create_user(name)?;
    }
    Ok(())
}

pub fn create_transaction(
    from_user: &str,
    to_user: &str,
    amount: f64,
    lamport_time: &i64,
    source_node: &str,
    optional_msg: &str,
    vector_clock: &std::collections::HashMap<String, i64>,
) -> rusqlite::Result<()> {
    use rusqlite::params;
    if from_user != NULL && calculate_solde(from_user)? < amount {
        log::error!(
            "Insufficient funds: '{}' has less than {}.",
            from_user,
            amount
        );
        return Err(rusqlite::Error::InvalidQuery);
    }

    ensure_user(from_user)?;
    ensure_user(to_user)?;

    {
        let conn = DB_CONN.lock().unwrap();
        conn.execute("INSERT INTO VectorClock DEFAULT VALUES", [])?;
        let vector_clock_id = conn.last_insert_rowid();

        let mut stmt = conn.prepare(
            "INSERT INTO VectorClockEntry (vector_clock_id, site_id, value) VALUES (?1, ?2, ?3)",
        )?;
        for (site_id, value) in vector_clock.iter() {
            stmt.execute(params![vector_clock_id, site_id, value])?;
        }

        conn.execute(
        "INSERT INTO Transactions (from_user, to_user, amount, lamport_time, vector_clock_id, source_node, optional_msg)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            from_user,
            to_user,
            amount,
            *lamport_time,
            vector_clock_id,
            source_node,
            optional_msg
        ],
    )?;
    }

    if from_user != NULL {
        update_solde(from_user)?;
    }
    if to_user != NULL {
        update_solde(to_user)?;
    }

    Ok(())
}

pub fn deposit(
    user: &str,
    amount: f64,
    lamport_time: &i64,
    source_node: &str,
    vector_clock: &std::collections::HashMap<String, i64>,
) -> rusqlite::Result<()> {
    if amount < 0.0 {
        log::error!("Negative deposit amount: {}", amount);
        return Err(rusqlite::Error::InvalidQuery);
    }
    if !user_exists(user)? {
        return Err(rusqlite::Error::InvalidQuery);
    }
    create_transaction(
        NULL,
        user,
        amount,
        lamport_time,
        source_node,
        "Deposit",
        vector_clock,
    )
}

pub fn withdraw(
    user: &str,
    amount: f64,
    lamport_time: &i64,
    source_node: &str,
    vector_clock: &std::collections::HashMap<String, i64>,
) -> rusqlite::Result<()> {
    if amount < 0.0 {
        log::error!("Negative withdrawal amount: {}", amount);
        return Err(rusqlite::Error::InvalidQuery);
    }
    if !user_exists(user)? {
        return Err(rusqlite::Error::InvalidQuery);
    }
    if calculate_solde(user)? < amount {
        return Err(rusqlite::Error::InvalidQuery);
    }
    create_transaction(
        user,
        NULL,
        amount,
        lamport_time,
        source_node,
        "Withdraw",
        vector_clock,
    )
}

pub fn create_user_with_solde(
    unique_name: &str,
    solde: f64,
    lamport_time: &i64,
    source_node: &str,
    vector_clock: &std::collections::HashMap<String, i64>,
) -> rusqlite::Result<()> {
    create_user(unique_name)?;
    create_transaction(
        NULL,
        unique_name,
        solde,
        lamport_time,
        source_node,
        "Initial deposit",
        vector_clock,
    )
}

pub fn refund_transaction(
    transac_time: i64,
    node: &str,
    lamport_time: &i64,
    source_node: &str,
    vector_clock: &std::collections::HashMap<String, i64>,
) -> rusqlite::Result<()> {
    if let Some(tx) = get_transaction(transac_time, node)? {
        create_transaction(
            &tx.to_user,
            &tx.from_user,
            tx.amount,
            lamport_time,
            source_node,
            "Refund",
            vector_clock,
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

pub fn get_transaction(transac_time: i64, node: &str) -> rusqlite::Result<Option<Transaction>> {
    use rusqlite::params;
    {
        let conn = DB_CONN.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT from_user, to_user, amount, lamport_time, source_node, optional_msg, vector_clock_id
        FROM Transactions WHERE lamport_time = ?1 AND source_node = ?2",
        )?;

        match stmt.query_row(params![transac_time, node], |row| {
            let from_user: String = row.get(0)?;
            let to_user: String = row.get(1)?;
            let amount: f64 = row.get(2)?;
            let lamport_time: i64 = row.get(3)?;
            let source_node: String = row.get(4)?;
            let optional_msg: Option<String> = row.get(5)?;
            let vector_clock_id: i64 = row.get(6)?;

            let mut clock_map = std::collections::HashMap::new();
            let mut vc_stmt = conn.prepare(
                "SELECT site_id, value FROM VectorClockEntry WHERE vector_clock_id = ?1",
            )?;
            let mut rows = vc_stmt.query(params![vector_clock_id])?;
            while let Some(vc_row) = rows.next()? {
                let site_id: String = vc_row.get(0)?;
                let value: i64 = vc_row.get(1)?;
                clock_map.insert(site_id, value);
            }

            Ok(Transaction {
                from_user,
                to_user,
                amount,
                lamport_time,
                source_node,
                optional_msg,
                vector_clock: clock_map,
            })
        }) {
            Ok(tx) => Ok(Some(tx)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e),
        }
    }
}

pub fn print_users() -> rusqlite::Result<()> {
    {
        let conn = DB_CONN.lock().unwrap();
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
}

pub fn print_transactions() -> rusqlite::Result<()> {
    {
        let conn = DB_CONN.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT from_user, to_user, amount, lamport_time, source_node, optional_msg, vector_clock_id FROM Transactions",
        )?;
        let txs = stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, f64>(2)?,
                row.get::<_, i64>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, Option<String>>(5)?,
                row.get::<_, i64>(6)?,
            ))
        })?;

        log::info!("-- Transactions --");
        for tx in txs {
            let (from, to, amount, time, node, msg, vector_clock_id) = tx?;
            let mut clock_map = std::collections::HashMap::new();
            let mut vc_stmt = conn.prepare(
                "SELECT site_id, value FROM VectorClockEntry WHERE vector_clock_id = ?1",
            )?;
            let mut rows = vc_stmt.query(rusqlite::params![vector_clock_id])?;
            while let Some(vc_row) = rows.next()? {
                let site_id: String = vc_row.get(0)?;
                let value: i64 = vc_row.get(1)?;
                clock_map.insert(site_id, value);
            }

            log::info!(
                "{} -> {} | {:.2} | time: {} | node: {} | msg: {:?} | vector_clock: {:?}",
                from,
                to,
                amount,
                time,
                node,
                msg,
                clock_map
            );
        }
        Ok(())
    }
}

pub fn print_transaction_for_user(name: &str) -> rusqlite::Result<()> {
    use rusqlite::params;
    {
        let conn = DB_CONN.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT from_user, to_user, amount, lamport_time, source_node, optional_msg, vector_clock_id
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
                row.get::<_, i64>(6)?,
            ))
        })?;

        log::info!("-- Transactions for user {} --", name);
        for tx in txs {
            let (from, to, amount, time, node, msg, vector_clock_id) = tx?;
            let mut clock_map = std::collections::HashMap::new();
            let mut vc_stmt = conn.prepare(
                "SELECT site_id, value FROM VectorClockEntry WHERE vector_clock_id = ?1",
            )?;
            let mut rows = vc_stmt.query(rusqlite::params![vector_clock_id])?;
            while let Some(vc_row) = rows.next()? {
                let site_id: String = vc_row.get(0)?;
                let value: i64 = vc_row.get(1)?;
                clock_map.insert(site_id, value);
            }
            log::info!(
                "{} -> {} | {:.2} | time: {} | node: {} | msg: {:?} | vector_clock: {:?}",
                from,
                to,
                amount,
                time,
                node,
                msg,
                clock_map
            );
        }
        Ok(())
    }
}
