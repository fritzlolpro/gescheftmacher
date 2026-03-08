// #![allow(unused)]
pub mod datagetter {
    use crate::from_str;
    use crate::goonmetrics::goonmetrics::*;
    use chrono::{Duration, Utc};
    use error_chain::error_chain;
    use reqwest;
    use rusqlite::{params, Connection as SQL_Connection, Result as SQL_Result, Statement};
    use serde::{Deserialize, Serialize};
    use std::path::PathBuf;
    use std::sync::mpsc;
    use struct_field_names_as_array::FieldNamesAsSlice;

    use tokio::task;

    error_chain! {
        foreign_links {
            Io(std::io::Error);
            HttpRequest(reqwest::Error);
        }
    }

    const MAX_GOONMETRICS_ID_QUANTITY: usize = 49;

    #[derive(Debug, PartialEq, Clone, FieldNamesAsSlice, Deserialize, Serialize)]
    pub struct ExtendedItemData {
        pub type_id: i32,
        pub type_volume: f32,
        pub timestamp: i64,
        pub type_name: String,
        pub jita_trade_data: TradeData,
        pub jita_buy_with_tax: f64,
        pub abroad_trade_data: TradeData,
        pub abroad_stocked_ratio: f64,
        pub shipping_price: f64,
        pub abroad_sell_taxed: f64,
        pub abroad_avg_daily: f64,
        pub profit_jita_buy_per_unit: f64,
        pub profit_jita_buy_daily: f64,
        pub margin_jita_buy: f64,
        pub money_freeze_buy: f64,
        pub freeze_rate: f64,
    }

    #[derive(Debug, PartialEq)]
    pub struct ItemDataFromDb {
        pub type_id: i32,
        pub type_volume: f32,
    }
    #[derive(Debug, PartialEq, Clone, FieldNamesAsSlice, Deserialize, Serialize)]
    pub struct TradeData {
        pub updated: String,
        pub weekly_movement: f64,
        pub buy_max: f64,
        pub buy_listed: i64,
        pub sell_min: f64,
        pub sell_listed: i64,
    }

    use std::collections::HashMap;

    pub struct ItemHistory {
        pub data: HashMap<i32, Vec<ExtendedItemData>>,
    }

    impl ItemHistory {
        pub fn new() -> Self {
            ItemHistory {
                data: HashMap::new(),
            }
        }

        pub fn add_item_data(&mut self, id: i32, item_data: ExtendedItemData) {
            self.data.entry(id).or_insert_with(Vec::new).push(item_data);
        }

        pub fn add_item_data_vec(&mut self, id: i32, item_data_vec: Vec<ExtendedItemData>) {
            self.data
                .entry(id)
                .or_insert_with(Vec::new)
                .extend(item_data_vec);
        }

        pub fn get_item_data(&self, id: i32) -> Option<&Vec<ExtendedItemData>> {
            self.data.get(&id)
        }

        pub fn all_ids_present_and_recent(&self, ids: &[i32], max_age_minutes: i64) -> bool {
            let now = Utc::now().timestamp();
            let max_age = Duration::minutes(max_age_minutes);

            for &id in ids {
                if let Some(data_vec) = self.get_item_data(id) {
                    if let Some(max_timestamp) = data_vec.iter().map(|data| data.timestamp).max() {
                        println!("max_timestamp: {}", max_timestamp);
                        println!("now: {}", now);
                        println!("max_age: {}", max_age.num_seconds());
                        if now - max_timestamp > max_age.num_seconds() {
                            return false;
                        }
                    } else {
                        return false;
                    }
                } else {
                    return false;
                }
            }
            true
        }

        pub fn get_most_recent_item_data(&self) -> Vec<ExtendedItemData> {
            self.data
                .values()
                .filter_map(|data_vec| data_vec.iter().max_by_key(|data| data.timestamp).cloned())
                .collect()
        }
    }

    #[derive(Debug, PartialEq, Clone)]
    pub struct ItemData {
        pub type_id: i32,
        pub type_volume: f32,
        pub type_name: String,
        pub jita_trade_data: Option<TradeData>,
        pub abroad_trade_data: Option<TradeData>,
    }

    pub trait DatabaseConnection {
        fn open(db_path: PathBuf) -> SQL_Result<Self>
        where
            Self: Sized;

        fn open_in_memory() -> SQL_Result<Self>
        where
            Self: Sized;

        fn prepare(&self, sql: &str) -> SQL_Result<Statement<'_>>;
        fn execute_no_params(&self, sql: &str) -> SQL_Result<usize>;
        fn get_stored_type_data(&self, name: &str) -> SQL_Result<ItemDataFromDb>;
        fn get_stored_type_volume_packed(&self, type_id: i32) -> SQL_Result<f32>;
        fn get_tradable_item_names(&self) -> SQL_Result<Vec<String>>;
        fn create_extended_item_data_table(&self) -> SQL_Result<()>;
        fn store_extended_item_data(&self, extended_item_data: ExtendedItemData) -> SQL_Result<()>;
        fn get_stored_extended_item_data(
            &self,
            extended_item_id: i32,
        ) -> SQL_Result<Vec<ExtendedItemData>>;
    }

    #[derive(Debug)]
    pub struct SqlLiteConnection(SQL_Connection);

    impl DatabaseConnection for SqlLiteConnection {
        fn open(db_path: PathBuf) -> SQL_Result<Self> {
            Ok(SqlLiteConnection(SQL_Connection::open(db_path)?))
        }

        fn open_in_memory() -> SQL_Result<Self>
        where
            Self: Sized,
        {
            Ok(SqlLiteConnection(SQL_Connection::open_in_memory()?))
        }

        fn prepare(&self, sql: &str) -> SQL_Result<Statement<'_>> {
            let mut res = self.0.prepare(sql)?;

            Ok(res)
        }

        fn execute_no_params(&self, sql: &str) -> SQL_Result<usize> {
            self.0.execute(sql, [])
        }

        fn get_stored_type_data(&self, type_name: &str) -> SQL_Result<ItemDataFromDb> {
            let mut stmt = self.0.prepare(
                "SELECT typeID, volume FROM invTypes
                WHERE typeName = :type_name",
            )?;
            let mut rows = stmt.query(&[(":type_name", type_name)])?;

            let mut names: Vec<f32> = Vec::new();
            while let Some(row) = rows.next()? {
                names.push(row.get(0)?);
                names.push(row.get(1)?);
            }
            let result = ItemDataFromDb {
                type_id: names[0] as i32,
                type_volume: names[1],
            };

            Ok(result)
        }

        fn create_extended_item_data_table(&self) -> SQL_Result<()> {
            self.0.execute(
                "CREATE TABLE IF NOT EXISTS extended_item_data (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                type_id INTEGER NOT NULL, 
                timestamp DATETIME DEFAULT CURRENT_TIMESTAMP, 
                type_volume REAL NOT NULL,  -- Assuming 'type_volume' is a REAL value
                type_name TEXT NOT NULL, 
                jita_trade_data JSON NOT NULL,
                abroad_trade_data JSON NOT NULL,
                jita_buy_with_tax REAL NOT NULL,
                abroad_stocked_ratio REAL NOT NULL,
                shipping_price REAL NOT NULL,
                abroad_sell_taxed REAL NOT NULL,
                abroad_avg_daily REAL NOT NULL,
                profit_jita_buy_per_unit REAL NOT NULL,
                profit_jita_buy_daily REAL NOT NULL,
                margin_jita_buy REAL NOT NULL,
                money_freeze_buy REAL NOT NULL,
                freeze_rate REAL NOT NULL)",
                [],
            )?;

            Ok(())
        }

        fn store_extended_item_data(&self, extended_item_data: ExtendedItemData) -> SQL_Result<()> {
            let mut stmt = self.0.prepare(
                "INSERT INTO extended_item_data (type_id, timestamp, type_volume, type_name, jita_trade_data, abroad_trade_data, jita_buy_with_tax, abroad_stocked_ratio, shipping_price, abroad_sell_taxed, abroad_avg_daily, profit_jita_buy_per_unit, profit_jita_buy_daily, margin_jita_buy, money_freeze_buy, freeze_rate)
                VALUES (:type_id, :timestamp, :type_volume, :type_name, :jita_trade_data, :abroad_trade_data, :jita_buy_with_tax, :abroad_stocked_ratio, :shipping_price, :abroad_sell_taxed, :abroad_avg_daily, :profit_jita_buy_per_unit, :profit_jita_buy_daily, :margin_jita_buy, :money_freeze_buy, :freeze_rate)",
            )?;

            let jtd = serde_json::to_string(&extended_item_data.jita_trade_data).unwrap();
            let atd = serde_json::to_string(&extended_item_data.abroad_trade_data).unwrap();

            let params = rusqlite::params![
                extended_item_data.type_id,
                extended_item_data.timestamp,
                extended_item_data.type_volume,
                extended_item_data.type_name,
                jtd,
                atd,
                extended_item_data.jita_buy_with_tax,
                extended_item_data.abroad_stocked_ratio,
                extended_item_data.shipping_price,
                extended_item_data.abroad_sell_taxed,
                extended_item_data.abroad_avg_daily,
                extended_item_data.profit_jita_buy_per_unit,
                extended_item_data.profit_jita_buy_daily,
                extended_item_data.margin_jita_buy,
                extended_item_data.money_freeze_buy,
                extended_item_data.freeze_rate,
            ];

            stmt.execute(params)?;

            Ok(())
        }

        fn get_stored_extended_item_data(&self, type_id: i32) -> SQL_Result<Vec<ExtendedItemData>> {
            let mut stmt = self
                .0
                .prepare("SELECT * FROM extended_item_data WHERE type_id = ?")?;

            println!("+++++self: {:?}", self.0);
            println!("trying to get data for type_id: {}", type_id);
            let mut rows = stmt.query(params![type_id])?;

            let mut result: Vec<ExtendedItemData> = Vec::new();

            while let Some(row) = rows.next()? {
                let type_id = row.get(1)?;
                let timestamp = row.get(2)?;
                let type_volume = row.get(3)?;
                let type_name = row.get(4)?;
                let jita_trade_data: String = row.get(5)?;
                let abroad_trade_data: String = row.get(6)?;
                let jita_buy_with_tax = row.get(7)?;
                let abroad_stocked_ratio = row.get(8)?;
                let shipping_price = row.get(9)?;
                let abroad_sell_taxed = row.get(10)?;
                let abroad_avg_daily = row.get(11)?;
                let profit_jita_buy_per_unit = row.get(12)?;
                let profit_jita_buy_daily = row.get(13)?;
                let margin_jita_buy = row.get(14)?;
                let money_freeze_buy = row.get(15)?;
                let freeze_rate = row.get(16)?;

                let jtd: TradeData = serde_json::from_str(&jita_trade_data).unwrap();
                let atd: TradeData = serde_json::from_str(&abroad_trade_data).unwrap();

                let item_data = ExtendedItemData {
                    type_id,
                    timestamp,
                    type_volume,
                    type_name,
                    jita_trade_data: jtd,
                    abroad_trade_data: atd,
                    jita_buy_with_tax,
                    abroad_stocked_ratio,
                    shipping_price,
                    abroad_sell_taxed,
                    abroad_avg_daily,
                    profit_jita_buy_per_unit,
                    profit_jita_buy_daily,
                    margin_jita_buy,
                    money_freeze_buy,
                    freeze_rate,
                };
                result.push(item_data);
            }

            Ok(result)
        }

        fn get_stored_type_volume_packed(&self, type_id: i32) -> SQL_Result<f32> {
            let mut stmt = self.0.prepare(
                "
                 select volume from invVolumes
                 where typeID = ?1
                ",
            )?;

            let mut rows = stmt.query(rusqlite::params![type_id])?;

            let mut res = Vec::new();
            while let Some(row) = rows.next()? {
                res.push(row.get(0)?);
            }

            match res.len() {
                0 => Err(rusqlite::Error::InvalidQuery),
                _ => Ok(res[0]),
            }
        }

        fn get_tradable_item_names(&self) -> SQL_Result<Vec<String>> {
            let mut stmt = self.0.prepare(
                "SELECT typeName FROM invTypes
             WHERE marketGroupID IS NOT NULL AND description <> ''",
            )?;

            let mut rows = stmt.query([])?;

            let mut names: Vec<String> = Vec::new();
            while let Some(row) = rows.next()? {
                names.push(row.get(0)?);
            }

            Ok(names)
        }
    }

    pub fn create_table_and_store_data(extended_item_data: ExtendedItemData) {
        let db_path = PathBuf::from("src/gescheftmacher.db");
        let connection = SqlLiteConnection::open(db_path).unwrap();

        if let Err(e) = connection.create_extended_item_data_table() {
            eprintln!("Error creating table: {:?}", e);
        }
        if let Err(e) = connection.store_extended_item_data(extended_item_data) {
            eprintln!("Error storing data: {:?}", e);
        }
    }

    pub fn get_stored_items_history(item_ids: &Vec<i32>) -> ItemHistory {
        let mut item_history = ItemHistory::new();
        let db_path = PathBuf::from("src/gescheftmacher.db");

        match SqlLiteConnection::open(db_path) {
            Ok(connection) => {
                // Ensure the table exists before querying
                if let Err(e) = connection.create_extended_item_data_table() {
                    eprintln!("Error creating table: {:?}", e);
                }

                println!("Database connection established.");

                for id in item_ids {
                    println!("Fetching data for ID: {}", id);
                    match connection.get_stored_extended_item_data(*id) {
                        Ok(stored) => {
                            println!("Data retrieved for ID: {}", id);
                            item_history.add_item_data_vec(*id, stored);
                        }
                        Err(e) => {
                            eprintln!("Error retrieving data for ID {}: {:?}", id, e);
                        }
                    }
                }
            }
            Err(e) => {
                eprintln!("Error opening database: {:?}", e);
            }
        }

        item_history
    }

    pub fn get_item_data_from_db(names: Vec<String>) -> Vec<ItemData> {
        let db_path = PathBuf::from("src/eve.db");
        let connection = SqlLiteConnection::open(db_path).unwrap();

        names
            .into_iter()
            .map(|name| {
                let stored = connection.get_stored_type_data(&name).unwrap();

                let item_id = stored.type_id;
                let packed_volume = connection.get_stored_type_volume_packed(item_id);

                let volume: f32;
                if let Err(_err) = packed_volume {
                    volume = stored.type_volume;
                } else {
                    volume = packed_volume.unwrap();
                }

                let result = ItemData {
                    type_name: name.to_string(),
                    type_id: item_id,
                    type_volume: volume,
                    jita_trade_data: None,
                    abroad_trade_data: None,
                };
                return result;
            })
            .collect()
    }

    pub async fn get_item_data_from_api(
        station_id: &str,
        item_ids: &Vec<i32>,
    ) -> Result<Vec<PriceData>> {
        let item_id_batches = split_large_id_bulks(item_ids, MAX_GOONMETRICS_ID_QUANTITY);

        let (tx, rx) = mpsc::channel();

        for item_id_batch in item_id_batches {
            let transmitter = tx.clone();
            let st_id = station_id.to_owned().clone();
            task::spawn(async move {
                let item_ids = &item_id_batch
                    .into_iter()
                    .map(|id| id.to_string() + ",")
                    .collect::<String>();

                let jita_url = format!(
                    "https://goonmetrics.\
                    apps.goonswarm.org/api/price_data/\
                    ?station_id={st_id}&type_id={item_ids}"
                );

                async fn fetcher(url: &str) -> Result<PriceData> {
                    let res = reqwest::get(url).await?;
                    let body = res.text().await?;

                    let data: Goonmetrics = from_str(&body).unwrap();
                    let pd = data.price_data;
                    return Ok(pd);
                }

                let data: PriceData = fetcher(&jita_url).await.unwrap();
                transmitter.send(data).unwrap();
            })
            .await
            .unwrap();
        }
        drop(tx);

        let mut result = vec![];
        for res in rx {
            for data in res.types {
                result.push(data);
            }
        }

        let data = PriceData { types: result };
        return Ok(vec![data]);
    }

    // ===== Watchlist =====

    #[derive(Debug, Clone)]
    pub struct WatchlistItem {
        pub id: i64,
        pub type_id: i32,
        pub type_name: String,
        pub type_volume: f32,
        pub group_name: String,
        pub active: bool,
    }

    fn open_gm_db() -> SqlLiteConnection {
        SqlLiteConnection::open(PathBuf::from("src/gescheftmacher.db"))
            .expect("Cannot open gescheftmacher.db")
    }

    fn open_eve_db() -> SqlLiteConnection {
        SqlLiteConnection::open(PathBuf::from("src/eve.db"))
            .expect("Cannot open eve.db")
    }

    pub fn init_watchlist_tables() {
        let conn = open_gm_db();
        conn.0
            .execute_batch(
                "CREATE TABLE IF NOT EXISTS watchlist_groups (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    name TEXT NOT NULL UNIQUE,
                    active INTEGER NOT NULL DEFAULT 1
                );
                CREATE TABLE IF NOT EXISTS watchlist_items (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    type_id INTEGER NOT NULL,
                    type_name TEXT NOT NULL UNIQUE,
                    type_volume REAL NOT NULL,
                    group_id INTEGER REFERENCES watchlist_groups(id),
                    active INTEGER NOT NULL DEFAULT 1
                );",
            )
            .expect("Failed to create watchlist tables");
    }

    pub fn is_watchlist_empty() -> bool {
        let conn = open_gm_db();
        let count: i64 = conn
            .0
            .query_row("SELECT COUNT(*) FROM watchlist_items", [], |row| row.get(0))
            .unwrap_or(0);
        count == 0
    }

    pub fn import_csv_to_watchlist(csv_path: &std::path::Path, group_name: &str) {
        let gm_conn = open_gm_db();
        let eve_conn = open_eve_db();

        gm_conn
            .0
            .execute(
                "INSERT OR IGNORE INTO watchlist_groups (name) VALUES (?1)",
                params![group_name],
            )
            .expect("Failed to insert group");

        let group_id: i64 = gm_conn
            .0
            .query_row(
                "SELECT id FROM watchlist_groups WHERE name = ?1",
                params![group_name],
                |row| row.get(0),
            )
            .expect("Failed to get group id");

        let content = std::fs::read_to_string(csv_path)
            .unwrap_or_else(|e| { eprintln!("Failed to read CSV {:?}: {:?}", csv_path, e); String::new() });

        let mut seen_names = std::collections::HashSet::new();

        for (i, line) in content.lines().enumerate() {
            if i == 0 {
                continue; // skip header
            }
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            let parts: Vec<&str> = trimmed.splitn(3, ',').collect();
            let name = parts[0].trim().to_string();
            if name.is_empty() || seen_names.contains(&name) {
                continue;
            }
            seen_names.insert(name.clone());

            let csv_type_id: i32 = parts
                .get(1)
                .and_then(|s| s.trim().parse().ok())
                .unwrap_or(0);
            let csv_volume: f32 = parts
                .get(2)
                .and_then(|s| s.trim().parse().ok())
                .unwrap_or(0.0);

            let (type_id, volume) = if csv_type_id == 0 || csv_volume == 0.0 {
                match eve_conn.get_stored_type_data(&name) {
                    Ok(data) => {
                        let vol = if csv_volume == 0.0 {
                            eve_conn
                                .get_stored_type_volume_packed(data.type_id)
                                .unwrap_or(data.type_volume)
                        } else {
                            csv_volume
                        };
                        let tid = if csv_type_id == 0 { data.type_id } else { csv_type_id };
                        (tid, vol)
                    }
                    Err(e) => {
                        eprintln!("Cannot find '{}' in eve.db: {:?}", name, e);
                        continue;
                    }
                }
            } else {
                (csv_type_id, csv_volume)
            };

            if let Err(e) = gm_conn.0.execute(
                "INSERT OR IGNORE INTO watchlist_items (type_id, type_name, type_volume, group_id) VALUES (?1, ?2, ?3, ?4)",
                params![type_id, name, volume, group_id],
            ) {
                eprintln!("Failed to insert '{}': {:?}", name, e);
            }
        }
        println!("Imported watchlist from {:?}", csv_path);
    }

    pub fn import_all_csv_watchlists() {
        let csv_dir = PathBuf::from("../watchlist");
        if !csv_dir.exists() {
            eprintln!("Watchlist CSV dir not found at {:?}", std::fs::canonicalize("../watchlist").unwrap_or(csv_dir));
            return;
        }
        match std::fs::read_dir(&csv_dir) {
            Ok(entries) => {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.extension().and_then(|e| e.to_str()) == Some("csv") {
                        let group = path
                            .file_stem()
                            .and_then(|s| s.to_str())
                            .unwrap_or("general")
                            .to_string();
                        println!("Importing group '{}' from {:?}", group, path);
                        import_csv_to_watchlist(&path, &group);
                    }
                }
            }
            Err(e) => eprintln!("Cannot read watchlist dir: {:?}", e),
        }
    }

    pub fn ensure_watchlist_initialized() {
        init_watchlist_tables();
        if is_watchlist_empty() {
            println!("Watchlist empty — importing from CSV files...");
            import_all_csv_watchlists();
        }
    }

    pub fn get_watchlist_as_item_data() -> Vec<ItemData> {
        let conn = open_gm_db();
        let mut stmt = conn
            .0
            .prepare(
                "SELECT type_id, type_name, type_volume FROM watchlist_items WHERE active = 1",
            )
            .expect("Failed to prepare watchlist query");

        stmt.query_map([], |row| {
            Ok(ItemData {
                type_id: row.get(0)?,
                type_name: row.get(1)?,
                type_volume: row.get(2)?,
                jita_trade_data: None,
                abroad_trade_data: None,
            })
        })
        .expect("Failed to query watchlist")
        .filter_map(|r| r.ok())
        .collect()
    }

    // ===== end Watchlist =====

    pub fn split_large_id_bulks(item_ids: &Vec<i32>, split_treshold: usize) -> Vec<Vec<i32>> {
        if item_ids.len() <= split_treshold {
            return vec![item_ids.to_owned()];
        } else {
            let mut result: Vec<Vec<i32>> = vec![];
            let mut batch = vec![];

            for i in 0..item_ids.len() {
                if batch.len() == split_treshold {
                    result.push(batch.clone());
                    batch = vec![];
                }
                batch.push(item_ids[i]);
                if i == item_ids.len() - 1 {
                    result.push(batch.clone());
                }
            }

            return result;
        }
    }

    pub fn merge_trade_data(
        items_data: &Vec<ItemData>,
        jita_trade_data: &Vec<PriceData>,
        abroad_trade_data: &Vec<PriceData>,
    ) -> Vec<ItemData> {
        let result: Vec<_> = items_data
            .into_iter()
            .map(|item| {
                let mut enriched_item = ItemData {
                    type_name: item.type_name.clone(),
                    type_id: item.type_id,
                    type_volume: item.type_volume,
                    jita_trade_data: None,
                    abroad_trade_data: None,
                };
                let id = item.type_id;
                let jt = &jita_trade_data[0].types;

                let item_jita_trade_data = jt.into_iter().find(|jtd| match jtd {
                    Types::Type(item_type) => {
                        return item_type.id == id;
                    }
                });

                match item_jita_trade_data {
                    Some(&Types::Type(ref item_type)) => {
                        enriched_item.jita_trade_data = Some(TradeData {
                            updated: item_type.updated.clone(),
                            weekly_movement: item_type
                                .all
                                .weekly_movement
                                .parse::<f64>()
                                .expect("Fail to parse"),
                            sell_listed: item_type
                                .sell
                                .listed
                                .parse::<i64>()
                                .expect("Fail to parse"),
                            sell_min: item_type.sell.min.parse::<f64>().expect("Fail to parse"),
                            buy_listed: item_type.buy.listed.parse::<i64>().expect("Fail to parse"),
                            buy_max: item_type.buy.max.parse::<f64>().expect("Fail to parse"),
                        })
                    }
                    _ => {
                        let en_item_jita_t_d = enriched_item.jita_trade_data;
                        panic!(
                            "fail to compare\n
                    JITA TRADE DATA:\n {:?}\n
                    ITEM FROM DB DATA:\n {:?}
                    ",
                            item_jita_trade_data, en_item_jita_t_d
                        )
                    }
                }

                let at = &abroad_trade_data[0].types;
                let item_abroad_trade_data = at.into_iter().find(|atd| match atd {
                    Types::Type(item_type) => {
                        return item_type.id == id;
                    }
                });

                match item_abroad_trade_data {
                    Some(&Types::Type(ref item_type)) => {
                        enriched_item.abroad_trade_data = Some(TradeData {
                            updated: item_type.updated.clone(),
                            weekly_movement: item_type
                                .all
                                .weekly_movement
                                .parse::<f64>()
                                .expect("Fail to parse"),
                            sell_listed: item_type
                                .sell
                                .listed
                                .parse::<i64>()
                                .expect("Fail to parse"),
                            sell_min: item_type.sell.min.parse::<f64>().expect("Fail to parse"),
                            buy_listed: item_type.buy.listed.parse::<i64>().expect("Fail to parse"),
                            buy_max: item_type.buy.max.parse::<f64>().expect("Fail to parse"),
                        })
                    }
                    _ => panic!("Terrible wrong shit"),
                }

                return enriched_item;
            })
            .collect();

        return result;
    }
}
