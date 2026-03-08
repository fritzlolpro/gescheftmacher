# Geschäftmacher — Copilot Agent Instructions

## Что это за проект

**Geschäftmacher** (нем. "делатель денег") — десктопное приложение для поиска выгодных торговых сделок в игре **Eve Online**.

Стратегия: купить товар в торговом хабе **Jita**, доставить и продать в торговом хабе **Goon** (Гунсварм). Profit считается с учётом налогов, стоимости доставки и объёма торгов.

## Технологический стек

- **Backend**: Rust — бизнес-логика, расчёты, БД, HTTP
- **Desktop shell**: Tauri 2.x
- **Frontend**: Vue 3 + TypeScript + Vite + Vuetify 3
- **БД**: SQLite через `rusqlite`
  - `src-tauri/src/eve.db` — игровая БД (предметы, объёмы, id). Read-only.
  - `src-tauri/src/gescheftmacher.db` — рабочая БД (watchlist, кэш цен)

## Структура проекта

```
gm_tauri/
  src/
    App.vue
    trading/
      mainTrade.vue      # Корневой компонент: табы, загрузка данных
      valeraMode.vue     # Таб VALERA_MOD — предфильтрованная таблица
      pastaMode.vue      # Таб PASTA — анализ вставленных фитов/предметов
      fitParser.ts       # Парсер EVE-фитов из текста
      types.ts           # Общие TypeScript интерфейсы
      tradeUtils.ts      # formatNumber, formatDate, sortItems, TABLE_HEADERS, useCollapsibleHeaders
  src-tauri/src/
    main.rs              # Точка входа + impl ItemData/ExtendedItemData + Tauri команды
    datagetter.rs        # API, кэш SQLite, merge данных
    goonmetrics.rs       # XML-десериализация ответов Goonmetrics
    watchlist.rs         # Watchlist: таблицы DB, CSV-импорт
    static_data.rs       # УСТАРЕЛО, не используется
  watchlist/             # CSV-файлы с предметами по группам
```

## Как запускать

```powershell
cd gm_tauri
npm run tauri dev     # ПРАВИЛЬНО: запускает Vite + Rust вместе
```

`cargo run` из `src-tauri/` — **НЕ РАБОТАЕТ** (нет Vite, WebView не откроет localhost:1420).

```powershell
npm run tauri build   # Продакшн сборка
```

## Константы (main.rs)

```rust
DELIVERY_PRICE_PER_CUBOMETR = 1200.0  // ISK/м³
JITA_TAXRATE              = 0.0108     // 1.08%
ABROAD_TAX_VALUE          = 0.056     // 5.6%
CACHE_EXPIRY_DURATION_MINUTES = 1200  // 20 часов
JITA_ID     = "60003760"
GOON_KEEP_ID = "1049588174021"        // текущий ID хаба Гунов
```

## Источник данных

**Goonmetrics API**: `https://goonmetrics.apps.goonswarm.org/api/price_data/?station_id={id}&type_id={ids}`

Возвращает XML → парсится через `serde-xml-rs`. Батчи по 49 id (`MAX_GOONMETRICS_ID_QUANTITY = 49`).

## Структура данных

### ItemData (промежуточная)
```rust
pub struct ItemData {
    type_id, type_volume, type_name,
    jita_trade_data: Option<TradeData>,
    abroad_trade_data: Option<TradeData>,
}
```

### ExtendedItemData (финальная, хранится в кэше и отдаётся фронту)
```rust
pub struct ExtendedItemData {
    type_id, type_volume, timestamp, type_name,
    jita_trade_data: TradeData,      // buy_max, sell_min, weekly_movement, buy_listed, sell_listed, updated
    jita_buy_with_tax,               // buy_max * (1 + 0.0108)
    abroad_trade_data: TradeData,
    abroad_stocked_ratio,            // sell_listed / weekly_movement
    shipping_price,                  // type_volume * 1200
    abroad_sell_taxed,               // sell_min * (1 - 0.056)
    abroad_avg_daily,                // weekly_movement / 7 / sqrt(stocked_ratio)
    profit_jita_buy_per_unit,        // abroad_sell_taxed - jita_buy_with_tax - shipping_price
    profit_jita_buy_daily,           // abroad_avg_daily * profit_per_unit
    margin_jita_buy,                 // profit_per_unit / (jita_buy_with_tax + shipping_price)
    money_freeze_buy,                // abroad_avg_daily * jita_buy_with_tax
    freeze_rate,                     // profit_daily / money_freeze_buy
}
```

**Важно**: `money_freeze_buy` в `ExtendedItemData` = `abroad_avg_daily × jita_buy_with_tax` (для дневной торговли).  
В PASTA-режиме freeze считается иначе: `qty × jita_buy_with_tax` (разовая закупка на фит).

## Поток данных при старте

```
main() →
  ensure_watchlist_initialized()        // создаёт таблицы, импортирует CSV если пусто
  get_watchlist_as_item_data()          // Vec<ItemData> из watchlist_items (active=1)
  fetch_and_compute_items(items_data)   // кэш → API → merge → ExtendedItemData
  run(data)                             // Tauri state: AppData { data }

Frontend: invoke("get_data") → JSON → Vue reactive data
```

## Tauri команды

| Команда | Аргументы | Возврат | Описание |
|---|---|---|---|
| `get_data` | — | JSON string | Все `ExtendedItemData` из AppData (загружены при старте) |
| `get_prices_for_items` | `item_names: Vec<String>` | JSON string | Резолвит имена → eve.db → API/кэш → `{ found: Vec<ExtendedItemData>, not_found: Vec<String> }` |
| `greet` | `name: String` | String | Тестовая |

## Frontend: вкладки

### Main (`mainTrade.vue`)
- Поиск по имени, слайдер минимальной маржи
- Полная таблица всех watchlist-предметов
- Группы колонок Jita / Abroad сворачиваются через чипы (`useCollapsibleHeaders`)
- Multi-sort: клик по заголовку добавляет в сортировку (без Shift)

### VALERA_MOD (`valeraMode.vue`)
- Фильтр: `margin_jita_buy ≥ 15%` AND `profit_daily ≥ 30M` AND `freeze_rate ≥ 0.1` AND `abroad_avg_daily ≥ 1`
- Сортировка по `abroad_avg_daily DESC` по умолчанию
- Чипы: настройки фильтра, кол-во items, общий Freeze, Daily Vol (м³), суммарный Profit/day

### PASTA (`pastaMode.vue`)
- Поле для вставки EVE-фитов (формат ниже) или произвольных названий предметов
- Поле сохраняется в `localStorage`
- Мгновенный матч против уже-загруженных данных (items prop из Main)
- Кнопка **GETPRICE** — отправляет незнакомые имена в `get_prices_for_items`, дополняет данные
- На каждую группу: chips (fitName, items, Fit Cost, Fit Vol, Shipping, Freeze) + spinner ×N
- При ×N > 1 пересчитываются колонки Total Cost, Total Vol
- Неизвестные предметы (not in eve.db) — серая строка *(not found)*

## Парсер фитов (fitParser.ts)

EVE-фит имеет формат:
```
[ShipName, FitName]           ← новая группа; ShipName = первый предмет
ModuleName                    ← qty = 1
ModuleName, ChargeLoaded      ← qty = 1, заряд отбрасывается
DroneName x100                ← qty = 100
```

Несколько фитов в одном поле — корректно парсятся в отдельные группы.  
Предметы до первого заголовка → группа `fitName=null` (General).

## Watchlist

CSV-файлы в `gm_tauri/watchlist/` (экспорт из Excel, читается первые 3 колонки):
```csv
ItemName,ItemId,vol.
Astero,33468,"2,500"
```
Если `ItemId=0` или `vol=0` → резолвится из `eve.db` по имени.  
Парсер в `watchlist.rs` обрабатывает quoted-числа с запятыми.

## Сортировка в таблицах

Все таблицы используют ручную сортировку (не `v-model:sort-by` Vuetify — оно ненадёжно с computed items):  
`sortItems(items, sortByRef)` из `tradeUtils.ts` + `applyColumnClick` для накопления multi-sort без Shift.  
Обработчик: `:sort-by="sortBy" @update:sort-by="onSortUpdate"`.

## База данных gescheftmacher.db

### `extended_item_data`
Кэш цен. Все поля `ExtendedItemData` + `timestamp` (Unix). Свежий = < 1200 мин (20 ч).  
Данные хранятся без автоочистки.

### `watchlist_groups` / `watchlist_items`
```sql
watchlist_items: id, type_id, type_name, type_volume, group_id, active
```
Импортируется из CSV при первом запуске. Сброс: удалить `gescheftmacher.db` (пересоздаётся автоматически).

## Важные нюансы

- БД пути **относительные от CWD** (`src/eve.db`, `src/gescheftmacher.db`). CWD при `npm run tauri dev` = `src-tauri/`. В продакшне нужен `app_data_dir()`.
- `merge_trade_data` делает `panic!` если предмет не найден в API-ответе — это может сломать запуск при добавлении новых предметов без данных.
- `static_data.rs` (TradePile) — устарело, не используется, можно удалить.
- `eve.db` нужно скачать отдельно: https://www.fuzzwork.co.uk/dump/latest/
- При изменении `GOON_KEEP_ID` нужно удалить `gescheftmacher.db` для сброса кэша.


## Технологический стек

- **Backend**: Rust (основная бизнес-логика, расчёты, работа с БД, HTTP запросы)
- **Desktop shell**: Tauri 2.x (мост между Rust и WebView)
- **Frontend**: Vue 3 + TypeScript + Vite + Vuetify (таблица с фильтрами)
- **БД**: SQLite через `rusqlite`
  - `eve.db` — игровая база данных (предметы, объёмы, id). Read-only. Лежит в `src-tauri/src/eve.db`
  - `gescheftmacher.db` — рабочая БД приложения (watchlist, кэш цен, история). Лежит в `src-tauri/src/gescheftmacher.db`

## Структура проекта

```
gm_tauri/
  src/                        # Vue frontend
    trading/mainTrade.vue     # Главная таблица с данными и фильтрами
    App.vue                   # Корень Vue приложения
  src-tauri/
    src/
      main.rs                 # Точка входа: tokio::main → fetch data → start Tauri
      datagetter.rs           # Логика API, SQLite (кэш цен), merge данных
      goonmetrics.rs          # Парсинг XML ответов Goonmetrics API
      watchlist.rs            # Watchlist: таблицы DB, CSV импорт, чтение предметов
      static_data.rs          # УСТАРЕЛО — старый хардкод списка предметов (не используется)
    Cargo.toml
    tauri.conf.json
  watchlist/                  # CSV файлы с предметами по группам
    Ammo.csv
    Drones.csv
    ImplantsBoosters.csv
    ManufactureResearch.csv
    ShipEquipment.csv
    Ships.csv
```

## Как запускать

**ПРАВИЛЬНЫЙ способ** (запускает и Vite dev server, и Rust):
```powershell
cd gm_tauri
npm run tauri dev
```

**НЕ ПРАВИЛЬНО** — `cargo run` из `src-tauri/` НЕ запускает Vite. WebView не откроет localhost:1420.

Для сборки продакшн:
```powershell
cd gm_tauri
npm run tauri build
```

## Константы и бизнес-логика (в main.rs)

```rust
DELIVERY_PRICE_PER_CUBOMETR = 850.0   // ISK за м³ за доставку
JITA_TAXRATE = 0.0108                  // налог на покупку в Jita (~1.08%)
ABROAD_TAX_VALUE = 0.056               // налог на продажу у Гунов (~5.6%)
CACHE_EXPIRY_DURATION_MINUTES = 120    // кэш цен: 2 часа
JITA_ID = "60003760"                   // ID станции Jita 4-4
GOON_KEEP_ID = "1046664001931"         // ID торгового хаба Гунов
```

## Источник данных

**Goonmetrics API** — `https://goonmetrics.apps.goonswarm.org/api/price_data/?station_id={id}&type_id={ids}`

Возвращает XML. Парсится через `serde-xml-rs` в структуры `Goonmetrics` → `PriceData` → `ItemType`.

Запросы батчатся по 49 предметов за раз (`MAX_GOONMETRICS_ID_QUANTITY = 49`).

## Структура данных ExtendedItemData

Финальная строка таблицы. Содержит все рассчитанные поля:
- `type_id`, `type_name`, `type_volume` — базовая инфо
- `jita_trade_data` / `abroad_trade_data` — сырые данные из API (TradeData: buy_max, sell_min, weekly_movement, listed)
- `jita_buy_with_tax` — цена покупки в Jita с налогом
- `abroad_sell_taxed` — цена продажи у Гунов после налога
- `shipping_price` — стоимость доставки = volume × 850
- `abroad_avg_daily` — средний дневной объём продаж у Гунов (скорректированный на stocked ratio)
- `profit_jita_buy_per_unit` — прибыль с единицы
- `profit_jita_buy_daily` — дневная прибыль
- `margin_jita_buy` — маржа (для фильтра в UI)
- `money_freeze_buy` — деньги "заморожены" в закупке
- `freeze_rate` — отношение дневной прибыли к заморозке

## База данных gescheftmacher.db

### Таблица `extended_item_data` (кэш цен + история)
Все поля `ExtendedItemData` + `timestamp` (Unix). Кэш считается свежим 2 часа.
Данные хранятся бесконечно (пока не договоримся об очистке).

### Таблица `watchlist_groups` (TODO — не реализовано)
Группы предметов: ammo, modules, ships, implants и т.д.

### Таблица `watchlist_items` (TODO — не реализовано)
Персональный список предметов для отслеживания.
Поля: `id`, `type_id`, `type_name`, `type_volume`, `group_id`, `active`.
Источник: импорт из CSV файлов.

## Watchlist CSV формат

CSV файлы лежат в `gm_tauri/watchlist/` по группам (экспорт из Excel).
Формат: первые три колонки — `ItemName, ItemId, vol.` Остальные колонки игнорируются.
Числа могут быть в quoted-формате с запятыми (`"2,500"`) — парсер в `watchlist.rs` это обрабатывает.

```csv
ItemName,ItemId,vol.,... (остальные колонки игнорируются)
Astero,33468,"2,500",...
Focused Void Bomb,34264,75.0000,...
```

Если `ItemId=0` или `vol=0` → данные резолвятся из `eve.db` по имени.

## Поток данных при старте приложения

```
main() (tokio::main)
  ├── ensure_watchlist_initialized() (watchlist.rs)
  │      ├── создаёт таблицы watchlist_groups / watchlist_items если нет
  │      └── если watchlist пуст → импортирует все CSV из gm_tauri/watchlist/
  ├── get_watchlist_as_item_data() → Vec<ItemData> из watchlist_items (active=1)
  ├── get_stored_items_history() → проверяем кэш в gescheftmacher.db
  ├── если кэш свежий (<2ч) → берём из кэша
  └── если кэш устарел → get_item_data_from_api() → сохраняем в кэш
          ↓
      run(data) → Tauri::Builder → AppData state
          ↓
      Frontend: invoke("get_data") → получает Vec<ExtendedItemData> как JSON
          ↓
      v-data-table с фильтрами (margin slider, search)
```

## Tauri команды (Rust → Vue)

- `get_data` → возвращает JSON строку всех `ExtendedItemData`
- `greet` → тестовая команда

## Текущие TODO в коде

1. `static_data.rs` (TradePile) → не используется, можно удалить
2. Фильтры в UI → уже частично сделан slider по margin, нужно больше
3. Пути к БД файлам сделать абсолютными (сейчас относительные — `src/gescheftmacher.db`)
4. Обработка ошибок в `merge_trade_data` — сейчас `panic!`

## Важные нюансы

- Пути к БД (`src/gescheftmacher.db`, `src/eve.db`) — **относительные от CWD**. При запуске через `npm run tauri dev` CWD = `src-tauri/`. Это работает. При запуске собранного бинаря путь может отличаться — надо будет перейти на `app_data_dir()` Tauri.
- Батчинг запросов к Goonmetrics: максимум 49 id за запрос, реализован `split_large_id_bulks()`.
- `get_abroad_avg_daily()` делит на `sqrt(stocked_ratio)` — это эвристика чтобы скорректировать объём на текущий сток.
- Click по `type_name` в таблице копирует имя в буфер обмена (для быстрого поиска в игре).
