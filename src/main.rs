use serde::{Deserialize, Serialize};
use serde_xml_rs::{from_str, to_string};

use egui::{Key, Vec2};
use egui_extras::{Column, TableBuilder};
use error_chain::error_chain;
use struct_field_names_as_array::FieldNamesAsSlice;
use tokio;

mod datagetter;
mod goonmetrics;
use datagetter::datagetter::{
    get_item_data_from_api, get_item_data_from_db, merge_trade_data, ItemData, TradeData,
};
const DELIVERY_PRICE_PER_CUBOMETR: f32 = 850.0;
const MIN_SELL_MARGIN: f32 = 1.15;
const JITA_TAXRATE: f64 = 0.0108;
const PROFIT_THRESHOLD: i64 = 30000000;
const FREEZE_RATE_THRESHOLD: f32 = 0.1;
const MARKET_RATE: i32 = 1;
const DAILY_VOL: i64 = 10;
error_chain! {
    foreign_links {
        Io(std::io::Error);
        HttpRequest(reqwest::Error);
    }
}
#[derive(Debug, PartialEq, Clone)]
pub struct ManagerInitData {
    items: Vec<ExtendedItemData>,
}

#[derive(Debug, PartialEq, Clone)]
struct TradeItemViewManager {
    items: Vec<ExtendedItemData>,
    table_headers: Vec<String>,
    table_rows: Vec<Vec<String>>,
}

impl TradeItemViewManager {
    fn new(data: ManagerInitData) -> Self {
        let trade_data_fields: Vec<String> = TradeData::FIELD_NAMES_AS_SLICE
            .to_owned()
            .into_iter()
            .map(|x| x.to_owned())
            .collect();

        let extended_data_fields: Vec<String> = ExtendedItemData::FIELD_NAMES_AS_SLICE
            .to_owned()
            .into_iter()
            .map(|x| x.to_owned())
            .collect();

        let mut table_headers = vec![];

        for ef in &extended_data_fields {
            if ef.to_string() == "jita_trade_data".to_owned() {
                for tdf in &trade_data_fields {
                    match tdf.as_str() {
                        "updated" => table_headers.push("j_upd".to_owned()),
                        "weekly_movement" => table_headers.push("j_wkmov".to_owned()),
                        "buy_max" => table_headers.push("j_buy".to_owned()),
                        "buy_listed" => table_headers.push("j_buylist".to_owned()),
                        "sell_min" => table_headers.push("j_sell".to_owned()),
                        "sell_listed" => table_headers.push("j_selllist".to_owned()),
                        _ => table_headers.push(tdf.to_owned()),
                    }
                }
            } else if ef.to_string() == "abroad_trade_data".to_owned() {
                for tdf in &trade_data_fields {
                    match tdf.as_str() {
                        "updated" => table_headers.push("ab_upd".to_owned()),
                        "weekly_movement" => table_headers.push("ab_wkmov".to_owned()),
                        "buy_max" => table_headers.push("ab_buy".to_owned()),
                        "buy_listed" => table_headers.push("ab_buylist".to_owned()),
                        "sell_min" => table_headers.push("ab_sell".to_owned()),
                        "sell_listed" => table_headers.push("ab_selllist".to_owned()),
                        _ => table_headers.push(tdf.to_owned()),
                    }
                }
            } else {
                table_headers.push(ef.to_owned())
            }
        }

        let mut table_rows: Vec<Vec<String>> = vec![];

        for entity in &data.items {
            let mut row: Vec<String> = vec![];
            for field in &extended_data_fields {
                match field.as_str() {
                    "type_id" => row.push(entity.type_id.to_string()),
                    "type_volume" => row.push(entity.type_volume.to_string()),
                    "type_name" => row.push(entity.type_name.to_string()),
                    "jita_trade_data" => {
                        for tdf in &trade_data_fields {
                            match tdf.as_str() {
                                "updated" => row.push(entity.jita_trade_data.updated.to_string()),
                                "weekly_movement" => {
                                    row.push(entity.jita_trade_data.weekly_movement.to_string())
                                }
                                "buy_max" => row.push(entity.jita_trade_data.buy_max.to_string()),
                                "buy_listed" => {
                                    row.push(entity.jita_trade_data.buy_listed.to_string())
                                }
                                "sell_min" => row.push(entity.jita_trade_data.sell_min.to_string()),
                                "sell_listed" => {
                                    row.push(entity.jita_trade_data.sell_listed.to_string())
                                }
                                _ => panic!("SOME FIELDS MISSING!"),
                            }
                        }
                    }
                    "abroad_trade_data" => {
                        for tdf in &trade_data_fields {
                            match tdf.as_str() {
                                "updated" => row.push(entity.abroad_trade_data.updated.to_string()),
                                "weekly_movement" => {
                                    row.push(entity.abroad_trade_data.weekly_movement.to_string())
                                }
                                "buy_max" => row.push(entity.abroad_trade_data.buy_max.to_string()),
                                "buy_listed" => {
                                    row.push(entity.abroad_trade_data.buy_listed.to_string())
                                }
                                "sell_min" => {
                                    row.push(entity.abroad_trade_data.sell_min.to_string())
                                }
                                "sell_listed" => {
                                    row.push(entity.abroad_trade_data.sell_listed.to_string())
                                }
                                _ => panic!("SOME FIELDS MISSING!"),
                            }
                        }
                    }
                    "shipping_price" => row.push(entity.shipping_price.to_string()),
                    "jita_buy_with_tax" => row.push(entity.jita_buy_with_tax.to_string()),
                    _ => panic!("SOME h-lvl probably custom fields missing!"),
                }
            }
            table_rows.push(row)
        }
        TradeItemViewManager {
            items: data.items,
            table_headers: table_headers,
            table_rows: table_rows,
        }
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
    shipping_price: f64,
}

impl ItemData {
    pub fn get_shipping_price(&self) -> f64 {
        let shipping_price = &self.type_volume * DELIVERY_PRICE_PER_CUBOMETR;
        return shipping_price as f64;
    }
    pub fn get_buy_price_with_tax(&self) -> f64 {
        let jtd = &self.jita_trade_data.as_ref().unwrap();
        return jtd.buy_max * JITA_TAXRATE + jtd.buy_max;
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
        let jtb_with_tax = data.get_buy_price_with_tax();

        ExtendedItemData {
            type_id: id,
            type_volume: volume,
            type_name: name,
            jita_trade_data: jtd,
            jita_buy_with_tax: jtb_with_tax,
            abroad_trade_data: atd,
            shipping_price: shipping_price,
        }
    }
}

// TODO: move to datagetter
#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).
    let names: Vec<&str> = vec!["Tritanium", "Buzzard"];
    // TODO: mod data getter
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

    // END mod data getter

    let mut extended_data_collection = vec![];
    for ele in merged_trade_data {
        let extended_item_data = ExtendedItemData::new(ele.to_owned());
        extended_data_collection.push(extended_item_data);
    }

    println!("EXTENDED DATA! \n {:?}", extended_data_collection);

    let item_view_manager = TradeItemViewManager::new(ManagerInitData {
        items: extended_data_collection,
    });
    // UI
    match render_ui(item_view_manager) {
        Err(_) => panic!("aaaaa"),
        _ => (),
    }

    Ok(())
}

fn render_ui(item_view_manager: TradeItemViewManager) -> eframe::Result<()> {
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
            app.set_data(item_view_manager);

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
    data: Option<TradeItemViewManager>,
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
    fn set_data(&mut self, data: TradeItemViewManager);
}

impl SetData for TemplateApp {
    fn set_data(&mut self, data: TradeItemViewManager) {
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

            // let fields = &self.data.clone().unwrap().table_headers;
            // // for field in fields {
            //     ui.horizontal(|ui: &mut egui::Ui| ui.label(field));
            // }

            ui.add(egui::Slider::new(&mut self.value, 0.0..=10.0).text("value"));
            if ui.button("Increment").clicked() {
                self.value += 1.0;
            }

            ui.separator();

            ui.add(egui::github_link_file!(
                "https://github.com/emilk/eframe_template/blob/main/",
                "Source code."
            ));

            show_table(self, ui);

            ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                powered_by_egui_and_eframe(ui);
                egui::warn_if_debug_build(ui);
            });
        });
    }
}

fn show_table(ctx: &mut TemplateApp, ui: &mut egui::Ui) {
    ui.allocate_ui(Vec2 { x: 600.0, y: 600.0 }, |ui| {
        let column_quantity = ctx.data.clone().unwrap().table_headers.len();
        let headers = ctx.data.clone().unwrap().table_headers;
        let rows = ctx.data.clone().unwrap().table_rows;
        TableBuilder::new(ui)
            .columns(Column::auto().resizable(true), column_quantity)
            .header(20.0, |mut header| {
                for h in headers {
                    header.col(|ui| {
                        ui.heading(h);
                    });
                }
            })
            .body(|mut body| {
                for r in rows {
                    body.row(30.0, |mut row| {
                        for cell in r {
                            row.col(|ui| {
                                ui.label(cell);
                            });
                        }
                    });
                }
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
}
