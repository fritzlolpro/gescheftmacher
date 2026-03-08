export interface TradeData {
  updated: string;
  weekly_movement: number;
  buy_max: number;
  buy_listed: number;
  sell_min: number;
  sell_listed: number;
}

export interface ExtendedItemData {
  type_id: number;
  type_volume: number;
  timestamp: number;
  type_name: string;
  jita_trade_data: TradeData;
  jita_buy_with_tax: number;
  abroad_trade_data: TradeData;
  abroad_stocked_ratio: number;
  shipping_price: number;
  abroad_sell_taxed: number;
  abroad_avg_daily: number;
  profit_jita_buy_per_unit: number;
  profit_jita_buy_daily: number;
  margin_jita_buy: number;
  money_freeze_buy: number;
  freeze_rate: number;
  [key: string]: number | string | TradeData;
}
