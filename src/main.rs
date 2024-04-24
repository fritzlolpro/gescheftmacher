use std::{any::Any, result};

use error_chain::error_chain;
use goonmetrics::goonmetrics::PriceData;
use reqwest;
use rusqlite::{Connection as SQL_Connection, Result as SQL_Result};
use serde::{Deserialize, Serialize};
use serde_xml_rs::{from_str, to_string};

use tokio;
mod goonmetrics;
use crate::goonmetrics::goonmetrics::Goonmetrics;
use std::path::Path;

error_chain! {
    foreign_links {
        Io(std::io::Error);
        HttpRequest(reqwest::Error);
    }
}

#[derive(Debug)]
struct ItemData {
    type_id: i32,
    type_volume: f32,
    type_name: String,
    jita_trade_data: Option<TradeData>,
    abroad_trade_data: Option<TradeData>,
}

#[derive(Debug)]
struct TradeData {
    updated: String,
    weekly_movement: f64,
    buy_max: f64,
    buy_listed: i64,
    sell_min: f64,
    sell_listed: i64,
}

#[derive(Debug)]
struct ItemDataFromDb {
    type_id: i32,
    type_volume: f32,
}

fn get_stored_type_data(conn: &SQL_Connection, type_name: &str) -> SQL_Result<ItemDataFromDb> {
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

fn get_item_data_from_db(names: Vec<&str>) -> Vec<ItemData> {
    let curr_dir = std::env::current_dir().unwrap();
    let db_path = Path::new(&curr_dir).join("src").join("eve.db");
    println!("PATH:\n{:?}", db_path);
    let eve_db = SQL_Connection::open(db_path).unwrap();

    // let mut stored_type_data = get_stored_type_data(&eve_db, &name);
    // println!("Item info:\n{:?}", stored_type_data);

    names
        .into_iter()
        .map(|name| {
            let stored = get_stored_type_data(&eve_db, name).unwrap();
            let result = ItemData {
                type_name: name.to_string(),
                type_id: stored.type_id,
                type_volume: stored.type_volume,
                jita_trade_data: None,
                abroad_trade_data: None,
            };
            return result;
        })
        .collect()
}

async fn get_item_data_from_api(station_id: &str, item_ids: Vec<i32>) -> Result<Vec<PriceData>> {
    let item_ids = item_ids
        .into_iter()
        .map(|id| id.to_string() + ",")
        .collect::<String>();
    let jita_url = format!(
        "https://goonmetrics.apps.goonswarm.org/api/price_data/?station_id={station_id}&type_id={item_ids}"
    );

    let res = reqwest::get(jita_url).await?;

    let body = res.text().await?;
    println!("Body:\n{}", body);

    let data: Goonmetrics = from_str(&body).unwrap();
    let pd = data.price_data;
    return Ok(vec![pd]);
}

#[tokio::main]
async fn main() -> Result<()> {
    let name = "Tritanium";
    let names: Vec<&str> = vec!["Tritanium", "Buzzard"];

    let items: Vec<ItemData> = get_item_data_from_db(names);
    println!("Bulk from db:\n{:?}", items);

    let item_ids: Vec<i32> = items.into_iter().map(|item| item.type_id).collect();
    println!("IDIS:\n{:?}", item_ids);

    let jita_id = "60003760";
    let goon_keep_id = "1030049082711";

    let data = get_item_data_from_api(&jita_id, item_ids).await;

    println!("DATA:\n{:?}", data);
    // let trit_id = 34;
    // let jita_url = format!(
    //     "https://goonmetrics.apps.goonswarm.org/api/price_data/?station_id={jita_id}&type_id=34,35"
    // );

    // let res = reqwest::get(jita_url).await?;
    // println!("Status: {}", res.status());
    // println!("Headers:\n{:#?}", res.headers());

    // let body = res.text().await?;
    // println!("Body:\n{}", body);

    // let data: Goonmetrics = from_str(&body).unwrap();
    // let pd = data.price_data;

    // println!("pd:\n{:?}", pd);
    // let jita_trade_data = TradeData {
    //     updated: pd.updated,
    //     weekly_movement: pd.all.weekly_movement.parse::<f64>().unwrap(),
    //     buy_max: pd.buy.max.parse().unwrap(),
    //     buy_listed: pd.buy.listed.parse().unwrap(),
    //     sell_min: pd.sell.min.parse().unwrap(),
    //     sell_listed: pd.sell.listed.parse().unwrap(),
    // };
    // println!("jita trade:\n{:?}", jita_trade_data);
    // let goon_url = format!(
    //     "https://goonmetrics.apps.goonswarm.org/api/price_data/?station_id={goon_keep_id}&type_id={trit_id}"
    // );
    // let res = reqwest::get(goon_url).await?;
    // let body = res.text().await?;
    // let data: Goonmetrics = from_str(&body).unwrap();
    // let pd = data.price_data;
    // println!("pd:\n{:?}", pd);
    // let goon_trade_data = TradeData {
    //     updated: pd.updated,
    //     weekly_movement: pd.all.weekly_movement.parse::<f64>().unwrap(),
    //     buy_max: pd.buy.max.parse().unwrap(),
    //     buy_listed: pd.buy.listed.parse().unwrap(),
    //     sell_min: pd.sell.min.parse().unwrap(),
    //     sell_listed: pd.sell.listed.parse().unwrap(),
    // };
    // println!("goon trade:\n{:?}", goon_trade_data);

    // let type_market_info = ItemData {
    //     type_id: stored_type_data.as_mut().unwrap().type_id,
    //     type_volume: stored_type_data.as_mut().unwrap().type_volume,
    //     type_name: name.to_string(),
    //     jita_trade_data: jita_trade_data,
    //     abroad_trade_data: goon_trade_data,
    // };
    // print!("All trade data:\n{:?}", type_market_info);
    Ok(())
}
