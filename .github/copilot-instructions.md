# Geschäftmacher — Copilot Agent Instructions

## Что это за проект

**Geschäftmacher** (нем. "делатель денег") — десктопное приложение для поиска выгодных торговых сделок в игре **Eve Online**.

Стратегия: купить товар в торговом хабе **Jita** (самый крупный рынок), доставить и продать в торговом хабе **Goon** (Гунсварм). Profit рассчитывается с учётом налогов, стоимости доставки и объёма торгов.

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
