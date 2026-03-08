// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
// #![allow(unused)]
use serde::{Deserialize, Serialize};
use serde_xml_rs::from_str;

use error_chain::error_chain;
use struct_field_names_as_array::FieldNamesAsSlice;
use tokio;

use chrono;

#[cfg(test)]
mod datagetter_tests;

mod static_data;
mod datagetter;
mod goonmetrics;
mod watchlist;
use datagetter::datagetter::{
    create_table_and_store_data, get_item_data_from_api,
    get_stored_items_history, merge_trade_data, ExtendedItemData,
    ItemData, TradeData, SqlLiteConnection, DatabaseConnection,
};
use watchlist::watchlist::{ensure_watchlist_initialized, get_watchlist_as_item_data};
use std::path::PathBuf;

use numfmt::Formatter;
use numfmt::Precision;

const DELIVERY_PRICE_PER_CUBOMETR: f32 = 1200.0;
const MIN_SELL_MARGIN_THRESHOLD: f32 = 1.15;
const JITA_TAXRATE: f64 = 0.0108;
const PROFIT_THRESHOLD: i64 = 30000000;
const FREEZE_RATE_THRESHOLD: f32 = 0.1;
const MARKET_RATE_THRESHOLD: i32 = 1;
const DAILY_VOL_THRESHOLD: i64 = 10;
const ABROAD_TAX_VALUE: f64 = 0.056;

const CACHE_EXPIRY_DURATION_MINUTES: i64 = 1200;

const JITA_ID: &str = "60003760";
const GOON_KEEP_ID: &str = "1049588174021";

error_chain! {
    foreign_links {
        Io(std::io::Error);
        HttpRequest(reqwest::Error);
    }
}

impl ItemData {
    pub fn get_shipping_price(&self) -> f64 {
        let shipping_price = &self.type_volume * DELIVERY_PRICE_PER_CUBOMETR;
        return shipping_price as f64;
    }
    pub fn get_jita_buy_price_with_tax(&self) -> f64 {
        let jtd = &self.jita_trade_data.as_ref().unwrap();
        return jtd.buy_max * JITA_TAXRATE + jtd.buy_max;
    }
    pub fn get_abroad_stocked_ratio(&self) -> f64 {
        let abtd = &self.abroad_trade_data.as_ref().unwrap();
        return abtd.sell_listed as f64 / abtd.weekly_movement;
    }

    pub fn get_abroad_sell_taxed(&self) -> f64 {
        let abtd = &self.abroad_trade_data.as_ref().unwrap();
        return abtd.sell_min - abtd.sell_min * ABROAD_TAX_VALUE;
    }

    pub fn get_abroad_avg_daily(&self) -> f64 {
        let abtd = &self.abroad_trade_data.as_ref().unwrap();

        let abstocked = &self.get_abroad_stocked_ratio();
        if *abstocked == 0.0 {
            return 0.0;
        }

        return abtd.weekly_movement / 7.0 / f64::sqrt(*abstocked);
    }

    pub fn get_profit_jita_buy_per_unit(&self) -> f64 {
        return &self.get_abroad_sell_taxed()
            - &self.get_jita_buy_price_with_tax()
            - &self.get_shipping_price();
    }

    pub fn get_profit_jita_buy_daily(&self) -> f64 {
        return &self.get_abroad_avg_daily() * &self.get_profit_jita_buy_per_unit();
    }

    pub fn get_margin_jita_buy(&self) -> f64 {
        return &self.get_profit_jita_buy_per_unit()
            / (&self.get_jita_buy_price_with_tax() + &self.get_shipping_price());
    }

    pub fn get_money_freeze_buy(&self) -> f64 {
        return &self.get_abroad_avg_daily() * &self.get_jita_buy_price_with_tax();
    }

    pub fn get_freeze_rate(&self) -> f64 {
        let mfb = &self.get_money_freeze_buy();
        if *mfb == 0.0 {
            return 0.0;
        }
        return &self.get_profit_jita_buy_daily() / mfb;
    }
}

impl ExtendedItemData {
    fn new(data: ItemData, timestamp: i64) -> Self {
        let shipping_price = data.get_shipping_price();
        let jtd = data.jita_trade_data.clone().unwrap();
        let atd = data.abroad_trade_data.clone().unwrap();
        let id = data.type_id;
        let name = data.type_name.to_owned();
        let volume = data.type_volume;
        let jtb_with_tax = data.get_jita_buy_price_with_tax();
        let abroad_stocked_ratio = data.get_abroad_stocked_ratio();
        let abroad_sell_taxed = data.get_abroad_sell_taxed();
        let abroad_avg_daily = data.get_abroad_avg_daily();
        let profit_jita_buy_per_unit = data.get_profit_jita_buy_per_unit();
        let profit_jita_buy_daily = data.get_profit_jita_buy_daily();
        let margin_jita_buy = data.get_margin_jita_buy();
        let money_freeze_buy = data.get_money_freeze_buy();
        let freeze_rate = data.get_freeze_rate();

        // TODO: Add filters to display only good stuff
        ExtendedItemData {
            type_id: id,
            timestamp: timestamp,
            type_volume: volume,
            type_name: name,
            jita_trade_data: jtd,
            jita_buy_with_tax: jtb_with_tax,
            abroad_trade_data: atd,
            abroad_stocked_ratio,
            shipping_price: shipping_price,
            abroad_sell_taxed: abroad_sell_taxed,
            abroad_avg_daily: abroad_avg_daily,
            profit_jita_buy_per_unit: profit_jita_buy_per_unit,
            profit_jita_buy_daily: profit_jita_buy_daily,
            margin_jita_buy: margin_jita_buy,
            money_freeze_buy: money_freeze_buy,
            freeze_rate: freeze_rate,
        }
    }
}

async fn fetch_and_compute_items(items_data: Vec<ItemData>) -> Vec<ExtendedItemData> {
    if items_data.is_empty() {
        return vec![];
    }
    let item_ids: Vec<i32> = items_data.iter().map(|i| i.type_id).collect();
    let current_time = chrono::Utc::now().timestamp();

    let items_history = get_stored_items_history(&item_ids);
    if items_history.all_ids_present_and_recent(&item_ids, CACHE_EXPIRY_DURATION_MINUTES) {
        println!("Cache hit — skipping API.");
        return items_history.get_most_recent_item_data();
    }

    println!("Cache miss — fetching from API.");
    let jita_data = match get_item_data_from_api(JITA_ID, &item_ids).await {
        Ok(d) => d,
        Err(e) => { eprintln!("Jita API error: {:?}", e); return vec![]; }
    };
    let goon_data = match get_item_data_from_api(GOON_KEEP_ID, &item_ids).await {
        Ok(d) => d,
        Err(e) => { eprintln!("Goon API error: {:?}", e); return vec![]; }
    };

    let merged = merge_trade_data(&items_data, &jita_data, &goon_data);
    let mut result = vec![];
    for item in merged {
        let ext = ExtendedItemData::new(item, current_time);
        create_table_and_store_data(ext.clone());
        result.push(ext);
    }
    result
}

fn resolve_item_names(names: &[String]) -> (Vec<ItemData>, Vec<String>) {
    let eve_conn = SqlLiteConnection::open(PathBuf::from("src/eve.db"))
        .expect("cannot open eve.db");
    let mut found: Vec<ItemData> = vec![];
    let mut not_found: Vec<String> = vec![];
    for name in names {
        match eve_conn.get_stored_type_data(name) {
            Ok(data) => {
                let volume = eve_conn
                    .get_stored_type_volume_packed(data.type_id)
                    .unwrap_or(data.type_volume);
                found.push(ItemData {
                    type_id: data.type_id,
                    type_volume: volume,
                    type_name: name.clone(),
                    jita_trade_data: None,
                    abroad_trade_data: None,
                });
            }
            Err(_) => not_found.push(name.clone()),
        }
    }
    (found, not_found)
}

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<()> {
    env_logger::init();
    ensure_watchlist_initialized();
    let items_data = get_watchlist_as_item_data();
    let extended_data = fetch_and_compute_items(items_data).await;
    run(extended_data);
    Ok(())
}

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[tauri::command]
async fn get_prices_for_items(item_names: Vec<String>) -> String {
    let (found_items, not_found) = resolve_item_names(&item_names);
    let found = fetch_and_compute_items(found_items).await;
    serde_json::to_string(&PriceQueryResult { found, not_found }).unwrap()
}

#[tauri::command]
fn get_data(state: tauri::State<AppData>) -> String {
    let data = state.data.clone();
    format!("{:?}", serde_json::to_string(&data).unwrap())
}

struct AppData {
    data: Vec<ExtendedItemData>,
}

#[derive(Serialize)]
struct PriceQueryResult {
    found: Vec<ExtendedItemData>,
    not_found: Vec<String>,
}
use tauri::{Builder, Manager};
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run(data: Vec<ExtendedItemData>) {
    Builder::default()
        .setup(move |app| {
            app.manage(AppData { data: data.clone() });
            Ok(())
        })
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![get_data, greet, get_prices_for_items])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

pub trait FormatForDisplay {
    fn format_for_display(&self) -> String;
    fn format_for_display_percentage(&self) -> String;
}

impl FormatForDisplay for f64 {
    fn format_for_display(&self) -> String {
        let mut f: Formatter;
        f = "[n/ ]".parse().unwrap();
        f = f.precision(Precision::Decimals(2));
        let res = f.fmt2(self.to_owned());
        return res.to_owned();
    }

    fn format_for_display_percentage(&self) -> String {
        let mut f: Formatter;
        f = "[.2%]".parse().unwrap();
        f = f.precision(Precision::Decimals(2));
        let res = f.fmt2(self.to_owned());
        return res.to_owned();
    }
}

impl FormatForDisplay for i64 {
    fn format_for_display(&self) -> String {
        let mut f: Formatter;
        f = "[n/ ]".parse().unwrap();
        f = f.precision(Precision::Decimals(2));
        let res = f.fmt2(self.to_owned());
        return res.to_owned();
    }
    fn format_for_display_percentage(&self) -> String {
        let mut f: Formatter;
        f = "[.2%]".parse().unwrap();
        f = f.precision(Precision::Decimals(2));
        let res = f.fmt2(self.to_owned());
        return res.to_owned();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::goonmetrics::goonmetrics::*;

    #[test]
    fn merge_stuff() {
        let items_data: &Vec<ItemData> = &[
            ItemData {
                type_id: 34,
                type_volume: 0.01,
                type_name: "Tritanium".to_string(),
                jita_trade_data: None,
                abroad_trade_data: None,
            },
            ItemData {
                type_id: 11192,
                type_volume: 19400.0,
                type_name: "Buzzard".to_string(),
                jita_trade_data: None,
                abroad_trade_data: None,
            },
        ]
        .to_vec();

        let mock_jita_trade_data: Result<Vec<PriceData>> = Ok([PriceData {
            types: [
                Types::Type(ItemType {
                    id: 34,
                    updated: "2024-05-03T13:36:22Z".to_string(),
                    all: All {
                        weekly_movement: "3".to_string(),
                    },
                    buy: Buy {
                        listed: "3".to_string(),
                        max: "3".to_string(),
                    },
                    sell: Sell {
                        listed: "3".to_string(),
                        min: "3".to_string(),
                    },
                }),
                Types::Type(ItemType {
                    id: 11192,
                    updated: "2024-05-03T13:36:22Z".to_string(),
                    all: All {
                        weekly_movement: "3".to_string(),
                    },
                    buy: Buy {
                        listed: "3".to_string(),
                        max: "3".to_string(),
                    },
                    sell: Sell {
                        listed: "3".to_string(),
                        min: "3".to_string(),
                    },
                }),
            ]
            .to_vec(),
        }]
        .to_vec());

        let mock_goon_trade_data: Result<Vec<PriceData>> = Ok([PriceData {
            types: [
                Types::Type(ItemType {
                    id: 34,
                    updated: "2024-05-03T13:36:22Z".to_string(),
                    all: All {
                        weekly_movement: "3".to_string(),
                    },
                    buy: Buy {
                        listed: "3".to_string(),
                        max: "3".to_string(),
                    },
                    sell: Sell {
                        listed: "3".to_string(),
                        min: "3".to_string(),
                    },
                }),
                Types::Type(ItemType {
                    id: 11192,
                    updated: "2024-05-03T13:36:22Z".to_string(),
                    all: All {
                        weekly_movement: "3".to_string(),
                    },
                    buy: Buy {
                        listed: "3".to_string(),
                        max: "3".to_string(),
                    },
                    sell: Sell {
                        listed: "3".to_string(),
                        min: "3".to_string(),
                    },
                }),
            ]
            .to_vec(),
        }]
        .to_vec());

        let desired_merge_result = vec![
            ItemData {
                type_id: 34,
                type_volume: 0.01,
                type_name: "Tritanium".to_string(),
                jita_trade_data: Some(TradeData {
                    updated: "2024-05-03T13:36:22Z".to_string(),
                    weekly_movement: "3".to_string().parse::<f64>().expect("CANT PARSE!"),
                    buy_max: "3".to_string().parse::<f64>().expect("CANT PARSE!"),
                    buy_listed: "3".to_string().parse::<i64>().expect("CANT PARSE!"),
                    sell_min: "3".to_string().parse::<f64>().expect("CANT PARSE!"),
                    sell_listed: "3".to_string().parse::<i64>().expect("CANT PARSE!"),
                }),
                abroad_trade_data: Some(TradeData {
                    updated: "2024-05-03T13:36:22Z".to_string(),
                    weekly_movement: "3".to_string().parse::<f64>().expect("CANT PARSE!"),
                    buy_max: "3".to_string().parse::<f64>().expect("CANT PARSE!"),
                    buy_listed: "3".to_string().parse::<i64>().expect("CANT PARSE!"),
                    sell_min: "3".to_string().parse::<f64>().expect("CANT PARSE!"),
                    sell_listed: "3".to_string().parse::<i64>().expect("CANT PARSE!"),
                }),
            },
            ItemData {
                type_id: 11192,
                type_volume: 19400.0,
                type_name: "Buzzard".to_string(),
                jita_trade_data: Some(TradeData {
                    updated: "2024-05-03T13:36:22Z".to_string(),
                    weekly_movement: "3".to_string().parse::<f64>().expect("CANT PARSE!"),
                    buy_max: "3".to_string().parse::<f64>().expect("CANT PARSE!"),
                    buy_listed: "3".to_string().parse::<i64>().expect("CANT PARSE!"),
                    sell_min: "3".to_string().parse::<f64>().expect("CANT PARSE!"),
                    sell_listed: "3".to_string().parse::<i64>().expect("CANT PARSE!"),
                }),
                abroad_trade_data: Some(TradeData {
                    updated: "2024-05-03T13:36:22Z".to_string(),
                    weekly_movement: "3".to_string().parse::<f64>().expect("CANT PARSE!"),
                    buy_max: "3".to_string().parse::<f64>().expect("CANT PARSE!"),
                    buy_listed: "3".to_string().parse::<i64>().expect("CANT PARSE!"),
                    sell_min: "3".to_string().parse::<f64>().expect("CANT PARSE!"),
                    sell_listed: "3".to_string().parse::<i64>().expect("CANT PARSE!"),
                }),
            },
        ];

        let actual_merge_result = merge_trade_data(
            items_data,
            &mock_jita_trade_data.expect("aaa"),
            &mock_goon_trade_data.expect("aaa"),
        );

        assert_eq!(desired_merge_result, actual_merge_result);
    }

    #[test]
    fn calculate_fields() {
        let mock_item = ItemData {
            type_id: 11192,
            type_volume: 2500.0,
            type_name: "Buzzard".to_owned(),
            jita_trade_data: Some(TradeData {
                updated: "2024-08-21T16:16:48Z".to_owned(),
                weekly_movement: 865.2,
                buy_max: 10_000_000.0,
                buy_listed: 138,
                sell_min: 23200000.0,
                sell_listed: 758,
            }),
            abroad_trade_data: Some(TradeData {
                updated: "2024-08-21T16:15:35Z".to_owned(),
                weekly_movement: 62.5,
                buy_max: 11_000_000.0,
                buy_listed: 18,
                sell_min: 15_000_000.0,
                sell_listed: 95,
            }),
        };
        println!(
            "Data abroad avg daily: \n {:?}",
            mock_item.get_abroad_avg_daily().format_for_display()
        );
        println!(
            "Jita_buy price with tax: \n {:?}",
            mock_item.get_jita_buy_price_with_tax().format_for_display()
        );
        println!(
            "Shipping price: \n {:?}",
            mock_item.get_shipping_price().format_for_display()
        );
        println!(
            "Abroad sell taxed: \n {:?}",
            mock_item.get_abroad_sell_taxed().format_for_display()
        );
        println!(
            "Jita_buy profit per unit: \n {:?}",
            mock_item
                .get_profit_jita_buy_per_unit()
                .format_for_display()
        );
        println!(
            "Jita_buy dialy profit: \n {:?}",
            mock_item.get_profit_jita_buy_daily().format_for_display()
        );
        println!(
            "Money freeze rate buy: \n {:?}",
            mock_item.get_money_freeze_buy().format_for_display()
        );
        println!(
            "Margin: \n {:?}",
            mock_item
                .get_margin_jita_buy()
                .format_for_display_percentage()
        );
        println!("Freeze rate: \n {:?}", mock_item.get_freeze_rate());
    }
}
