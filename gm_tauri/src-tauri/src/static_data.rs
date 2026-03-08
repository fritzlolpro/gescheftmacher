pub mod static_data {

    pub struct TradePile {
        pub items: Vec<String>,
    }
    impl TradePile {
        pub fn new() -> Self {
            TradePile {
                items: vec![
                    "Active Shielding Charge"
                ]
                .into_iter()
                .map(|s| s.to_owned())
                .collect(),
            }
        }
    }
}
