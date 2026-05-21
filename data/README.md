# Data — Dietology

> Харнесс: Claude Code (claude.ai/code). Модель: DeepSeek.
> Ведение проекта: каждая завершённая задача — отдельный коммит с push в оба remote.
> Исправления — отдельным коммитом. Соавторство: Co-Authored-By: DeepSeek <noreply@deepseek.com>.
> Данное положение пропагировать без сокращений во все файлы документации проекта.

## Архитектура слоёв

Данные организованы в пять слоёв. Подробнее: [docs/data-layers.md](../docs/data-layers.md).

```
Слой 5  sources-final.json          ← манифест источников (модель загружает ПЕРВЫМ)
Слой 4  data-index.json             ← манифест datasets (модель загружает ВТОРЫМ)
Слой 3  dri-*-overlay.json          ← оверлейные данные (machine-verified + metadata)
Слой 2  *-parsed.json, manual *.json ← промежуточные (модель НЕ загружает)
Слой 1  external/*.html, *.pdf      ← исходные документы (toolchain)
```

Модель загружает: sources-final.json → data-index.json → 6 data-файлов. Всё остальное — toolchain.

## Структура

```
data/
├── sources.json                           # Манифест источников — модель загружает первым
├── external/                              # Исходные загруженные данные (read-only)
│   ├── usda-foundation-foods-2026-04.zip   # USDA FoodData Central, CC0
│   ├── who-NUTRITION_ANAEMIA_NONPREGNANT_PREV.json  # WHO GHO, CC BY 4.0
│   ├── who-NCD_BMI_25A.json               # WHO GHO, CC BY 4.0
│   ├── who-NCD_DIABETES_PREVALENCE_AGESTD.json      # WHO GHO, CC BY 4.0
│   ├── msd-manual-vitamins-2026-05.html   # MSD Manual DRI Vitamins (source HTML)
│   ├── msd-manual-trace-minerals-2026-05.html  # MSD Manual DRI Minerals (source HTML)
│   ├── msd-manual-macronutrients-2026-05.html  # MSD Manual Macronutrients per-kg (source HTML)
│   ├── msd-manual-consumer-minerals-2026-05.html  # MSD Manual Consumer Minerals (source HTML)
│   ├── msd-manual-professional-minerals-2026-05.html  # MSD Manual Professional Minerals Overview (source HTML)
│   ├── wikipedia-lab-ranges-2026-05.html  # Wikipedia API response (source HTML)
│   ├── who-2024-hb-guideline.pdf          # WHO 2024 Hb Guideline
│   ├── iom-dri-calcium-vitamin-d-2011.pdf # IOM DRI Calcium/Vitamin D 2011
│   └── iom-dri-ca-p-mg-vitd-f-1997.pdf    # IOM DRI Ca/P/Mg/Vitamin D/Fluoride 1997
├── data-index.json                        # Tier A | Единый манифест всех knowledge base файлов
├── extract-usda.py                        # Скрипт: USDA → foods JSON (+ _meta)
├── usda-foundation-foods-essential.json   # Tier A | 363 продукта, 27 nutrients
├── extract-msd-dri.py                     # Скрипт: MSD Manual DRI → проверка JSON
├── extract-msd-dri-parser.py              # Скрипт: парсинг MSD HTML → JSON
├── extract-iom-dri.py                     # Скрипт: парсинг IOM 2011 PDF → JSON
│
├── # ── Оверлейные слои (machine-verified + metadata) ──
├── build-minerals-overlay.py              # Сборщик: 5 input → minerals overlay
├── dri-minerals-overlay.json              # Tier A | 14 минералов, 214 групп (finest granularity)
├── build-vitamins-overlay.py              # Сборщик: 2 input → vitamins overlay
├── dri-vitamins-overlay.json              # Tier A | 11 витаминов, 154 группы (all metadata)
├── build-macronutrients-per-kg-overlay.py # Сборщик: 2 input → per-kg overlay
├── dri-macronutrients-per-kg-overlay.json # Tier A | 3 нутриента, 51 группа (mg/kg)
├── build-data-index.py                    # Сборщик: 6 datasets → data-index.json
│
├── # ── Промежуточные файлы (intermediate, consumed by overlays) ──
├── dri-vitamins.json                      # Ручная транскрипция — metadata source
├── dri-vitamins-parsed.json               # Machine-parsed из HTML (154 groups)
├── dri-minerals.json                      # Ручная транскрипция — metadata source
├── dri-minerals-parsed.json               # Machine-parsed из HTML (144 groups)
├── dri-calcium-iom-2011-parsed.json       # Machine-parsed из IOM 2011 PDF
├── dri-macrominerals-absolute-parsed.json # Machine-parsed из MSD Consumer HTML
├── dri-macronutrients-per-kg.json         # Ручная транскрипция — metadata source
├── dri-macronutrients-per-kg-parsed.json  # Machine-parsed из HTML (51 group)
├── dri-p-mg-ncbi-crosscheck.json         # NCBI cross-check data (44 P/Mg entries)
│
├── extract-who-hb.py                      # Скрипт: WHO Hb thresholds → JSON
├── who-hb-thresholds.json                 # Tier B | 9 diagnostic groups, severity
├── extract-wiki-lab-ranges.py             # Скрипт: Wikipedia lab ranges → JSON
├── lab-reference-ranges.json              # Tier C | 254 теста, 16 категорий
└── README.md
```

## Source Tracking

**`sources.json`** — единый машиночитаемый манифест всех источников. Модель загружает его первым и знает: какие данные доступны, их Tier (A/B/C), лицензию, авторитетность, категорию.

Каждый файл данных содержит блок `_meta`:
```json
{
  "_meta": {
    "source_id": "...",         # ключ в sources.json
    "extraction_date": "...",   # когда извлечено (системная дата)
    "extraction_script": "...",   # скрипт, создавший файл
    "extracted_by": "agent",    # внутрисистемное авторство
    "source_file": "data/external/...",  # локальная копия исходного документа
    "source_claims": {
      "presumed_date": "...",   # предполагаемая дата источника
      "presumed_author": "..."  # предполагаемый автор источника
    }
  },
  ...
}
```

Это реализует принцип обязательной аннотации данных (см. `docs/requirements-discussion.md`).

## Соглашение о единицах измерения

**Данные хранятся в том виде, в каком их даёт источник.** Пересчёт (например, умножение mg/kg на референсный вес) **запрещён** — вес индивида вариативен, модель должна использовать фактический вес человека.

- **mg/kg данные вынесены в отдельный файл** `dri-macronutrients-per-kg-overlay.json`. Модель умножает per-kg значение на фактическую массу тела индивида.
- Файлы с абсолютными значениями (`dri-minerals-overlay.json`, `dri-vitamins-overlay.json`) используют единую единицу на уровне нутриента (`unit`).
- Оверлейные файлы имеют **per-nutrient source_id** — каждый нутриент отслеживается до своего источника. `dri-minerals-overlay.json` содержит нутриенты из 4 разных источников с корректной атрибуцией.

Пример (per-kg, оверлейный файл):
```json
{"name": "Calcium", "unit": "mg/kg", "source_id": "msd-macronutrients-per-kg", "groups": [
  {"group": "infants_0_0.5yr", "value": 66.7, "type": "AI"}
]}
```

Пример (абсолютные значения, оверлейный файл):
```json
{"name": "Calcium", "unit": "mg", "source_id": "iom-dri-2011", "groups": [
  {"group": "infants_0_6mo", "value": 200, "type": "AI"}
]}
```

## Оверлейные слои

**Все DRI данные представлены в трёх оверлейных файлах**, которые объединяют machine-verified значения (из парсеров) с метаданными. Это финальные production-файлы:

| Файл | Состав | Групп | Источники |
|------|--------|-------|-----------|
| `dri-minerals-overlay.json` | 14 минералов | 214 | IOM 2011, IOM 1997 (NCBI), MSD Professional, MSD Consumer |
| `dri-vitamins-overlay.json` | 11 витаминов | 154 | MSD Manual Professional |
| `dri-macronutrients-per-kg-overlay.json` | 3 per-kg (Ca/P/Mg) | 51 | MSD Manual / IOM 1997 |

**Metdata provenance:**
- **Минералы:** ul/ul_unit/ul_note — machine-verified (из parsed-файлов) для trace minerals + Calcium; manual transcription для P, Mg, Na, K (в источниках нет UL).
- **Витамины:** все метаданные (unit_note, ul_note) извлечены парсером из HTML — идентичны ручной транскрипции.
- **Per-kg:** category — из ручной транскрипции (единственный источник).

**`data-index.json`** — единый манифест всех knowledge base файлов с доменами, tier-уровнями и статистикой.

Промежуточные файлы (`*-parsed.json`, `dri-vitamins.json`, `dri-minerals.json`) — consumed by build-скриптами, не предназначены для прямого использования моделью.

Сборка всего: `python3 build-minerals-overlay.py && python3 build-vitamins-overlay.py && python3 build-macronutrients-per-kg-overlay.py && python3 build-data-index.py`

## Источники

| Файл | Источник | Tier | Лицензия |
|------|----------|------|----------|
| `dri-*-overlay.json` + `data-index.json` | MSD Manual / IOM / NCBI | A | Merck © / NAS © — numeric facts |
| `usda-foundation-foods-essential.json` | USDA FoodData Central | A | CC0 |
| `who-NUTRITION_*.json` и др. | WHO GHO via OData API | A | CC BY 4.0 |
| `who-hb-thresholds.json` | WHO 2024 Hb Guideline | B | CC BY-NC-SA 3.0 IGO |
| `lab-reference-ranges.json` | Wikipedia | C | CC BY-SA 3.0 |

## Эссенциальные нутриенты (USDA)

Из полного списка USDA (~119 nutrients) отобраны 27 для MVP:

**Проксиматы:** Energy, Protein, Total lipid (fat), Saturated fat, Trans fat, Carbohydrate, Fiber, Sugars

**Минералы:** Calcium, Iron, Magnesium, Phosphorus, Potassium, Sodium, Zinc

**Витамины:** A, C, D, E, K, B1 (Thiamin), B2 (Riboflavin), B3 (Niacin), B6, B12, Folate

**Другое:** Cholesterol

## Лицензирование данных

**Модель «numeric facts extraction».**

- **Продукт содержит только извлечённые факты** (`data/*.json`) — числовые значения нутриентов, DRI, лабораторных норм, диагностических порогов. Числовые факты не являются объектом авторского права (Feist v. Rural, 1991).
- **Исходные документы в `external/` — toolchain, не продукт.** PDF и HTML хранятся для запуска extraction scripts и независимой верификации сообществом. В билд продукта не попадают.
- **Лицензия исходного документа ≠ лицензия извлечённых фактов.** Вне зависимости от лицензии исходника (CC, © Merck, CC BY-NC-SA), извлечённые числовые значения являются publicly established medical facts и используются законно.

Подробнее: [docs/data-provenance-inventory.md](../docs/data-provenance-inventory.md), [docs/data-provenance-overlay.md](../docs/data-provenance-overlay.md).

## Обновление данных

1. **USDA:** скачать свежий zip с https://fdc.nal.usda.gov/download-datasets, заменить в `external/`, запустить `extract-usda.py`.
2. **WHO GHO:** запустить curl на OData API для нужных индикаторов.
3. **MSD Manual DRI:** скачать свежие HTML в `external/`, запустить `extract-msd-dri-parser.py` для парсинга всех таблиц, затем запустить build-скрипты оверлеев:
   ```
   python3 build-minerals-overlay.py
   python3 build-vitamins-overlay.py
   python3 build-macronutrients-per-kg-overlay.py
   python3 build-data-index.py
   ```
4. **WHO Hb:** запустить `extract-who-hb.py` — валидирует JSON. Для переэкстракции: скачать PDF вручную с iris.who.int (JS-only), запустить pdfplumber.
5. **Wikipedia lab ranges:** запустить `extract-wiki-lab-ranges.py` — получает свежий викитекст через API и перестраивает JSON.

## TODO

- [x] Скачать National Academies DRI PDF (1997, 2011) — в `external/`
- [x] Скачать WHO 2024 Hb Guideline PDF — в `external/`
- [x] Machine-verified extraction для всех DRI значений через оверлейные слои
- [x] Cross-verification P/Mg через NCBI Bookshelf (IOM 1997)
- [x] Единый data-index.json манифест
- [ ] Экстракция DRI Summary Tables из National Academies PDF (pp. 529–542) — для Na/K возрастной разбивки
- [ ] Докачка дополнительных индикаторов WHO GHO через OData API
- [ ] Мониторинг: EFSA DRV Finder (при появлении статического экспорта)
- [ ] Мониторинг: NIH ODS (при снятии Cloudflare-блокировки)
