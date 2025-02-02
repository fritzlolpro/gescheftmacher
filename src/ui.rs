pub mod ui {
    use crate::datagetter::datagetter::TradeData;
    use crate::ExtendedItemData;
    use egui::Vec2;
    use egui_extras::{Column, TableBuilder};
    use struct_field_names_as_array::FieldNamesAsSlice;

    use numfmt::Formatter;
    use numfmt::Precision;
    #[derive(Debug, PartialEq, Clone)]
    pub struct TradeItemViewManagerInitData {
        pub items: Vec<ExtendedItemData>,
    }
    #[derive(Debug, PartialEq, Clone)]
    pub struct TradeItemViewManager {
        items: Vec<ExtendedItemData>,
        table_headers: Vec<String>,
        table_rows: Vec<Vec<String>>,
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

    impl TradeItemViewManager {
        pub fn new(data: TradeItemViewManagerInitData) -> Self {
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
                                    "updated" => {
                                        row.push(entity.jita_trade_data.updated.to_string())
                                    }
                                    "weekly_movement" => row.push(
                                        entity.jita_trade_data.weekly_movement.format_for_display(),
                                    ),
                                    "buy_max" => row
                                        .push(entity.jita_trade_data.buy_max.format_for_display()),
                                    "buy_listed" => row.push(
                                        entity.jita_trade_data.buy_listed.format_for_display(),
                                    ),
                                    "sell_min" => row
                                        .push(entity.jita_trade_data.sell_min.format_for_display()),
                                    "sell_listed" => row.push(
                                        entity.jita_trade_data.sell_listed.format_for_display(),
                                    ),
                                    _ => panic!("SOME FIELDS MISSING!"),
                                }
                            }
                        }
                        "abroad_trade_data" => {
                            for tdf in &trade_data_fields {
                                match tdf.as_str() {
                                    "updated" => {
                                        row.push(entity.abroad_trade_data.updated.to_string())
                                    }
                                    "weekly_movement" => row.push(
                                        entity
                                            .abroad_trade_data
                                            .weekly_movement
                                            .format_for_display(),
                                    ),
                                    "buy_max" => row.push(
                                        entity.abroad_trade_data.buy_max.format_for_display(),
                                    ),
                                    "buy_listed" => row.push(
                                        entity.abroad_trade_data.buy_listed.format_for_display(),
                                    ),
                                    "sell_min" => row.push(
                                        entity.abroad_trade_data.sell_min.format_for_display(),
                                    ),
                                    "sell_listed" => row.push(
                                        entity.abroad_trade_data.sell_listed.format_for_display(),
                                    ),
                                    _ => panic!("SOME FIELDS MISSING!"),
                                }
                            }
                        }
                        "abroad_stocked_ratio" => {
                            row.push(entity.abroad_stocked_ratio.format_for_display_percentage())
                        }
                        "shipping_price" => row.push(entity.shipping_price.format_for_display()),
                        "jita_buy_with_tax" => {
                            row.push(entity.jita_buy_with_tax.format_for_display())
                        }
                        "abroad_sell_taxed" => {
                            row.push(entity.abroad_sell_taxed.format_for_display())
                        }
                        "abroad_avg_daily" => {
                            row.push(entity.abroad_avg_daily.format_for_display())
                        }
                        "profit_jita_buy_per_unit" => {
                            row.push(entity.profit_jita_buy_per_unit.format_for_display())
                        }
                        "profit_jita_buy_daily" => {
                            row.push(entity.profit_jita_buy_daily.format_for_display())
                        }
                        "margin_jita_buy" => row.push(entity.margin_jita_buy.format_for_display()),
                        "money_freeze_buy" => {
                            row.push(entity.money_freeze_buy.format_for_display())
                        }
                        "freeze_rate" => row.push(entity.freeze_rate.format_for_display()),
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

    pub fn render_ui(item_view_manager: TradeItemViewManager) -> eframe::Result<()> {
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
        // TODO: add filtering to ui by adding checkboxes
        let filtered = vec![
            "shipping_price".to_owned(),
            "profit_jita_buy_per_unit".to_owned(),
            "type_id".to_owned(),
            "type_volume".to_owned(),
        ];

    

        let column_quantity = ctx.data.clone().unwrap().table_headers.len() - filtered.len();
        let mut headers = ctx.data.clone().unwrap().table_headers;
        let mut rows = ctx.data.clone().unwrap().table_rows;

        filtered.iter().for_each(|f| {
            if let Some(index) = headers.iter().position(|h| h == f) {
                headers.remove(index);
                rows.iter_mut().for_each(|r| {
                    r.remove(index);
                });
                
            }
        });

     
        egui::ScrollArea::both().show(ui, |ui| {
            TableBuilder::new(ui)
                .striped(true)
                .sense(egui::Sense::click())
                .columns(
                    Column::auto().resizable(true).at_least(100.0),
                    column_quantity,
                )
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
                })
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
}

#[cfg(test)]
mod tests {
    use numfmt::Formatter;

    use crate::ui::ui::FormatForDisplay;

    #[test]
    fn format_thouthands() {
        let mut f: Formatter;
        f = "[n/ ]".parse().unwrap();
        assert_eq!("1 000.5", f.fmt2(1000.5));
    }

    #[test]
    fn format_small() {
        let mut f: Formatter;
        f = "[n/ ]".parse().unwrap();
        assert_eq!("4.85", f.fmt2(4.85));
    }

    #[test]
    fn format_as_trait() {
        assert_eq!("4.85", 4.85.format_for_display())
    }
    #[test]
    fn long_no_float() {
        assert_eq!("1 000 000.0", 1000000.00.format_for_display())
    }
    #[test]
    fn long_with_float() {
        assert_eq!("1 000 000.55", 1000000.55.format_for_display())
    }
}
