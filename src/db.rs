#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Transaction {
    pub from_user: String,
    pub to_user: String,
    pub amount: f64,
    pub lamport_time: i64,
    pub source_node: String,
    pub optional_msg: Option<String>,
    pub vector_clock: std::collections::HashMap<String, i64>,
}

#[cfg(feature = "server")]
lazy_static::lazy_static! {
    static ref DB_CONN: std::sync::Mutex<rusqlite::Connection> =
        std::sync::Mutex::new(rusqlite::Connection::open("peillute.db").unwrap());
}

#[cfg(feature = "server")]
const NULL: &str = "NULL";

#[cfg(feature = "server")]
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

#[cfg(feature = "server")]
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
#[cfg(feature = "server")]
pub fn drop_tables() -> rusqlite::Result<()> {
    {
        let conn = DB_CONN.lock().unwrap();
        conn.execute("DROP TABLE IF EXISTS Transactions;", [])?;
        conn.execute("DROP TABLE IF EXISTS User;", [])?;
    }
    log::debug!("Tables dropped successfully.");
    Ok(())
}

#[cfg(feature = "server")]
pub fn user_exists(name: &str) -> rusqlite::Result<bool> {
    {
        use rusqlite::params;
        let conn = DB_CONN.lock().unwrap();
        let mut stmt = conn.prepare("SELECT EXISTS(SELECT 1 FROM User WHERE unique_name = ?1)")?;
        let exists: bool = stmt.query_row(params![name], |row| row.get(0))?;
        Ok(exists)
    }
}

#[cfg(feature = "server")]
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

#[cfg(feature = "server")]
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

#[cfg(feature = "server")]
pub fn update_solde(name: &str) -> rusqlite::Result<()> {
    use rusqlite::params;

    if !user_exists(name)? {
        let err = rusqlite::Error::SqliteFailure(
            rusqlite::ffi::Error::new(rusqlite::ffi::ErrorCode::Unknown as i32),
            Some(format!("User '{}' does not exist.", name).into())
        );

        return Err(err);
    }
    let solde = calculate_solde(name)?;
    {
        let conn = DB_CONN.lock().unwrap();
        conn.execute(
            "UPDATE User SET solde = ?1 WHERE unique_name = ?2",
            params![solde, name],
        )?;
        log::debug!("Updated solde for {} to {}", name, solde);
        Ok(())
    }
}

#[cfg(feature = "server")]
pub fn ensure_user(name: &str) -> rusqlite::Result<()> {
    if name != NULL && !user_exists(name)? {
        create_user(name)?;
    }
    Ok(())
}

#[cfg(feature = "server")]
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
        let err = rusqlite::Error::SqliteFailure(
            rusqlite::ffi::Error::new(rusqlite::ffi::ErrorCode::Unknown as i32),
            Some(format!("Insufficient funds: '{}' has less than {}.",
                         from_user, amount).into())
        );

        return Err(err);
    }

    ensure_user(from_user)?;
    ensure_user(to_user)?;

    log::debug!(
        "Creating transaction from {} to {} with amount {}",
        from_user,
        to_user,
        amount
    );

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

#[cfg(feature = "server")]
pub fn deposit(
    user: &str,
    amount: f64,
    lamport_time: &i64,
    source_node: &str,
    vector_clock: &std::collections::HashMap<String, i64>,
) -> rusqlite::Result<()> {
    if !user_exists(user)? {
        let err = rusqlite::Error::SqliteFailure(
            rusqlite::ffi::Error::new(rusqlite::ffi::ErrorCode::Unknown as i32),
            Some(format!("Unknown User: {}", user).into())
        );

        return Err(err);
    }
    log::debug!("Depositing {} to {}", amount, user);

    if let Err(e) = create_transaction(NULL, user, amount, lamport_time, source_node, "Deposit") {
        log::error!("Error while creating deposit transaction: {}", e);
        return Err(e);
    }
    Ok(())

    if amount < 0.0 {
        let err = rusqlite::Error::SqliteFailure(
            rusqlite::ffi::Error::new(rusqlite::ffi::ErrorCode::Unknown as i32),
            Some(format!("Negative deposit amount: {}", amount).into())
        );

        return Err(err);
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

#[cfg(feature = "server")]
pub fn withdraw(
    user: &str,
    amount: f64,
    lamport_time: &i64,
    source_node: &str,
    vector_clock: &std::collections::HashMap<String, i64>,
) -> rusqlite::Result<()> {
    if amount < 0.0 {
        let err = rusqlite::Error::SqliteFailure(
            rusqlite::ffi::Error::new(rusqlite::ffi::ErrorCode::Unknown as i32),
            Some(format!("Negative withdrawal amount: {}", amount).into())
        );

        return Err(err);
    }
    if !user_exists(user)? {
        let err = rusqlite::Error::SqliteFailure(
            rusqlite::ffi::Error::new(rusqlite::ffi::ErrorCode::Unknown as i32),
            Some(format!("Unknown user: {}", user).into())
        );

        return Err(err);
    }
    if calculate_solde(user)? < amount {
        let err = rusqlite::Error::SqliteFailure(
            rusqlite::ffi::Error::new(rusqlite::ffi::ErrorCode::Unknown as i32),
            Some(format!("User {} not enough money", user).into())
        );

        return Err(err);
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

#[allow(unused)]
#[cfg(feature = "server")]
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

#[cfg(feature = "server")]
pub fn has_been_refunded(
    transac_time: i64,
    node: &str,
) -> rusqlite::Result<bool> {
    use rusqlite::params;
    {
        let conn = DB_CONN.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT EXISTS(SELECT 1 FROM Transactions WHERE optional_msg = ?1)",
        )?;
    
        let optional_msg = format!("Refund transaction {}-{}", node, transac_time);
        let exists: bool = stmt.query_row(params![optional_msg], |row| row.get(0))?;
    
        Ok(exists)
    }
}

#[cfg(feature = "server")]
pub fn refund_transaction(
    transac_time: i64,
    node: &str,
    lamport_time: &i64,
    source_node: &str,
    vector_clock: &std::collections::HashMap<String, i64>,
) -> rusqlite::Result<()> {
    if let Some(tx) = get_transaction(transac_time, node)? {
        if calculate_solde(&tx.to_user)? < tx.amount {
            let err = rusqlite::Error::SqliteFailure(
                rusqlite::ffi::Error::new(rusqlite::ffi::ErrorCode::Unknown as i32),
                Some(format!("User {} has not enough money to give back", &tx.to_user).into())
            );
            return Err(err);
        }

        if tx.optional_msg.is_some() && tx.optional_msg.unwrap().starts_with("Refund transaction") {
            let err = rusqlite::Error::SqliteFailure(
                rusqlite::ffi::Error::new(rusqlite::ffi::ErrorCode::Unknown as i32),
                Some(format!("Transaction {}-{} is a refund transaction", node, transac_time).into())
            );
            return Err(err);
        }

        if has_been_refunded(transac_time, node)? {
            let err = rusqlite::Error::SqliteFailure(
                rusqlite::ffi::Error::new(rusqlite::ffi::ErrorCode::Unknown as i32),
                Some(format!("Transaction {}-{} already refunded", node, transac_time).into())
            );
            return Err(err);
        }

        create_transaction(
            &tx.to_user,
            &tx.from_user,
            tx.amount,
            lamport_time,
            source_node,
            &format!("Refund transaction {}-{}", node, transac_time),
            vector_clock,
        )?;
    } else {
        let err = rusqlite::Error::SqliteFailure(
            rusqlite::ffi::Error::new(rusqlite::ffi::ErrorCode::Unknown as i32),
            Some(format!("No transaction found at time {} from node {}",
                         transac_time,
                         node).into())
        );

        return Err(err);
    }
    Ok(())
}

#[cfg(feature = "server")]
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

#[cfg(feature = "server")]
pub fn print_users() -> rusqlite::Result<()> {
    {
        let conn = DB_CONN.lock().unwrap();
        let mut stmt = conn.prepare("SELECT unique_name, solde FROM User")?;
        let users = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, f64>(1)?))
        })?;

        println!("-- Users --");
        for user in users {
            let (name, solde) = user?;
            println!("{}: {:.2}", name, solde);
        }
        Ok(())
    }
}

#[cfg(feature = "server")]
pub fn get_users() -> rusqlite::Result<Vec<String>> {
    {
        let conn = DB_CONN.lock().unwrap();
        let mut stmt = conn.prepare("SELECT unique_name FROM User")?;
        let users = stmt.query_map([], |row| Ok(row.get::<_, String>(0)?))?;
        let mut users_vec = Vec::new();
        for user in users {
            users_vec.push(user?);
        }
        Ok(users_vec)
    }
}

#[cfg(feature = "server")]
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

        println!("-- Transactions --");
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

#[cfg(feature = "server")]
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

        println!("-- Transactions for user {} --", name);
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

#[cfg(feature = "server")]
pub fn get_transactions_for_user(name: &str) -> rusqlite::Result<Vec<Transaction>> {
    use rusqlite::params;
    {
        let conn = DB_CONN.lock().unwrap();
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

        let mut txs_vec = Vec::new();
        for tx in txs {
            let (from, to, amount, time, node, msg) = tx?;
            txs_vec.push(Transaction {
                from_user: from,
                to_user: to,
                amount: amount,
                lamport_time: time,
                source_node: node,
                optional_msg: msg,
            });
        }
        Ok(txs_vec)
    }

#[cfg(feature = "server")]
pub fn get_local_db_state() -> rusqlite::Result<std::collections::HashMap<String, f64>> {
    let mut state = std::collections::HashMap::new();
    {
        let conn = DB_CONN.lock().unwrap();
        let mut stmt = conn.prepare("SELECT unique_name, solde FROM User")?;
        let users = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, f64>(1)?))
        })?;

        for user in users {
            let (name, solde) = user?;
            state.insert(name, solde);
        }
    }
    Ok(state)
}

#[cfg(feature = "server")]
pub fn get_local_transaction_log() -> rusqlite::Result<Vec<Transaction>> {
    let conn = DB_CONN.lock().unwrap();
    let mut stmt = conn.prepare(
        "SELECT from_user, to_user, amount, lamport_time, source_node, optional_msg, vector_clock_id
         FROM Transactions")?;
    let rows = stmt.query_map([], |row| {
        Ok(Transaction {
            from_user: row.get(0)?,
            to_user: row.get(1)?,
            amount: row.get(2)?,
            lamport_time: row.get(3)?,
            source_node: row.get(4)?,
            optional_msg: row.get(5)?,
            vector_clock: std::collections::HashMap::new(),
        })
    })?;

    let mut out = Vec::new();
    for r in rows {
        out.push(r?);
    }
    Ok(out)
}
