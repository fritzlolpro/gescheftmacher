pub mod goonmetrics {
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    pub struct Goonmetrics {
        pub price_data: PriceData,
    }

    #[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
    pub struct PriceData {
        #[serde(rename = "$value")]
        pub types: Vec<Types>,
    }

    #[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
    #[serde(rename_all = "snake_case")]
    pub enum Types {
        Type(ItemType),
    }

    #[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
    pub struct ItemType {
        pub id: i32,
        pub updated: String,
        pub all: All,
        pub buy: Buy,
        pub sell: Sell,
    }

    #[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
    pub struct All {
        pub weekly_movement: String,
    }
    #[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
    pub struct Sell {
        pub listed: String,
        pub min: String,
    }
    #[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
    pub struct Buy {
        pub listed: String,
        pub max: String,
    }
}
