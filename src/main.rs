use std::{any::Any, result};

use error_chain::error_chain;
use goonmetrics::goonmetrics::PriceData;
use reqwest;
use rusqlite::{Connection as SQL_Connection, Result as SQL_Result};
use serde::{Deserialize, Serialize};
use serde_xml_rs::{from_str, to_string};

use tokio;
mod goonmetrics;
use crate::goonmetrics::goonmetrics::*;
use std::path::Path;

use egui::{Key, Vec2};
use egui_extras::{Column, TableBuilder};
use struct_field_names_as_array::FieldNamesAsSlice;

const DELIVERY_PRICE_PER_CUBOMETR: f32 = 850.0;
error_chain! {
    foreign_links {
        Io(std::io::Error);
        HttpRequest(reqwest::Error);
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct ItemData {
    type_id: i32,
    type_volume: f32,
    type_name: String,
    jita_trade_data: Option<TradeData>,
    abroad_trade_data: Option<TradeData>,
}

#[derive(Debug, PartialEq, Clone)]
struct TradeData {
    updated: String,
    weekly_movement: f64,
    buy_max: f64,
    buy_listed: i64,
    sell_min: f64,
    sell_listed: i64,
}

#[derive(Debug, PartialEq, Clone)]
pub struct ManagerInitData {
    items: Vec<ExtendedItemData>,
    table_headers: Vec<String>,
}

#[derive(Debug, PartialEq, Clone)]
struct TradeItemManager {
    items: Vec<ExtendedItemData>,
    table_headers: Vec<String>,
}

pub trait DataManager {
    fn new(data: ManagerInitData) -> Self;
}

impl DataManager for TradeItemManager {
    fn new(data: ManagerInitData) -> Self {
        TradeItemManager {
            items: data.items,
            table_headers: data.table_headers,
        }
    }
}
#[derive(Debug, PartialEq, Clone, FieldNamesAsSlice)]
pub struct ExtendedItemData {
    type_id: i32,
    type_volume: f32,
    type_name: String,
    jita_trade_data: TradeData,
    abroad_trade_data: TradeData,
    shipping_price: f64,
}

pub trait BuildExtededData {
    fn new(data: ItemData) -> Self;
}

impl BuildExtededData for ExtendedItemData {
    fn new(data: ItemData) -> Self {
        let shipping_price = data.get_shipping_price();
        let jtd = data.jita_trade_data.unwrap();
        let atd = data.abroad_trade_data.unwrap();
        let id = data.type_id;
        let name = data.type_name.to_owned();
        let volume = data.type_volume;

        ExtendedItemData {
            type_id: id,
            type_volume: volume,
            type_name: name,
            jita_trade_data: jtd,
            abroad_trade_data: atd,
            shipping_price: shipping_price,
        }
    }
}

pub trait CalculateFields {
    fn get_shipping_price(&self) -> f64;
}

impl CalculateFields for ItemData {
    fn get_shipping_price(&self) -> f64 {
        let shipping_price = &self.type_volume * DELIVERY_PRICE_PER_CUBOMETR;
        return shipping_price as f64;
    }
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

async fn get_item_data_from_api(station_id: &str, item_ids: &Vec<i32>) -> Result<Vec<PriceData>> {
    let item_ids = &item_ids
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

fn merge_trade_data(
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
                Some(&goonmetrics::goonmetrics::Types::Type(ref item_type)) => {
                    enriched_item.jita_trade_data = Some(TradeData {
                        updated: item_type.updated.clone(),
                        weekly_movement: item_type
                            .all
                            .weekly_movement
                            .parse::<f64>()
                            .expect("Fail to parse"),
                        sell_listed: item_type.sell.listed.parse::<i64>().expect("Fail to parse"),
                        sell_min: item_type.sell.min.parse::<f64>().expect("Fail to parse"),
                        buy_listed: item_type.buy.listed.parse::<i64>().expect("Fail to parse"),
                        buy_max: item_type.buy.max.parse::<f64>().expect("Fail to parse"),
                    })
                }
                _ => panic!("Terrible wrong shit"),
            }

            let at = &abroad_trade_data[0].types;
            let item_abroad_trade_data = at.into_iter().find(|atd| match atd {
                Types::Type(item_type) => {
                    return item_type.id == id;
                }
            });

            match item_abroad_trade_data {
                Some(&goonmetrics::goonmetrics::Types::Type(ref item_type)) => {
                    enriched_item.abroad_trade_data = Some(TradeData {
                        updated: item_type.updated.clone(),
                        weekly_movement: item_type
                            .all
                            .weekly_movement
                            .parse::<f64>()
                            .expect("Fail to parse"),
                        sell_listed: item_type.sell.listed.parse::<i64>().expect("Fail to parse"),
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

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).
    let name = "Tritanium";
    let names: Vec<&str> = vec!["Tritanium", "Buzzard"];

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

    let mut i = vec![];
    for ele in merged_trade_data {
        let extended_item_data = ExtendedItemData::new(ele.to_owned());
        i.push(extended_item_data);
    }

    println!("EXTENDED DATA! \n {:?}", i);

    let fields = ExtendedItemData::FIELD_NAMES_AS_SLICE
        .to_owned()
        .into_iter()
        .map(|x| x.to_owned())
        .collect();
    println!("FIELDS NAMES! \n {:?}", fields);

    let test_data = String::from("test_data_external");

    let item_manager = TradeItemManager::new(ManagerInitData {
        items: i,
        table_headers: fields,
    });
    // UI
    match render_ui(item_manager) {
        Err(_) => panic!("aaaaa"),
        _ => (),
    }

    Ok(())
}

fn render_ui(item_manager: TradeItemManager) -> eframe::Result<()> {
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([400.0, 300.0])
            .with_min_inner_size([300.0, 220.0]),
        follow_system_theme: false,
        ..Default::default()
    };

    eframe::run_native(
        "eframe template",
        native_options,
        Box::new(|cc| {
            let mut app = TemplateApp::new(cc);
            app.set_data(item_manager);

            return Box::new(app);
        }),
    )
}
/// We derive Deserialize/Serialize so we can persist app state on shutdown.
#[derive(serde::Deserialize, serde::Serialize, Debug)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct TemplateApp {
    // Example stuff:
    label: String,
    #[serde(skip)] // This how you opt-out of serialization of a field
    data: Option<TradeItemManager>,
    test_data_internal: String,
    #[serde(skip)] // This how you opt-out of serialization of a field
    value: f32,
}

impl Default for TemplateApp {
    fn default() -> Self {
        Self {
            // Example stuff:
            label: "Hello World!".to_owned(),
            value: 2.7,
            test_data_internal: "test_internal_default".to_owned(),
            data: None,
        }
    }
}

trait SetData {
    fn set_data(&mut self, data: TradeItemManager);
}

impl SetData for TemplateApp {
    fn set_data(&mut self, data: TradeItemManager) {
        self.data = Some(data);
    }
}

impl TemplateApp {
    /// Called once before the first frame.
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // This is also where you can customize the look and feel of egui using
        // `cc.egui_ctx.set_visuals` and `cc.egui_ctx.set_fonts`.

        // Load previous app state (if any).
        // Note that you must enable the `persistence` feature for this to work.
        if let Some(storage) = cc.storage {
            return eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default();
        }

        Default::default()
    }
}

impl eframe::App for TemplateApp {
    /// Called by the frame work to save state before shutdown.
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }

    /// Called each time the UI needs repainting, which may be many times per second.
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Put your widgets into a `SidePanel`, `TopBottomPanel`, `CentralPanel`, `Window` or `Area`.
        // For inspiration and more examples, go to https://emilk.github.io/egui

        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            // The top panel is often a good place for a menu bar:

            egui::menu::bar(ui, |ui| {
                // NOTE: no File->Quit on web pages!
                let is_web = cfg!(target_arch = "wasm32");
                if !is_web {
                    ui.menu_button("File", |ui| {
                        if ui.button("Quit").clicked() {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                    });
                    ui.add_space(16.0);
                }

                egui::widgets::global_dark_light_mode_buttons(ui);
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            // The central panel the region left after adding TopPanel's and SidePanel's
            ui.heading("eframe template");

            ui.horizontal(|ui| {
                ui.label("Write something: ");
                ui.label(&self.test_data_internal);
                ui.text_edit_singleline(&mut self.label);
            });

            let fields = &self.data.clone().unwrap().table_headers;
            for field in fields {
                ui.horizontal(|ui: &mut egui::Ui| ui.label(field));
            }

            ui.add(egui::Slider::new(&mut self.value, 0.0..=10.0).text("value"));
            if ui.button("Increment").clicked() {
                self.value += 1.0;
            }

            ui.separator();

            ui.add(egui::github_link_file!(
                "https://github.com/emilk/eframe_template/blob/main/",
                "Source code."
            ));

            show_table(ui);

            ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                powered_by_egui_and_eframe(ui);
                egui::warn_if_debug_build(ui);
            });
        });
    }
}

fn show_table(ui: &mut egui::Ui) {
    ui.allocate_ui(Vec2 { x: 600.0, y: 600.0 }, |ui| {
        TableBuilder::new(ui)
            .columns(Column::auto().resizable(true), 6)
            .header(20.0, |mut header| {
                header.col(|ui| {
                    ui.heading("jit wk mo");
                });
                header.col(|ui| {
                    ui.heading("jit buy mx");
                });
                header.col(|ui| {
                    ui.heading("jit lis");
                });
                header.col(|ui| {
                    ui.heading("out wk mo");
                });
                header.col(|ui| {
                    ui.heading("out buy mx");
                });
                header.col(|ui| {
                    ui.heading("out lis");
                });
            })
            .body(|mut body| {
                body.row(30.0, |mut row| {
                    row.col(|ui| {
                        ui.label("Hello");
                    });
                    row.col(|ui| {
                        ui.button("world!");
                    });
                });
            });
    });
}

fn powered_by_egui_and_eframe(ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        ui.label("Powered by ");
        ui.hyperlink_to("egui", "https://github.com/emilk/egui");
        ui.label(" and ");
        ui.hyperlink_to(
            "eframe",
            "https://github.com/emilk/egui/tree/master/crates/eframe",
        );
        ui.label(".");
    });
}
#[cfg(test)]
mod tests {
    use super::*;

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
}
