#[cfg(test)]
mod tests {
    use super::*;
    use crate::datagetter::datagetter::*;
    use rusqlite::{
        Connection as SQL_Connection, Error as SQL_Error, Result as SQL_Result, Statement,
    };
    use rusqlite::{Row, Rows};
    use std::any::type_name_of_val;
    use std::path::PathBuf;
    #[test]
    fn test_split_by_treshold_small() {
        let treshold: usize = 3;
        let items = vec![33, 55, 31];
        assert_eq!(split_large_id_bulks(&items, treshold), vec![items])
    }

    #[test]
    fn test_split_by_treshold_big() {
        let treshold: usize = 2;
        let items = vec![33, 55, 31, 77];
        let exp_result = vec![vec![33, 55], vec![31, 77]];
        assert_eq!(split_large_id_bulks(&items, treshold), exp_result)
    }

    #[test]
    fn test_split_by_treshold_biger_no_even() {
        let treshold: usize = 2;
        let items = vec![33, 55, 31, 77, 99];
        let binding = vec![33, 55];
        let binding_2 = vec![31, 77];
        let binding_3 = vec![99];
        let exp_result = vec![binding, binding_2, binding_3];
        assert_eq!(split_large_id_bulks(&items, treshold), exp_result)
    }

    #[test]
    fn get_item_from_db_by_name() {
        let name = "Hulk";
        let db_path = PathBuf::from("src/eve.db");
        let connection = SqlLiteConnection::open(db_path).unwrap();

        let stored = connection.get_stored_type_data(name).unwrap();
        println!("aaaa:\n{:?}", stored);
        assert_eq!(
            stored,
            ItemDataFromDb {
                type_id: 22544,
                type_volume: 150000.0
            }
        )
    }

    #[test]
    fn test_get_tradable_item_names_some_results() {
        let db_path = PathBuf::from("src/eve.db");
        let connection = SqlLiteConnection::open(db_path).unwrap();

        let result = connection.get_tradable_item_names().unwrap();

        assert!(!result.is_empty());
        for name in &result {
            assert!(name.len() > 0);
        }
    }

    #[test]
    fn get_item_packed_volume_by_id() {
        let hulk_id = 22544;
        let hulk_packed_volume = 3750 as f32;

        let db_path = PathBuf::from("src/eve.db");
        let connection = SqlLiteConnection::open(db_path).unwrap();

        let stored = connection.get_stored_type_volume_packed(hulk_id).unwrap();
        println!("aaaa:\n{:?}", stored);
        assert_eq!(hulk_packed_volume, stored)
    }

    #[test]
    fn return_packed_volume_if_exists() {
        let name = "Hulk";
        let hulk_packed_volume = 3750 as f32;

        let db_path = PathBuf::from("src/eve.db");
        let connection = SqlLiteConnection::open(db_path).unwrap();

        let stored = connection.get_stored_type_data(name).unwrap();
        let item_id = stored.type_id;

        let packed_volume = connection.get_stored_type_volume_packed(item_id);

        let volume: f32;
        if let Err(_err) = packed_volume {
            volume = stored.type_volume;
        } else {
            volume = packed_volume.unwrap();
        }

        assert_eq!(hulk_packed_volume, volume)
    }

    #[test]
    fn return_regular_volume_if_packed_not_exists() {
        let name = "Tritanium";
        let trit_volume = 0.01;

        let db_path = PathBuf::from("src/eve.db");
        let connection = SqlLiteConnection::open(db_path).unwrap();

        let stored = connection.get_stored_type_data(name).unwrap();
        let item_id = stored.type_id;

        println!("aaaa:\n{:?}", stored);
        let packed_volume = connection.get_stored_type_volume_packed(item_id);

        let volume: f32;
        if let Err(_err) = packed_volume {
            volume = stored.type_volume;
        } else {
            volume = packed_volume.unwrap();
        }

        assert_eq!(trit_volume, volume)
    }

    #[test]
    fn test_create_extended_item_data_table() {
        let mut conn = SqlLiteConnection::open_in_memory().unwrap();

        conn.create_extended_item_data_table().unwrap();
        let query =
            "SELECT name FROM sqlite_master WHERE type='table' AND name='extended_item_data'";
        let mut stmt = conn.prepare(query).unwrap();
        let result = stmt
            .query_row([], |row| Ok(row.get::<_, String>(0)))
            .unwrap();

        assert_eq!(result.unwrap(), "extended_item_data");
    }

    #[test]

    fn test_extended_item_data_table_columns() {
        let mut conn = SqlLiteConnection::open_in_memory().unwrap();

        conn.create_extended_item_data_table().unwrap();

        let mut names = Vec::new();
        let query = "PRAGMA table_info(extended_item_data)";
        let mut stmt = conn.prepare(query).unwrap();
        let rows = stmt.query_map([], |row| row.get::<_, String>(1)).unwrap();

        for row in rows {
            names.push(row);
        }

        assert_eq!(names.len(), 17);
        assert!(names
            .iter()
            .any(|r| *r.as_ref().unwrap() == "id".to_owned()));
        assert!(names
            .iter()
            .any(|r| *r.as_ref().unwrap() == "type_id".to_owned()));
        assert!(names
            .iter()
            .any(|r| *r.as_ref().unwrap() == "timestamp".to_owned()));
        assert!(names
            .iter()
            .any(|r| *r.as_ref().unwrap() == "type_volume".to_owned()));
        assert!(names
            .iter()
            .any(|r| *r.as_ref().unwrap() == "type_name".to_owned()));
        assert!(names
            .iter()
            .any(|r| *r.as_ref().unwrap() == "jita_trade_data".to_owned()));
        assert!(names
            .iter()
            .any(|r| *r.as_ref().unwrap() == "abroad_trade_data".to_owned()));
        assert!(names
            .iter()
            .any(|r| *r.as_ref().unwrap() == "jita_buy_with_tax".to_owned()));
        assert!(names
            .iter()
            .any(|r| *r.as_ref().unwrap() == "abroad_stocked_ratio".to_owned()));
        assert!(names
            .iter()
            .any(|r| *r.as_ref().unwrap() == "shipping_price".to_owned()));
        assert!(names
            .iter()
            .any(|r| *r.as_ref().unwrap() == "abroad_sell_taxed".to_owned()));
        assert!(names
            .iter()
            .any(|r| *r.as_ref().unwrap() == "abroad_avg_daily".to_owned()));
        assert!(names
            .iter()
            .any(|r| *r.as_ref().unwrap() == "profit_jita_buy_per_unit".to_owned()));
        assert!(names
            .iter()
            .any(|r| *r.as_ref().unwrap() == "profit_jita_buy_daily".to_owned()));
        assert!(names
            .iter()
            .any(|r| *r.as_ref().unwrap() == "margin_jita_buy".to_owned()));
        assert!(names
            .iter()
            .any(|r| *r.as_ref().unwrap() == "money_freeze_buy".to_owned()));
        assert!(names
            .iter()
            .any(|r| *r.as_ref().unwrap() == "freeze_rate".to_owned()));
    }

    #[test]
    fn test_store_extended_item_data() {
        let mut conn = SqlLiteConnection::open_in_memory().unwrap();

        conn.create_extended_item_data_table().unwrap();

        let extended_item_data = ExtendedItemData {
            type_id: 123,
            type_volume: 456.0,
            timestamp: 789,
            type_name: "Test Item".to_string(),
            jita_trade_data: TradeData {
                updated: "2025-03-15T00:00:00Z".to_string(),
                weekly_movement: 100.0,
                buy_max: 200.0,
                buy_listed: 300,
                sell_min: 400.0,
                sell_listed: 500,
            },
            jita_buy_with_tax: 600.0,
            abroad_trade_data: TradeData {
                updated: "2025-03-15T00:00:00Z".to_string(),
                weekly_movement: 700.0,
                buy_max: 800.0,
                buy_listed: 900,
                sell_min: 1000.0,
                sell_listed: 1100,
            },
            abroad_stocked_ratio: 1200.0,
            shipping_price: 1300.0,
            abroad_sell_taxed: 1400.0,
            abroad_avg_daily: 1500.0,
            profit_jita_buy_per_unit: 1600.0,
            profit_jita_buy_daily: 1700.0,
            margin_jita_buy: 1800.0,
            money_freeze_buy: 1900.0,
            freeze_rate: 2000.0,
        };

        conn.store_extended_item_data(extended_item_data.clone()).unwrap();

        let stored_data = conn.get_stored_extended_item_data(123).unwrap();
        assert_eq!(stored_data.len(), 1);
        let stored_item = &stored_data[0];

        assert_eq!(stored_item.type_id, extended_item_data.type_id);
        assert_eq!(stored_item.type_volume, extended_item_data.type_volume);
        assert_eq!(stored_item.timestamp, extended_item_data.timestamp);
        assert_eq!(stored_item.type_name, extended_item_data.type_name);
        assert_eq!(stored_item.jita_trade_data, extended_item_data.jita_trade_data);
        assert_eq!(stored_item.jita_buy_with_tax, extended_item_data.jita_buy_with_tax);
        assert_eq!(stored_item.abroad_trade_data, extended_item_data.abroad_trade_data);
        assert_eq!(stored_item.abroad_stocked_ratio, extended_item_data.abroad_stocked_ratio);
        assert_eq!(stored_item.shipping_price, extended_item_data.shipping_price);
        assert_eq!(stored_item.abroad_sell_taxed, extended_item_data.abroad_sell_taxed);
        assert_eq!(stored_item.abroad_avg_daily, extended_item_data.abroad_avg_daily);
        assert_eq!(stored_item.profit_jita_buy_per_unit, extended_item_data.profit_jita_buy_per_unit);
        assert_eq!(stored_item.profit_jita_buy_daily, extended_item_data.profit_jita_buy_daily);
        assert_eq!(stored_item.margin_jita_buy, extended_item_data.margin_jita_buy);
        assert_eq!(stored_item.money_freeze_buy, extended_item_data.money_freeze_buy);
        assert_eq!(stored_item.freeze_rate, extended_item_data.freeze_rate);
    }

     #[test]
    fn test_store_extended_item_data() {
        let mut conn = SqlLiteConnection::open_in_memory().unwrap();

        conn.create_extended_item_data_table().unwrap();

        let extended_item_data = ExtendedItemData {
            type_id: 123,
            type_volume: 456.0,
            timestamp: 789,
            type_name: "Test Item".to_string(),
            jita_trade_data: TradeData {
                updated: "2025-03-15T00:00:00Z".to_string(),
                weekly_movement: 100.0,
                buy_max: 200.0,
                buy_listed: 300,
                sell_min: 400.0,
                sell_listed: 500,
            },
            jita_buy_with_tax: 600.0,
            abroad_trade_data: TradeData {
                updated: "2025-03-15T00:00:00Z".to_string(),
                weekly_movement: 700.0,
                buy_max: 800.0,
                buy_listed: 900,
                sell_min: 1000.0,
                sell_listed: 1100,
            },
            abroad_stocked_ratio: 1200.0,
            shipping_price: 1300.0,
            abroad_sell_taxed: 1400.0,
            abroad_avg_daily: 1500.0,
            profit_jita_buy_per_unit: 1600.0,
            profit_jita_buy_daily: 1700.0,
            margin_jita_buy: 1800.0,
            money_freeze_buy: 1900.0,
            freeze_rate: 2000.0,
        };

        conn.store_extended_item_data(extended_item_data.clone()).unwrap();

        let stored_data = conn.get_stored_extended_item_data(123).unwrap();
        assert_eq!(stored_data.len(), 1);
        let stored_item = &stored_data[0];

        assert_eq!(stored_item.type_id, extended_item_data.type_id);
        assert_eq!(stored_item.type_volume, extended_item_data.type_volume);
        assert_eq!(stored_item.timestamp, extended_item_data.timestamp);
        assert_eq!(stored_item.type_name, extended_item_data.type_name);
        assert_eq!(stored_item.jita_trade_data, extended_item_data.jita_trade_data);
        assert_eq!(stored_item.jita_buy_with_tax, extended_item_data.jita_buy_with_tax);
        assert_eq!(stored_item.abroad_trade_data, extended_item_data.abroad_trade_data);
        assert_eq!(stored_item.abroad_stocked_ratio, extended_item_data.abroad_stocked_ratio);
        assert_eq!(stored_item.shipping_price, extended_item_data.shipping_price);
        assert_eq!(stored_item.abroad_sell_taxed, extended_item_data.abroad_sell_taxed);
        assert_eq!(stored_item.abroad_avg_daily, extended_item_data.abroad_avg_daily);
        assert_eq!(stored_item.profit_jita_buy_per_unit, extended_item_data.profit_jita_buy_per_unit);
        assert_eq!(stored_item.profit_jita_buy_daily, extended_item_data.profit_jita_buy_daily);
        assert_eq!(stored_item.margin_jita_buy, extended_item_data.margin_jita_buy);
        assert_eq!(stored_item.money_freeze_buy, extended_item_data.money_freeze_buy);
        assert_eq!(stored_item.freeze_rate, extended_item_data.freeze_rate);
    }
}
