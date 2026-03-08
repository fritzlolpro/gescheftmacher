import { ref, computed } from 'vue'

const numberFormatter = new Intl.NumberFormat('ru-RU', {
  minimumFractionDigits: 2,
  maximumFractionDigits: 2,
})

export function formatNumber(value: number): string {
  return numberFormatter.format(value);
}

export function formatDate(dateString: string): string {
  const date = new Date(dateString);
  return date.toLocaleString();
}

export interface SortItem { key: string; order: 'asc' | 'desc' }

function getNestedValue(obj: unknown, path: string): unknown {
  return path.split('.').reduce(
    (acc: unknown, key) => acc != null ? (acc as Record<string, unknown>)[key] : undefined,
    obj
  );
}

export function sortItems<T>(items: T[], sortBy: SortItem[]): T[] {
  if (!sortBy.length) return items;
  return [...items].sort((a, b) => {
    for (const { key, order } of sortBy) {
      const aVal = getNestedValue(a, key);
      const bVal = getNestedValue(b, key);
      const mult = order === 'desc' ? -1 : 1;
      if (aVal == null && bVal == null) continue;
      if (aVal == null) return mult;
      if (bVal == null) return -mult;
      if (aVal < bVal) return -mult;
      if (aVal > bVal) return mult;
    }
    return 0;
  });
}

/** Обрабатывает клик по столбцу: добавляет/цикличит/удаляет из multi-sort без Shift. */
export function applyColumnClick(sortBy: SortItem[], key: string): SortItem[] {
  const idx = sortBy.findIndex(s => s.key === key);
  if (idx < 0) return [...sortBy, { key, order: 'asc' }];
  if (sortBy[idx].order === 'asc') return sortBy.map(s => s.key === key ? { key, order: 'desc' } : s);
  return sortBy.filter(s => s.key !== key);
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

export function useCollapsibleHeaders() {
  const collapsed = ref(new Set<string>())

  const groupTitles = TABLE_HEADERS
    .filter(h => h.children)
    .map(h => h.title)

  const headers = computed(() =>
    TABLE_HEADERS.filter(h => !(h.children && collapsed.value.has(h.title)))
  )

  function toggleGroup(title: string) {
    const s = new Set(collapsed.value)
    s.has(title) ? s.delete(title) : s.add(title)
    collapsed.value = s
  }

  function isCollapsed(title: string) {
    return collapsed.value.has(title)
  }

  return { headers, toggleGroup, isCollapsed, groupTitles }
}
