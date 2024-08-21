pub mod datagetter {
    use crate::from_str;
    use crate::goonmetrics::goonmetrics::*;
    use error_chain::error_chain;
    use reqwest;
    use rusqlite::{Connection as SQL_Connection, Result as SQL_Result};
    use serde::{Deserialize, Serialize};
    use std::path::Path;
    use std::sync::mpsc;
    use struct_field_names_as_array::FieldNamesAsSlice;
    use tokio::task;

    error_chain! {
        foreign_links {
            Io(std::io::Error);
            HttpRequest(reqwest::Error);
        }
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

    #[derive(Debug, PartialEq, Clone)]
    pub struct ItemData {
        pub type_id: i32,
        pub type_volume: f32,
        pub type_name: String,
        pub jita_trade_data: Option<TradeData>,
        pub abroad_trade_data: Option<TradeData>,
    }

    // TODO: from this db get unpacked volume try other table if it has packed use packed
    // get packed volume form invVolumes
    // SELECT volume FROM invVolumes
    // WHERE typeID = 22544
    pub fn get_stored_type_data(
        conn: &SQL_Connection,
        type_name: &str,
    ) -> SQL_Result<ItemDataFromDb> {
        let mut stmt = conn.prepare(
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

    pub fn get_stored_type_volume_packed(conn: &SQL_Connection, type_id: i32) -> SQL_Result<f32> {
        let mut stmt = conn.prepare(
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

    pub fn get_item_data_from_db(names: Vec<&str>) -> Vec<ItemData> {
        let curr_dir = std::env::current_dir().unwrap();
        let db_path = Path::new(&curr_dir).join("src").join("eve.db");
        println!("PATH:\n{:?}", db_path);
        let src_path_connection = SQL_Connection::open(db_path);

        let exe = std::env::current_exe().unwrap();
        let exe_loc = exe.parent().unwrap();
        let exe_path = Path::new(&exe_loc).join("eve.db");
        let eve_db: SQL_Connection;

        if let Err(_err) = src_path_connection {
            eve_db = SQL_Connection::open(exe_path.clone()).unwrap()
        } else {
            eve_db = src_path_connection.unwrap()
        }
        println!("exe_path:\n{:?}", exe_path);

        names
            .into_iter()
            .map(|name| {
                let stored = get_stored_type_data(&eve_db, name).unwrap();

                let item_id = stored.type_id;
                let packed_volume = get_stored_type_volume_packed(&eve_db, item_id);

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

    const MAX_GOONMETRICS_ID_QUANTITY: usize = 99;

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
#[cfg(test)]
mod tests {
    use crate::datagetter::datagetter::*;
    use rusqlite::Connection as SQL_Connection;
    use std::path::Path;
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
        let curr_dir = std::env::current_dir().unwrap();
        let db_path = Path::new(&curr_dir).join("src").join("eve.db");
        println!("PATH:\n{:?}", db_path);
        let src_path_connection = SQL_Connection::open(db_path);

        let exe = std::env::current_exe().unwrap();
        let exe_loc = exe.parent().unwrap();
        let exe_path = Path::new(&exe_loc).join("eve.db");
        let eve_db: SQL_Connection;

        if let Err(_err) = src_path_connection {
            eve_db = SQL_Connection::open(exe_path.clone()).unwrap()
        } else {
            eve_db = src_path_connection.unwrap()
        }
        let stored = get_stored_type_data(&eve_db, name).unwrap();
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
    fn get_item_packed_volume_by_id() {
        let hulk_id = 22544;
        let hulk_packed_volume = 3750 as f32;

        let curr_dir = std::env::current_dir().unwrap();
        let db_path = Path::new(&curr_dir).join("src").join("eve.db");
        let src_path_connection = SQL_Connection::open(db_path);

        let exe = std::env::current_exe().unwrap();
        let exe_loc = exe.parent().unwrap();
        let exe_path = Path::new(&exe_loc).join("eve.db");
        let eve_db: SQL_Connection;

        if let Err(_err) = src_path_connection {
            eve_db = SQL_Connection::open(exe_path.clone()).unwrap()
        } else {
            eve_db = src_path_connection.unwrap()
        }
        let stored = get_stored_type_volume_packed(&eve_db, hulk_id).unwrap();
        println!("aaaa:\n{:?}", stored);
        assert_eq!(hulk_packed_volume, stored)
    }

    #[test]
    fn return_packed_volume_if_exists() {
        let name = "Hulk";
        let hulk_packed_volume = 3750 as f32;
        let curr_dir = std::env::current_dir().unwrap();
        let db_path = Path::new(&curr_dir).join("src").join("eve.db");
        let src_path_connection = SQL_Connection::open(db_path);

        let exe = std::env::current_exe().unwrap();
        let exe_loc = exe.parent().unwrap();
        let exe_path = Path::new(&exe_loc).join("eve.db");
        let eve_db: SQL_Connection;

        if let Err(_err) = src_path_connection {
            eve_db = SQL_Connection::open(exe_path.clone()).unwrap()
        } else {
            eve_db = src_path_connection.unwrap()
        }

        let stored = get_stored_type_data(&eve_db, name).unwrap();
        let item_id = stored.type_id;

        let packed_volume = get_stored_type_volume_packed(&eve_db, item_id);

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
        let curr_dir = std::env::current_dir().unwrap();
        let db_path = Path::new(&curr_dir).join("src").join("eve.db");
        let src_path_connection = SQL_Connection::open(db_path);

        let exe = std::env::current_exe().unwrap();
        let exe_loc = exe.parent().unwrap();
        let exe_path = Path::new(&exe_loc).join("eve.db");
        let eve_db: SQL_Connection;

        if let Err(_err) = src_path_connection {
            eve_db = SQL_Connection::open(exe_path.clone()).unwrap()
        } else {
            eve_db = src_path_connection.unwrap()
        }

        let stored = get_stored_type_data(&eve_db, name).unwrap();
        let item_id = stored.type_id;

        println!("aaaa:\n{:?}", stored);
        let packed_volume = get_stored_type_volume_packed(&eve_db, item_id);

        let volume: f32;
        if let Err(_err) = packed_volume {
            volume = stored.type_volume;
        } else {
            volume = packed_volume.unwrap();
        }

        assert_eq!(trit_volume, volume)
    }
}
