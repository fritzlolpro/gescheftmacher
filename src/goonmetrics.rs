pub mod goonmetrics {
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    pub struct Goonmetrics {
        pub price_data: PriceData,
    }

    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    pub struct PriceData {
        #[serde(rename = "$value")]
        types: Vec<Types>,
    }

    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    #[serde(rename_all = "snake_case")]
    enum Types {
        Type(ItemType),
    }

    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    pub struct ItemType {
        pub id: i32,
        pub updated: String,
        pub all: All,
        pub buy: Buy,
        pub sell: Sell,
    }

    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    pub struct All {
        pub weekly_movement: String,
    }
    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    pub struct Sell {
        pub listed: String,
        pub min: String,
    }
    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    pub struct Buy {
        pub listed: String,
        pub max: String,
    }
}
