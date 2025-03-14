<script setup lang="ts">
import { ref, computed } from "vue";
import { invoke } from "@tauri-apps/api/core";

interface TradeData {
  updated: string;
  weekly_movement: number;
  buy_max: number;
  buy_listed: number;
  sell_min: number;
  sell_listed: number;
}

interface ExtendedItemData {
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
}

const search = ref("")
const headers = ref<{ title: string; align: "center" | "start" | "end"; value?: string; children?: { title: string; align: "center" | "start" | "end"; value: string; }[] }[]>([
  // Basic information
  { title: 'Id', align: 'center', value: 'type_id' },
  { title: 'Volume', align: 'center', value: 'type_volume' },
  { title: 'Name', align: 'center', value: 'type_name' },
  {
    title: 'JB Margin',
    align: 'center',
    value: 'margin_jita_buy'
  },

  // Jita trade data section
  {
    title: 'Jita',
    align: 'center',
    children: [
      {
        title: 'Updated',
        align: 'center',
        value: 'jita_trade_data.updated'
      },
      {
        title: 'Weekly Movement',
        align: 'center',
        value: 'jita_trade_data.weekly_movement'
      },
      {
        title: 'Buy Max',
        align: 'center',
        value: 'jita_trade_data.buy_max'
      },
      {
        title: 'Jita Buy with Tax',
        align: 'center',
        value: 'jita_buy_with_tax'
      },
      {
        title: 'Buy Listed',
        align: 'center',
        value: 'jita_trade_data.buy_listed'
      },
      {
        title: 'Sell Min',
        align: 'center',
        value: 'jita_trade_data.sell_min'
      },
      {
        title: 'Sell Listed',
        align: 'center',
        value: 'jita_trade_data.sell_listed'
      }
    ]
  },
  {
    title: 'Shipping Price',
    align: 'center',
    value: 'shipping_price'
  },
  // Abroad trade data section
  {
    title: 'Abroad',
    align: 'center',
    children: [
      {
        title: 'Updated',
        align: 'center',
        value: 'abroad_trade_data.updated'
      },
      {
        title: 'Weekly Movement',
        align: 'center',
        value: 'abroad_trade_data.weekly_movement'
      },
      {
        title: 'Buy Max',
        align: 'center',
        value: 'abroad_trade_data.buy_max'
      },
      {
        title: 'Buy Listed',
        align: 'center',
        value: 'abroad_trade_data.buy_listed'
      },
      {
        title: 'Sell Min',
        align: 'center',
        value: 'abroad_trade_data.sell_min'
      },
      {
        title: 'Sell Listed',
        align: 'center',
        value: 'abroad_trade_data.sell_listed'
      },
      {
        title: 'Abroad Stocked Ratio',
        align: 'center',
        value: 'abroad_stocked_ratio'
      },
      {
        title: 'Abroad Sell Taxed',
        align: 'center',
        value: 'abroad_sell_taxed'
      },
      {
        title: 'Abroad Avg Daily',
        align: 'center',
        value: 'abroad_avg_daily'
      }
    ]
  },
  {
    title: 'Money Freeze Buy',
    align: 'center',
    value: 'money_freeze_buy'
  },
  {
    title: 'Freeze Rate',
    align: 'center',
    value: 'freeze_rate'
  },


]);

const items = ref<ExtendedItemData[]>([]);


function truncateDecimal(number: number) {
  return Number(number.toFixed(2));
}

async function get_data() {
  let raw_data = await invoke("get_data") as string;
  const parsed = JSON.parse(raw_data)
  items.value = typeof parsed === 'string' ? JSON.parse(parsed) : parsed;

  items.value.forEach(item => {
    for (const [key, value] of Object.entries(item)) {
      if (key === 'margin_jita_buy') {
        item[key] = truncateDecimal(value as number * 100)
      }
    }

  })
  console.log(items.value)
}


const filteredItems = computed(() => {
  console.log(items.value)
  return items.value.filter(item => item.margin_jita_buy >= slider.value)
})

const slider = ref(30)
const minMargin = 0
</script>

<template>
  <main class="container">
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
    <v-data-table :sort-by="[{ key: 'margin_jita_buy', order: 'desc' }, { key: 'freeze_rate', order: 'desc' }]"
      multi-sort :search="search" density="compact" :headers="headers" :items="items"></v-data-table>
  </main>
</template>