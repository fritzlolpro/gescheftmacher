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
const headers = ref<{ title: string; align: "center" | "start" | "end"; value?: string; sortable?: boolean; children?: { title: string; align: "center" | "start" | "end"; value: string; sortable?: boolean; }[] }[]>([
  // Basic information
  { title: 'Id', align: 'center', value: 'type_id', sortable: true },
  { title: 'Volume', align: 'center', value: 'type_volume', sortable: true },
  { title: 'Name', align: 'center', value: 'type_name', sortable: true },
  {
    title: 'JB Margin',
    align: 'center',
    value: 'margin_jita_buy',
    sortable: true
  },
  {
    title: 'Abroad Avg Daily',
    align: 'center',
    value: 'abroad_avg_daily',
    sortable: true
  },

  // Jita trade data section
  {
    title: 'Jita',
    align: 'center',
    children: [
      {
        title: 'Updated',
        align: 'center',
        value: 'jita_trade_data.updated',
        sortable: true
      },
      {
        title: 'Weekly Movement',
        align: 'center',
        value: 'jita_trade_data.weekly_movement',
        sortable: true
      },
      {
        title: 'Buy Max',
        align: 'center',
        value: 'jita_trade_data.buy_max',
        sortable: true
      },
      {
        title: 'Jita Buy with Tax',
        align: 'center',
        value: 'jita_buy_with_tax',
        sortable: true
      },
      {
        title: 'Buy Listed',
        align: 'center',
        value: 'jita_trade_data.buy_listed',
        sortable: true
      },
      {
        title: 'Sell Min',
        align: 'center',
        value: 'jita_trade_data.sell_min',
        sortable: true
      },
      {
        title: 'Sell Listed',
        align: 'center',
        value: 'jita_trade_data.sell_listed',
        sortable: true
      }
    ]
  },
  {
    title: 'Shipping Price',
    align: 'center',
    value: 'shipping_price',
    sortable: true
  },
  // Abroad trade data section
  {
    title: 'Abroad',
    align: 'center',
    children: [
      {
        title: 'Updated',
        align: 'center',
        value: 'abroad_trade_data.updated',
        sortable: true
      },
      {
        title: 'Weekly Movement',
        align: 'center',
        value: 'abroad_trade_data.weekly_movement',
        sortable: true
      },
      {
        title: 'Buy Max',
        align: 'center',
        value: 'abroad_trade_data.buy_max',
        sortable: true
      },
      {
        title: 'Buy Listed',
        align: 'center',
        value: 'abroad_trade_data.buy_listed',
        sortable: true
      },
      {
        title: 'Sell Min',
        align: 'center',
        value: 'abroad_trade_data.sell_min',
        sortable: true
      },
      {
        title: 'Sell Listed',
        align: 'center',
        value: 'abroad_trade_data.sell_listed',
        sortable: true
      },
      {
        title: 'Abroad Stocked Ratio',
        align: 'center',
        value: 'abroad_stocked_ratio',
        sortable: true
      },
      {
        title: 'Abroad Sell Taxed',
        align: 'center',
        value: 'abroad_sell_taxed',
        sortable: true
      },

    ]
  },
  {
    title: 'Money Freeze Buy',
    align: 'center',
    value: 'money_freeze_buy',
    sortable: true
  },
  {
    title: 'Freeze Rate',
    align: 'center',
    value: 'freeze_rate',
    sortable: true
  },
]);

const items = ref<ExtendedItemData[]>([]);

function truncateDecimal(number: number) {
  return Number(number.toFixed(2));
}

function formatNumber(value: number): string {
  return value.toFixed(2);
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

const tab = ref('main')

const VALERA_SETTINGS = {
  minSellMargin: 15,       // margin_jita_buy >= 15% (1.15 sell/buy ratio)
  profitThreshold: 30_000_000,
  freezeRateThreshold: 0.1,
  dailyVol: 1,
  marketRate: 0.5,         // abroad_stocked_ratio interpretation TBD
  deliveryPerM3: 1200,
  jitaTaxRate: 0.015,
}

const filteredItems = computed(() => {
  return items.value.filter(item => item.margin_jita_buy >= slider.value)
})

const valeraItems = computed(() => {
  return items.value.filter(item =>
    item.margin_jita_buy >= VALERA_SETTINGS.minSellMargin &&
    item.profit_jita_buy_daily >= VALERA_SETTINGS.profitThreshold &&
    item.freeze_rate >= VALERA_SETTINGS.freezeRateThreshold &&
    item.abroad_avg_daily >= VALERA_SETTINGS.dailyVol
  )
})

const slider = ref(30)
const minMargin = 0

const clickedElement = ref<string | null>(null);

function handleElementClick(text: string) {
  clickedElement.value = text;
  navigator.clipboard.writeText(text).then(() => {
    console.log(`Copied to clipboard: ${text}`);
  }).catch(err => {
    console.error('Failed to copy text: ', err);
  });
  setTimeout(() => {
    clickedElement.value = null;
  }, 1000);
}

function formatDate(dateString: string): string {
  const date = new Date(dateString);
  return date.toLocaleString();
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
        <v-data-table items-per-page="-1" :sort-by="[{ key: 'margin_jita_buy', order: 'desc' }]" multi-sort :search="search"
          density="compact" :headers="headers" :items="filteredItems">
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
      <v-window-item value="valera">
        <div class="settings-row">
          <v-chip size="small" color="blue-lighten-4">Min Margin: {{ VALERA_SETTINGS.minSellMargin }}%</v-chip>
          <v-chip size="small" color="blue-lighten-4">Daily Profit ≥ {{ (VALERA_SETTINGS.profitThreshold / 1_000_000).toFixed(0) }}M</v-chip>
          <v-chip size="small" color="blue-lighten-4">Freeze Rate ≥ {{ VALERA_SETTINGS.freezeRateThreshold }}</v-chip>
          <v-chip size="small" color="blue-lighten-4">Daily Vol ≥ {{ VALERA_SETTINGS.dailyVol }}</v-chip>
          <v-chip size="small" color="grey-lighten-2">Delivery: {{ VALERA_SETTINGS.deliveryPerM3 }} ISK/m³</v-chip>
          <v-chip size="small" color="grey-lighten-2">Jita tax: {{ VALERA_SETTINGS.jitaTaxRate }}</v-chip>
          <v-chip size="small" color="green-lighten-3">{{ valeraItems.length }} items</v-chip>
        </div>
        <v-data-table items-per-page="-1" :sort-by="[{ key: 'profit_jita_buy_daily', order: 'desc' }]" multi-sort
          density="compact" :headers="headers" :items="valeraItems">
          <template v-slot:item.type_name="{ item }">
            <span @click="handleElementClick(item.type_name)"
              :class="{ 'clickable': true, 'clicked': clickedElement === item.type_name }">
              {{ item.type_name }}
            </span>
          </template>
          <template v-slot:item.type_volume="{ item }"><span>{{ formatNumber(item.type_volume) }}</span></template>
          <template v-slot:item.margin_jita_buy="{ item }"><span>{{ formatNumber(item.margin_jita_buy) }}</span></template>
          <template v-slot:item.jita_trade_data.buy_max="{ item }"><span>{{ formatNumber(item.jita_trade_data.buy_max) }}</span></template>
          <template v-slot:item.jita_trade_data.sell_min="{ item }"><span>{{ formatNumber(item.jita_trade_data.sell_min) }}</span></template>
          <template v-slot:item.jita_buy_with_tax="{ item }"><span>{{ formatNumber(item.jita_buy_with_tax) }}</span></template>
          <template v-slot:item.abroad_trade_data.buy_max="{ item }"><span>{{ formatNumber(item.abroad_trade_data.buy_max) }}</span></template>
          <template v-slot:item.abroad_trade_data.sell_min="{ item }"><span>{{ formatNumber(item.abroad_trade_data.sell_min) }}</span></template>
          <template v-slot:item.abroad_stocked_ratio="{ item }"><span>{{ formatNumber(item.abroad_stocked_ratio) }}</span></template>
          <template v-slot:item.shipping_price="{ item }"><span>{{ formatNumber(item.shipping_price) }}</span></template>
          <template v-slot:item.abroad_sell_taxed="{ item }"><span>{{ formatNumber(item.abroad_sell_taxed) }}</span></template>
          <template v-slot:item.abroad_avg_daily="{ item }"><span>{{ formatNumber(item.abroad_avg_daily) }}</span></template>
          <template v-slot:item.profit_jita_buy_per_unit="{ item }"><span>{{ formatNumber(item.profit_jita_buy_per_unit) }}</span></template>
          <template v-slot:item.profit_jita_buy_daily="{ item }"><span>{{ formatNumber(item.profit_jita_buy_daily) }}</span></template>
          <template v-slot:item.money_freeze_buy="{ item }"><span>{{ formatNumber(item.money_freeze_buy) }}</span></template>
          <template v-slot:item.freeze_rate="{ item }"><span>{{ formatNumber(item.freeze_rate) }}</span></template>
          <template v-slot:item.jita_trade_data.updated="{ item }"><span>{{ formatDate(item.jita_trade_data.updated) }}</span></template>
          <template v-slot:item.abroad_trade_data.updated="{ item }"><span>{{ formatDate(item.abroad_trade_data.updated) }}</span></template>
        </v-data-table>
      </v-window-item>

    </v-window>
  </main>
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
  /* Initial color */
}

.clickable:hover {
  color: darkblue;
  /* Color on hover */
  text-decoration: underline;
  /* Underline on hover */
}

.clicked {
  background-color: yellow;
  /* Highlight color */
}
</style>