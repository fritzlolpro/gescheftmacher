use serde::{Deserialize, Serialize};
use serde_xml_rs::from_str;

use error_chain::error_chain;
use struct_field_names_as_array::FieldNamesAsSlice;
use tokio;

mod ui;
use ui::ui::{render_ui, TradeItemViewManager, TradeItemViewManagerInitData};
mod datagetter;
mod goonmetrics;
use datagetter::datagetter::{
    get_item_data_from_api, get_item_data_from_db, merge_trade_data, ItemData, TradeData,
};

const DELIVERY_PRICE_PER_CUBOMETR: f32 = 850.0;
const MIN_SELL_MARGIN_THRESHOLD: f32 = 1.15;
const JITA_TAXRATE: f64 = 0.0108;
const PROFIT_THRESHOLD: i64 = 30000000;
const FREEZE_RATE_THRESHOLD: f32 = 0.1;
const MARKET_RATE_THRESHOLD: i32 = 1;
const DAILY_VOL_THRESHOLD: i64 = 10;
const ABROAD_TAX_VALUE: f64 = 0.056;

error_chain! {
    foreign_links {
        Io(std::io::Error);
        HttpRequest(reqwest::Error);
    }
}

#[derive(Debug, PartialEq, Clone, FieldNamesAsSlice, Deserialize, Serialize)]
pub struct ExtendedItemData {
    type_id: i32,
    type_volume: f32,
    type_name: String,
    jita_trade_data: TradeData,
    jita_buy_with_tax: f64,
    abroad_trade_data: TradeData,
    abroad_stocked_ratio: f64,
    shipping_price: f64,
    abroad_sell_taxed: f64,
    abroad_avg_daily: f64,
    profit_jita_buy_per_unit: f64,
    profit_jita_buy_daily: f64,
    margin_jita_buy: f64,
    money_freeze_buy: f64,
    freeze_rate: f64,
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
        return &self.get_profit_jita_buy_daily() / &self.get_money_freeze_buy();
    }
}

impl ExtendedItemData {
    fn new(data: ItemData) -> Self {
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
            type_volume: volume,
            type_name: name,
            jita_trade_data: jtd,
            jita_buy_with_tax: jtb_with_tax,
            abroad_trade_data: atd,
            abroad_stocked_ratio: abroad_stocked_ratio,
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

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<()> {
    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).
    let names: Vec<&str> = vec!["Tritanium", "Buzzard", "Hulk"];
    let items_data: &Vec<ItemData> = &get_item_data_from_db(names);
    println!("Bulk from db:\n{:?}", items_data);

    let item_ids: &Vec<i32> = &items_data.into_iter().map(|item| item.type_id).collect();
    println!("IDIS:\n{:?}", item_ids);

    let jita_id = "60003760";
    let goon_keep_id = "1030049082711";

    let jita_trade_data = get_item_data_from_api(&jita_id, &item_ids).await;
    println!("JITA TRADE DATA:\n{:?}", jita_trade_data);

    let goon_trade_data = get_item_data_from_api(&goon_keep_id, &item_ids).await;
    println!("GOON TRADE DATA:\n{:?}", goon_trade_data);

    let merged_trade_data = merge_trade_data(
        &items_data,
        &jita_trade_data.expect("hui"),
        &goon_trade_data.expect("hui"),
    );
    println!("MERGED:\n{:?}", merged_trade_data);

    let mut extended_data_collection = vec![];
    for ele in merged_trade_data {
        let extended_item_data = ExtendedItemData::new(ele.to_owned());
        extended_data_collection.push(extended_item_data);
    }

    println!("EXTENDED DATA! \n {:?}", extended_data_collection);

    let item_view_manager = TradeItemViewManager::new(TradeItemViewManagerInitData {
        items: extended_data_collection,
    });
    // UI
    match render_ui(item_view_manager) {
        Err(_) => panic!("aaaaa"),
        _ => (),
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::goonmetrics::goonmetrics::*;
    use crate::ui::ui::FormatForDisplay;
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
