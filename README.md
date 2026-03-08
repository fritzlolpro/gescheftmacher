get eve db here https://www.fuzzwork.co.uk/dump/latest/

# Geschäftmacher

Десктопное приложение (Tauri) для поиска выгодных торговых сделок в Eve Online.
Стратегия: купить в **Jita**, доставить и продать в **Goon space**.

## Стек

- **Rust** — бизнес-логика, расчёты, API, SQLite
- **Tauri 2.x** — десктоп-шелл
- **Vue 3 + Vuetify 3 + Vite** — интерфейс

## Запуск

```powershell
cd gm_tauri
npm run tauri dev    # dev (Vite + Rust вместе)
npm run tauri build  # продакшн
```

> `cargo run` из `src-tauri/` НЕ запускает Vite — WebView не откроется.

## Что умеет

| Вкладка | Описание |
|---|---|
| **Main** | Полная таблица watchlist-предметов с поиском и фильтром по марже. Группы Jita / Abroad сворачиваются. Multi-sort кликом по заголовку. |
| **VALERA_MOD** | Предфильтрованная таблица: margin ≥ 15%, profit ≥ 30M, freeze_rate ≥ 0.1, daily_vol ≥ 1. Чипы с суммарными Freeze / Vol / Profit. |
| **PASTA** | Вставить EVE-фит (или просто имена предметов) → мгновенный матч с ценами + GETPRICE для неизвестных. Группы по фиту, множитель ×N, Money Freeze. |

## Структура

```
gm_tauri/
  src/trading/
    mainTrade.vue      # Главный компонент, таб-контейнер
    valeraMode.vue     # VALERA_MOD таб
    pastaMode.vue      # PASTA таб
    fitParser.ts       # Парсер EVE-фитов
    tradeUtils.ts      # Утилиты: formatNumber, sortItems, TABLE_HEADERS, useCollapsibleHeaders
    types.ts           # Общие интерфейсы TypeScript
  src-tauri/src/
    main.rs            # Точка входа + расчёты + Tauri команды
    datagetter.rs      # API, кэш SQLite, merge данных
    watchlist.rs       # Watchlist DB + CSV-импорт
    goonmetrics.rs     # XML-десериализация Goonmetrics API
  watchlist/           # CSV-файлы предметов по группам (Excel-экспорт)
```

## Источники данных

- **Goonmetrics API**: `https://goonmetrics.apps.goonswarm.org/api/price_data/?station_id={id}&type_id={ids}` (XML, батчи по 49)
- **eve.db**: скачать на https://www.fuzzwork.co.uk/dump/latest/, положить в `src-tauri/src/eve.db`
- **gescheftmacher.db**: создаётся автоматически в `src-tauri/src/`

## Ключевые константы (main.rs)

```rust
DELIVERY_PRICE_PER_CUBOMETR = 1200.0  // ISK/м³
JITA_TAXRATE                = 0.0108
ABROAD_TAX_VALUE            = 0.056
CACHE_EXPIRY_DURATION_MINUTES = 1200  // 20 часов
JITA_ID      = "60003760"
GOON_KEEP_ID = "1049588174021"
```

## Tauri команды

- `get_data` → JSON всех `ExtendedItemData` (загружаются при старте)
- `get_prices_for_items(item_names: Vec<String>)` → `{ found: Vec<ExtendedItemData>, not_found: Vec<String> }`
