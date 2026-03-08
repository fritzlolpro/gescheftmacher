export function formatNumber(value: number): string {
  return value.toFixed(2);
}

export function formatDate(dateString: string): string {
  const date = new Date(dateString);
  return date.toLocaleString();
}

type HeaderAlign = "center" | "start" | "end";
interface ChildHeader { title: string; align: HeaderAlign; value: string; sortable?: boolean }
interface Header { title: string; align: HeaderAlign; value?: string; sortable?: boolean; children?: ChildHeader[] }

export const TABLE_HEADERS: Header[] = [
  { title: 'Id',     align: 'center', value: 'type_id',     sortable: true },
  { title: 'Volume', align: 'center', value: 'type_volume', sortable: true },
  { title: 'Name',   align: 'center', value: 'type_name',   sortable: true },
  { title: 'JB Margin',       align: 'center', value: 'margin_jita_buy',  sortable: true },
  { title: 'Abroad Avg Daily', align: 'center', value: 'abroad_avg_daily', sortable: true },
  {
    title: 'Jita', align: 'center',
    children: [
      { title: 'Updated',          align: 'center', value: 'jita_trade_data.updated',         sortable: true },
      { title: 'Weekly Movement',  align: 'center', value: 'jita_trade_data.weekly_movement',  sortable: true },
      { title: 'Buy Max',          align: 'center', value: 'jita_trade_data.buy_max',          sortable: true },
      { title: 'Jita Buy with Tax',align: 'center', value: 'jita_buy_with_tax',                sortable: true },
      { title: 'Buy Listed',       align: 'center', value: 'jita_trade_data.buy_listed',       sortable: true },
      { title: 'Sell Min',         align: 'center', value: 'jita_trade_data.sell_min',         sortable: true },
      { title: 'Sell Listed',      align: 'center', value: 'jita_trade_data.sell_listed',      sortable: true },
    ]
  },
  { title: 'Shipping Price', align: 'center', value: 'shipping_price', sortable: true },
  {
    title: 'Abroad', align: 'center',
    children: [
      { title: 'Updated',             align: 'center', value: 'abroad_trade_data.updated',         sortable: true },
      { title: 'Weekly Movement',     align: 'center', value: 'abroad_trade_data.weekly_movement',  sortable: true },
      { title: 'Buy Max',             align: 'center', value: 'abroad_trade_data.buy_max',          sortable: true },
      { title: 'Buy Listed',          align: 'center', value: 'abroad_trade_data.buy_listed',       sortable: true },
      { title: 'Sell Min',            align: 'center', value: 'abroad_trade_data.sell_min',         sortable: true },
      { title: 'Sell Listed',         align: 'center', value: 'abroad_trade_data.sell_listed',      sortable: true },
      { title: 'Abroad Stocked Ratio',align: 'center', value: 'abroad_stocked_ratio',               sortable: true },
      { title: 'Abroad Sell Taxed',   align: 'center', value: 'abroad_sell_taxed',                  sortable: true },
    ]
  },
  { title: 'Money Freeze Buy', align: 'center', value: 'money_freeze_buy', sortable: true },
  { title: 'Freeze Rate',      align: 'center', value: 'freeze_rate',      sortable: true },
];
