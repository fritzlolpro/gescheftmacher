<script setup lang="ts">
import { ref, computed } from "vue";
import { invoke } from "@tauri-apps/api/core";
import type { ExtendedItemData } from "./types";
import { TABLE_HEADERS, formatNumber, formatDate, sortItems, applyColumnClick } from "./tradeUtils";
import type { SortItem } from "./tradeUtils";
import ValeraMode from "./valeraMode.vue";

const search = ref("")
const tab = ref('main')
const slider = ref(30)
const minMargin = 0

const items = ref<ExtendedItemData[]>([]);

function truncateDecimal(number: number) {
  return Number(number.toFixed(2));
}

async function get_data() {
  const raw_data = await invoke("get_data") as string;
  const parsed = JSON.parse(raw_data)
  items.value = typeof parsed === 'string' ? JSON.parse(parsed) : parsed;
  items.value.forEach(item => {
    if (typeof item.margin_jita_buy === 'number') {
      item.margin_jita_buy = truncateDecimal(item.margin_jita_buy * 100)
    }
  })
}

const mainSortBy = ref<SortItem[]>([{ key: 'margin_jita_buy', order: 'desc' }])

const filteredItems = computed(() => {
  const filtered = items.value.filter(item => item.margin_jita_buy >= slider.value)
  return sortItems(filtered, mainSortBy.value)
})

function onMainSortUpdate(newSort: SortItem[]) {
  if (!newSort.length) { mainSortBy.value = []; return; }
  if (newSort.length === 1) {
    mainSortBy.value = applyColumnClick(mainSortBy.value, newSort[0].key)
  } else {
    mainSortBy.value = newSort
  }
}

const clickedElement = ref<string | null>(null);

function handleElementClick(text: string) {
  clickedElement.value = text;
  navigator.clipboard.writeText(text).catch(err => {
    console.error('Failed to copy text: ', err);
  });
  setTimeout(() => { clickedElement.value = null }, 1000);
}
</script>

<template>
  <main class="container">
    <v-tabs v-model="tab" bg-color="primary">
      <v-tab value="main">Main</v-tab>
      <v-tab value="valera">VALERA_MOD</v-tab>
    </v-tabs>

    <v-window v-model="tab">

      <!-- ===== MAIN TAB ===== -->
      <v-window-item value="main">
        <form class="row" @submit.prevent="get_data">
          <button type="submit">Greet</button>
        </form>
        <v-text-field v-model="search" label="Search" prepend-inner-icon="mdi-magnify" variant="outlined" hide-details
          single-line></v-text-field>
        <v-slider v-model="slider" :min="minMargin" class="align-center" hide-details>
          <template v-slot:append>
            <v-text-field v-model="slider" density="compact" type="number" hide-details single-line></v-text-field>
          </template>
        </v-slider>
        <v-data-table items-per-page="-1" :sort-by="mainSortBy" @update:sort-by="onMainSortUpdate" multi-sort
          :search="search" density="compact" :headers="TABLE_HEADERS" :items="filteredItems">
          <template v-slot:item.type_name="{ item }">
            <span @click="handleElementClick(item.type_name)"
              :class="{ 'clickable': true, 'clicked': clickedElement === item.type_name }">
              {{ item.type_name }}
            </span>
          </template>
          <template v-slot:item.type_volume="{ item }">
            <span>{{ formatNumber(item.type_volume) }}</span>
          </template>
          <template v-slot:item.margin_jita_buy="{ item }">
            <span>{{ formatNumber(item.margin_jita_buy) }}</span>
          </template>
          <template v-slot:item.jita_trade_data.buy_max="{ item }">
            <span>{{ formatNumber(item.jita_trade_data.buy_max) }}</span>
          </template>
          <template v-slot:item.jita_trade_data.sell_min="{ item }">
            <span>{{ formatNumber(item.jita_trade_data.sell_min) }}</span>
          </template>
          <template v-slot:item.jita_buy_with_tax="{ item }">
            <span>{{ formatNumber(item.jita_buy_with_tax) }}</span>
          </template>
          <template v-slot:item.abroad_trade_data.buy_max="{ item }">
            <span>{{ formatNumber(item.abroad_trade_data.buy_max) }}</span>
          </template>
          <template v-slot:item.abroad_trade_data.sell_min="{ item }">
            <span>{{ formatNumber(item.abroad_trade_data.sell_min) }}</span>
          </template>
          <template v-slot:item.abroad_stocked_ratio="{ item }">
            <span>{{ formatNumber(item.abroad_stocked_ratio) }}</span>
          </template>
          <template v-slot:item.shipping_price="{ item }">
            <span>{{ formatNumber(item.shipping_price) }}</span>
          </template>
          <template v-slot:item.abroad_sell_taxed="{ item }">
            <span>{{ formatNumber(item.abroad_sell_taxed) }}</span>
          </template>
          <template v-slot:item.abroad_avg_daily="{ item }">
            <span>{{ formatNumber(item.abroad_avg_daily) }}</span>
          </template>
          <template v-slot:item.profit_jita_buy_per_unit="{ item }">
            <span>{{ formatNumber(item.profit_jita_buy_per_unit) }}</span>
          </template>
          <template v-slot:item.profit_jita_buy_daily="{ item }">
            <span>{{ formatNumber(item.profit_jita_buy_daily) }}</span>
          </template>
          <template v-slot:item.money_freeze_buy="{ item }">
            <span>{{ formatNumber(item.money_freeze_buy) }}</span>
          </template>
          <template v-slot:item.freeze_rate="{ item }">
            <span>{{ formatNumber(item.freeze_rate) }}</span>
          </template>
          <template v-slot:item.jita_trade_data.updated="{ item }">
            <span>{{ formatDate(item.jita_trade_data.updated) }}</span>
          </template>
          <template v-slot:item.abroad_trade_data.updated="{ item }">
            <span>{{ formatDate(item.abroad_trade_data.updated) }}</span>
          </template>
        </v-data-table>
      </v-window-item>

      <!-- ===== VALERA_MOD TAB ===== -->
      <v-window-item value="valera" eager>
        <ValeraMode :items="items" />
      </v-window-item>

    </v-window>
  </main>
</template>

<style scoped>
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