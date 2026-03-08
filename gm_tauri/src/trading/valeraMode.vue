<script setup lang="ts">
import { ref, computed } from "vue";
import type { ExtendedItemData } from "./types";
import { formatNumber, formatDate, sortItems, applyColumnClick, useCollapsibleHeaders } from "./tradeUtils";
import type { SortItem } from "./tradeUtils";

const props = defineProps<{
  items: ExtendedItemData[]
}>()

const VALERA_SETTINGS = {
  minSellMargin: 15,
  profitThreshold: 30_000_000,
  freezeRateThreshold: 0.1,
  dailyVol: 1,
  marketRate: 0.5,
  deliveryPerM3: 1200,
  jitaTaxRate: 0.015,
}

const sortBy = ref<SortItem[]>([{ key: 'abroad_avg_daily', order: 'desc' }])

const { headers: activeHeaders, toggleGroup, isCollapsed, groupTitles } = useCollapsibleHeaders()

const valeraItems = computed(() => {
  const filtered = props.items.filter(item =>
    item.margin_jita_buy >= VALERA_SETTINGS.minSellMargin &&
    item.profit_jita_buy_daily >= VALERA_SETTINGS.profitThreshold &&
    item.freeze_rate >= VALERA_SETTINGS.freezeRateThreshold &&
    item.abroad_avg_daily >= VALERA_SETTINGS.dailyVol
  )
  return sortItems(filtered, sortBy.value)
})

const totalMoneyFreeze = computed(() =>
  valeraItems.value.reduce((sum: number, item: ExtendedItemData) => sum + (item.money_freeze_buy as number), 0)
)
const totalDailyProfit = computed(() =>
  valeraItems.value.reduce((sum: number, item: ExtendedItemData) => sum + (item.profit_jita_buy_daily as number), 0)
)
const totalVolume = computed(() =>
  valeraItems.value.reduce((sum: number, item: ExtendedItemData) => sum + (item.type_volume as number) * (item.abroad_avg_daily as number), 0)
)

function onSortUpdate(newSort: SortItem[]) {
  if (!newSort.length) { sortBy.value = []; return; }
  if (newSort.length === 1) {
    sortBy.value = applyColumnClick(sortBy.value, newSort[0].key)
  } else {
    sortBy.value = newSort
  }
}

const clickedElement = ref<string | null>(null)

function handleElementClick(text: string) {
  clickedElement.value = text
  navigator.clipboard.writeText(text).catch(err => {
    console.error('Failed to copy text: ', err)
  })
  setTimeout(() => { clickedElement.value = null }, 1000)
}
</script>

<template>
  <div class="settings-row">
    <v-chip size="small" color="blue-lighten-4">Min Margin: {{ VALERA_SETTINGS.minSellMargin }}%</v-chip>
    <v-chip size="small" color="blue-lighten-4">Daily Profit ≥ {{ (VALERA_SETTINGS.profitThreshold /
      1_000_000).toFixed(0) }}M</v-chip>
    <v-chip size="small" color="blue-lighten-4">Freeze Rate ≥ {{ VALERA_SETTINGS.freezeRateThreshold }}</v-chip>
    <v-chip size="small" color="blue-lighten-4">Daily Vol ≥ {{ VALERA_SETTINGS.dailyVol }}</v-chip>
    <v-chip size="small" color="grey-lighten-2">Delivery: {{ VALERA_SETTINGS.deliveryPerM3 }} ISK/m³</v-chip>
    <v-chip size="small" color="grey-lighten-2">Jita tax: {{ VALERA_SETTINGS.jitaTaxRate }}</v-chip>
    <v-chip size="small" color="green-lighten-3">{{ valeraItems.length }} items</v-chip>
    <v-divider vertical class="mx-1" />
    <v-chip size="small" color="orange-lighten-3">Freeze: {{ formatNumber(totalMoneyFreeze) }} ISK</v-chip>
    <v-chip size="small" color="purple-lighten-3">Daily Vol: {{ formatNumber(totalVolume) }} m³</v-chip>
    <v-chip size="small" color="green-darken-1" text-color="white">Profit/day: {{ formatNumber(totalDailyProfit) }}
      ISK</v-chip>
    <v-divider vertical class="mx-1" />
    <v-chip v-for="g in groupTitles" :key="g" size="small"
      :color="isCollapsed(g) ? 'grey-lighten-2' : 'blue-grey-lighten-3'" @click="toggleGroup(g)" style="cursor:pointer">
      {{ isCollapsed(g) ? '▶' : '▼' }} {{ g }}
    </v-chip>
  </div>

  <v-data-table items-per-page="-1" :sort-by="sortBy" @update:sort-by="onSortUpdate" multi-sort density="compact"
    :headers="activeHeaders" :items="valeraItems">
    <template v-slot:item.type_name="{ item }">
      <span @click="handleElementClick(item.type_name)"
        :class="{ 'clickable': true, 'clicked': clickedElement === item.type_name }">{{ item.type_name }}</span>
    </template>
    <template v-slot:item.type_volume="{ item }"><span>{{ formatNumber(item.type_volume) }}</span></template>
    <template v-slot:item.margin_jita_buy="{ item }"><span>{{ formatNumber(item.margin_jita_buy) }}</span></template>
    <template v-slot:item.jita_trade_data.buy_max="{ item }"><span>{{ formatNumber(item.jita_trade_data.buy_max)
        }}</span></template>
    <template v-slot:item.jita_trade_data.sell_min="{ item }"><span>{{ formatNumber(item.jita_trade_data.sell_min)
        }}</span></template>
    <template v-slot:item.jita_buy_with_tax="{ item }"><span>{{ formatNumber(item.jita_buy_with_tax)
        }}</span></template>
    <template v-slot:item.abroad_trade_data.buy_max="{ item }"><span>{{ formatNumber(item.abroad_trade_data.buy_max)
        }}</span></template>
    <template v-slot:item.abroad_trade_data.sell_min="{ item }"><span>{{ formatNumber(item.abroad_trade_data.sell_min)
        }}</span></template>
    <template v-slot:item.abroad_stocked_ratio="{ item }"><span>{{ formatNumber(item.abroad_stocked_ratio)
        }}</span></template>
    <template v-slot:item.shipping_price="{ item }"><span>{{ formatNumber(item.shipping_price) }}</span></template>
    <template v-slot:item.abroad_sell_taxed="{ item }"><span>{{ formatNumber(item.abroad_sell_taxed)
        }}</span></template>
    <template v-slot:item.abroad_avg_daily="{ item }"><span>{{ formatNumber(item.abroad_avg_daily) }}</span></template>
    <template v-slot:item.profit_jita_buy_per_unit="{ item }"><span>{{ formatNumber(item.profit_jita_buy_per_unit)
        }}</span></template>
    <template v-slot:item.profit_jita_buy_daily="{ item }"><span>{{ formatNumber(item.profit_jita_buy_daily)
        }}</span></template>
    <template v-slot:item.money_freeze_buy="{ item }"><span>{{ formatNumber(item.money_freeze_buy) }}</span></template>
    <template v-slot:item.freeze_rate="{ item }"><span>{{ formatNumber(item.freeze_rate) }}</span></template>
    <template v-slot:item.jita_trade_data.updated="{ item }"><span>{{ formatDate(item.jita_trade_data.updated)
        }}</span></template>
    <template v-slot:item.abroad_trade_data.updated="{ item }"><span>{{ formatDate(item.abroad_trade_data.updated)
        }}</span></template>
  </v-data-table>
</template>

<style scoped>
.settings-row {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
  padding: 8px 0;
}

.clickable {
  cursor: pointer;
  color: blue;
}

.clickable:hover {
  color: darkblue;
  text-decoration: underline;
}

.clicked {
  background-color: yellow;
}
</style>
