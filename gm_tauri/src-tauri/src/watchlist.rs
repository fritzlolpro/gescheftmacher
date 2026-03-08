pub mod watchlist {
    use crate::datagetter::datagetter::{DatabaseConnection, ItemData, SqlLiteConnection};
    use rusqlite::params;
    use std::path::PathBuf;

    fn open_gm_db() -> SqlLiteConnection {
        SqlLiteConnection::open(PathBuf::from("src/gescheftmacher.db"))
            .expect("Cannot open gescheftmacher.db")
    }

    fn open_eve_db() -> SqlLiteConnection {
        SqlLiteConnection::open(PathBuf::from("src/eve.db")).expect("Cannot open eve.db")
    }

    /// Парсит одно поле CSV: снимает кавычки и убирает запятые-разделители тысяч.
    fn parse_csv_field(raw: &str) -> String {
        let trimmed = raw.trim();
        if trimmed.starts_with('"') && trimmed.ends_with('"') {
            trimmed[1..trimmed.len() - 1].replace(',', "")
        } else {
            trimmed.replace(',', "")
        }
    }

    /// Парсит строку CSV с учётом quoted-полей. Возвращает первые `limit` полей.
    fn parse_csv_line(line: &str, limit: usize) -> Vec<String> {
        let mut fields = Vec::new();
        let mut in_quotes = false;
        let mut current = String::new();

        for c in line.chars() {
            match c {
                '"' => in_quotes = !in_quotes,
                ',' if !in_quotes => {
                    fields.push(parse_csv_field(&current));
                    current.clear();
                    if fields.len() >= limit {
                        return fields;
                    }
                }
                _ => current.push(c),
            }
        }
        if fields.len() < limit {
            fields.push(parse_csv_field(&current));
        }
        fields
    }

    pub fn init_watchlist_tables() {
        let conn = open_gm_db();
        conn.0
            .execute_batch(
                "CREATE TABLE IF NOT EXISTS watchlist_groups (
                    id   INTEGER PRIMARY KEY AUTOINCREMENT,
                    name TEXT    NOT NULL UNIQUE
                );
                CREATE TABLE IF NOT EXISTS watchlist_items (
                    id          INTEGER PRIMARY KEY AUTOINCREMENT,
                    type_id     INTEGER NOT NULL,
                    type_name   TEXT    NOT NULL UNIQUE,
                    type_volume REAL    NOT NULL,
                    group_id    INTEGER REFERENCES watchlist_groups(id),
                    active      INTEGER NOT NULL DEFAULT 1
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

        let content = std::fs::read_to_string(csv_path).unwrap_or_else(|e| {
            eprintln!("Failed to read CSV {:?}: {:?}", csv_path, e);
            String::new()
        });

        let mut seen_names = std::collections::HashSet::new();

        for (i, line) in content.lines().enumerate() {
            if i == 0 {
                continue; // пропускаем заголовок
            }
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }

            let fields = parse_csv_line(trimmed, 3);
            let name = fields.get(0).map(|s| s.trim().to_string()).unwrap_or_default();

            if name.is_empty() || seen_names.contains(&name) {
                continue;
            }
            seen_names.insert(name.clone());

            let csv_type_id: i32 = fields
                .get(1)
                .and_then(|s| s.trim().parse().ok())
                .unwrap_or(0);
            let csv_volume: f32 = fields
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
                        let tid = if csv_type_id == 0 {
                            data.type_id
                        } else {
                            csv_type_id
                        };
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
                "INSERT OR IGNORE INTO watchlist_items (type_id, type_name, type_volume, group_id)
                 VALUES (?1, ?2, ?3, ?4)",
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
            eprintln!("Watchlist CSV dir not found: {:?}", csv_dir);
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
                "SELECT type_id, type_name, type_volume
                 FROM watchlist_items
                 WHERE active = 1",
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
}
