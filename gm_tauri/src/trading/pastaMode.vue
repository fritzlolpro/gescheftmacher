<script setup lang="ts">
import { ref, computed, watch } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type { ExtendedItemData } from './types'
import { formatNumber } from './tradeUtils'
import { parsePaste } from './fitParser'
import type { ParsedFitGroup } from './fitParser'

const props = defineProps<{ items: ExtendedItemData[] }>()

// ── persistence ──────────────────────────────────────────────────────────────
const pasteText = ref<string>(localStorage.getItem('pasta_paste') ?? '')
watch(pasteText, v => localStorage.setItem('pasta_paste', v))

// ── state ─────────────────────────────────────────────────────────────────────
const extraPriceData = ref<Map<string, ExtendedItemData>>(new Map())
const notFoundNames = ref<Set<string>>(new Set())
const loading = ref(false)

// group multipliers keyed by group index
const multipliers = ref<number[]>([])

// ── derived ───────────────────────────────────────────────────────────────────
const parsedPaste = computed(() => parsePaste(pasteText.value))

watch(
  () => parsedPaste.value.groups.length,
  (n) => {
    while (multipliers.value.length < n) multipliers.value.push(1)
  },
  { immediate: true }
)

/** Build a lookup map: lowercase name → ExtendedItemData from items prop */
const itemsByName = computed(() => {
  const m = new Map<string, ExtendedItemData>()
  for (const item of props.items) m.set(item.type_name.toLowerCase(), item)
  return m
})

function lookupItem(name: string): ExtendedItemData | null {
  return (
    itemsByName.value.get(name.toLowerCase()) ??
    extraPriceData.value.get(name.toLowerCase()) ??
    null
  )
}

interface ResolvedItem {
  name: string
  qty: number
  found: boolean
  priceData: ExtendedItemData | null
  // flat computed fields
  fit_cost: number
  fit_volume: number
  total_cost: number
  total_volume: number
  // pass-through for table rendering (ExtendedItemData fields or 0)
  type_id: number
  type_volume: number
  jita_buy_with_tax: number
  abroad_sell_taxed: number
  profit_jita_buy_per_unit: number
  margin_jita_buy: number
  shipping_price: number
}

interface ResolvedGroup {
  fitName: string | null
  shipName: string | null
  items: ResolvedItem[]
  multiplier: number
  totalFitCost: number
  totalFitVolume: number
  totalShipping: number
}

const resolvedGroups = computed<ResolvedGroup[]>(() => {
  return parsedPaste.value.groups.map((group: ParsedFitGroup, gi: number) => {
    const mult = multipliers.value[gi] ?? 1
    const items: ResolvedItem[] = group.items.map(fi => {
      const pd = lookupItem(fi.name)
      const jbu = pd?.jita_buy_with_tax ?? 0
      const vol = pd?.type_volume ?? 0
      const fit_cost = fi.qty * jbu
      const fit_volume = fi.qty * vol
      return {
        name: fi.name,
        qty: fi.qty,
        found: pd !== null && !notFoundNames.value.has(fi.name.toLowerCase()),
        priceData: pd,
        fit_cost,
        fit_volume,
        total_cost: fit_cost * mult,
        total_volume: fit_volume * mult,
        type_id: pd?.type_id ?? 0,
        type_volume: vol,
        jita_buy_with_tax: jbu,
        abroad_sell_taxed: pd?.abroad_sell_taxed ?? 0,
        profit_jita_buy_per_unit: pd?.profit_jita_buy_per_unit ?? 0,
        margin_jita_buy: pd?.margin_jita_buy ?? 0,
        shipping_price: pd?.shipping_price ?? 0,
      }
    })

    const totalFitCost = items.reduce((s, i) => s + i.fit_cost, 0)
    const totalFitVolume = items.reduce((s, i) => s + i.fit_volume, 0)
    const totalShipping = items.reduce((s, i) => s + i.shipping_price * i.qty, 0)

    return { fitName: group.fitName, shipName: group.shipName, items, multiplier: mult, totalFitCost, totalFitVolume, totalShipping }
  })
})

// ── GETPRICE ──────────────────────────────────────────────────────────────────
const unknownNames = computed(() => {
  const all = new Set<string>()
  for (const g of parsedPaste.value.groups) {
    for (const item of g.items) {
      const lc = item.name.toLowerCase()
      if (!itemsByName.value.has(lc) && !extraPriceData.value.has(lc)) {
        all.add(item.name)
      }
    }
  }
  return [...all]
})

async function getPrice() {
  if (!unknownNames.value.length) return
  loading.value = true
  try {
    const raw = await invoke<string>('get_prices_for_items', { itemNames: unknownNames.value })
    const result: { found: ExtendedItemData[]; not_found: string[] } = JSON.parse(raw)
    const map = new Map(extraPriceData.value)
    for (const item of result.found) map.set(item.type_name.toLowerCase(), item)
    extraPriceData.value = map
    const nf = new Set(notFoundNames.value)
    for (const name of result.not_found) nf.add(name.toLowerCase())
    notFoundNames.value = nf
  } catch (e) {
    console.error('get_prices_for_items error', e)
  } finally {
    loading.value = false
  }
}

function clearPaste() {
  pasteText.value = ''
  extraPriceData.value = new Map()
  notFoundNames.value = new Set()
  multipliers.value = []
}

// ── clipboard copy ────────────────────────────────────────────────────────────
const clickedName = ref<string | null>(null)
function copyName(name: string) {
  clickedName.value = name
  navigator.clipboard.writeText(name).catch(console.error)
  setTimeout(() => { clickedName.value = null }, 1000)
}

// ── table headers ─────────────────────────────────────────────────────────────
type HA = 'center' | 'start' | 'end'
const PASTA_HEADERS: { title: string; align: HA; value: string; sortable?: boolean }[] = [
  { title: 'Name',         align: 'start',  value: 'name',                   sortable: true },
  { title: 'Qty',          align: 'center', value: 'qty',                    sortable: true },
  { title: 'Jita Buy',     align: 'center', value: 'jita_buy_with_tax',      sortable: true },
  { title: 'Abroad Sell',  align: 'center', value: 'abroad_sell_taxed',      sortable: true },
  { title: 'Profit/unit',  align: 'center', value: 'profit_jita_buy_per_unit', sortable: true },
  { title: 'Margin %',     align: 'center', value: 'margin_jita_buy',        sortable: true },
  { title: 'Shipping/unit',align: 'center', value: 'shipping_price',         sortable: true },
  { title: 'Fit Cost',     align: 'center', value: 'fit_cost',               sortable: true },
  { title: 'Fit Vol m³',   align: 'center', value: 'fit_volume',             sortable: true },
  { title: 'Total Cost ×N',align: 'center', value: 'total_cost',             sortable: true },
  { title: 'Total Vol ×N', align: 'center', value: 'total_volume',           sortable: true },
]

function groupLabel(g: ResolvedGroup): string {
  if (g.fitName) return g.shipName ? `[${g.shipName}] ${g.fitName}` : g.fitName
  return 'General'
}
</script>

<template>
  <div class="pasta-root">
    <!-- ── Input area ── -->
    <div class="pasta-input-row">
      <v-textarea
        v-model="pasteText"
        label="Paste fit or item names here"
        rows="6"
        variant="outlined"
        hide-details
        class="pasta-textarea"
        placeholder="[Rorqual, My Fit]&#10;Drone Damage Amplifier II&#10;Hornet II x100&#10;&#10;Or just paste item names one per line"
      />
      <div class="pasta-actions">
        <v-btn
          color="primary"
          :loading="loading"
          :disabled="unknownNames.length === 0"
          @click="getPrice"
          prepend-icon="mdi-currency-usd"
        >
          GETPRICE
          <template v-if="unknownNames.length > 0">
            <v-chip size="x-small" class="ml-1">{{ unknownNames.length }}</v-chip>
          </template>
        </v-btn>
        <v-btn variant="tonal" @click="clearPaste" prepend-icon="mdi-delete-outline">Clear</v-btn>
      </div>
    </div>

    <!-- ── Groups ── -->
    <div v-for="(group, gi) in resolvedGroups" :key="gi" class="fit-group">
      <!-- Group header chips -->
      <div class="group-header">
        <v-chip size="small" color="blue-darken-2" class="fit-label">
          {{ groupLabel(group) }}
        </v-chip>
        <v-chip size="small" color="grey-lighten-2">{{ group.items.length }} items</v-chip>
        <v-chip size="small" color="orange-lighten-3">
          Fit Cost: {{ formatNumber(group.totalFitCost) }} ISK
        </v-chip>
        <v-chip size="small" color="purple-lighten-3">
          Fit Vol: {{ formatNumber(group.totalFitVolume) }} m³
        </v-chip>
        <v-chip size="small" color="teal-lighten-3">
          Shipping: {{ formatNumber(group.totalShipping) }} ISK
        </v-chip>
        <v-chip v-if="group.multiplier > 1" size="small" color="green-lighten-3">
          Total ×{{ group.multiplier }} Cost:
          {{ formatNumber(group.totalFitCost * group.multiplier) }} ISK
        </v-chip>

        <!-- Multiplier spinner -->
        <v-text-field
          v-model.number="multipliers[gi]"
          label="×N"
          type="number"
          min="1"
          density="compact"
          hide-details
          variant="outlined"
          style="max-width: 80px"
          class="ml-2"
        />
      </div>

      <!-- Group table -->
      <v-data-table
        items-per-page="-1"
        density="compact"
        :headers="PASTA_HEADERS"
        :items="group.items"
        :row-props="(item: any) => item.item.found ? {} : { class: 'row-unknown' }"
      >
        <template v-slot:item.name="{ item }">
          <span
            v-if="item.found"
            @click="copyName(item.name)"
            :class="{ clickable: true, clicked: clickedName === item.name }"
          >{{ item.name }}</span>
          <span v-else class="unknown-name">{{ item.name }} <em>(not found)</em></span>
        </template>
        <template v-slot:item.jita_buy_with_tax="{ item }">
          <span>{{ item.found ? formatNumber(item.jita_buy_with_tax) : '—' }}</span>
        </template>
        <template v-slot:item.abroad_sell_taxed="{ item }">
          <span>{{ item.found ? formatNumber(item.abroad_sell_taxed) : '—' }}</span>
        </template>
        <template v-slot:item.profit_jita_buy_per_unit="{ item }">
          <span>{{ item.found ? formatNumber(item.profit_jita_buy_per_unit) : '—' }}</span>
        </template>
        <template v-slot:item.margin_jita_buy="{ item }">
          <span>{{ item.found ? formatNumber(item.margin_jita_buy * 100) + '%' : '—' }}</span>
        </template>
        <template v-slot:item.shipping_price="{ item }">
          <span>{{ item.found ? formatNumber(item.shipping_price) : '—' }}</span>
        </template>
        <template v-slot:item.fit_cost="{ item }">
          <span>{{ item.found ? formatNumber(item.fit_cost) : '—' }}</span>
        </template>
        <template v-slot:item.fit_volume="{ item }">
          <span>{{ item.found ? formatNumber(item.fit_volume) : '—' }}</span>
        </template>
        <template v-slot:item.total_cost="{ item }">
          <span>{{ item.found ? formatNumber(item.total_cost) : '—' }}</span>
        </template>
        <template v-slot:item.total_volume="{ item }">
          <span>{{ item.found ? formatNumber(item.total_volume) : '—' }}</span>
        </template>
      </v-data-table>
    </div>

    <div v-if="parsedPaste.groups.length === 0 && pasteText.trim()" class="paste-empty">
      No items parsed — check the paste format.
    </div>
  </div>
</template>

<style scoped>
.pasta-root {
  padding: 8px 0;
}
.pasta-input-row {
  display: flex;
  gap: 12px;
  align-items: flex-start;
  margin-bottom: 16px;
}
.pasta-textarea {
  flex: 1;
}
.pasta-actions {
  display: flex;
  flex-direction: column;
  gap: 8px;
  padding-top: 4px;
}
.fit-group {
  margin-bottom: 20px;
  border: 1px solid #e0e0e0;
  border-radius: 4px;
  overflow: hidden;
}
.group-header {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
  align-items: center;
  padding: 8px;
  background: #fafafa;
  border-bottom: 1px solid #e0e0e0;
}
.fit-label {
  font-weight: 600;
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
.unknown-name {
  color: #999;
}
.paste-empty {
  color: #888;
  padding: 16px;
  text-align: center;
}
</style>
