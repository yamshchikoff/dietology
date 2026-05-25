# Датасет 9: Лабораторные референсные диапазоны

Типичные референсные диапазоны для 347 анализов крови и мочи. 16 категорий.

**Источник:** Wikipedia (агрегация из MedlinePlus, Uppsala University Hospital, Mayo Clinic, ~200 источников)
**Уровень:** C — третичный источник

## Категории (16)

| Категория | Тестов | Примеры |
|-----------|--------|---------|
| blood_gases | 75 | pH, pCO2, pO2, HCO3 |
| sex_hormones | 38 | Testosterone, Estradiol, Progesterone |
| hematology_rbc | 31 | Hemoglobin, Hematocrit, MCV, Ferritin |
| ions_and_trace_metals | 38 | Sodium, Potassium, Calcium, Iron |
| metabolites | 24 | Glucose, Creatinine, Urea, Uric acid |
| other_hormones | 28 | Cortisol, Insulin, Growth hormone |
| thyroid | 19 | TSH, T3, T4 |
| vitamins | 17 | Vitamin D, B12, Folate |
| lipids | 15 | Cholesterol, Triglycerides, HDL, LDL |
| liver_function | 12 | ALT, AST, Bilirubin, Albumin |
| cardiac | 12 | Troponin, CK-MB, NT-proBNP |
| tumour_markers | 12 | PSA, CA-125, CEA |
| immunology | 11 | CRP, Rheumatoid factor |
| coagulation | 9 | PT, INR, aPTT |
| toxicology | 5 | Lead, Mercury |
| unknown | 1 | — |

## Возвращаемые поля

- `test_name` — название теста (строка)
- `category` — категория (строка, см. таблицу)
- `range_type` — тип диапазона, напр. «standard», «optimal», «26, 50» (строка или null; может отсутствовать)
- `lower` — нижняя граница (строка или null, может быть нечисловым значением вроде «<0.5»; может отсутствовать)
- `upper` — верхняя граница (строка или null; может отсутствовать)
- `unit` — единица измерения (строка или null)

## КРИТИЧЕСКИ: уровень C

Это **третичный источник**. Референсные диапазоны различаются между лабораториями и методами анализа.

При цитировании ты обязан:
- Говорить «типичный референсный диапазон», не «норма»
- Упоминать уровень источника (C)
- Рекомендовать клиническую верификацию для диагностических решений

## Describe-инструмент

**`describe_lab_ranges(category?)`**

- **Без аргументов** — возвращает индекс: `categories[]` (16 имён с количеством тестов в каждой), `total_tests` (347).
- **С аргументом `category`** — возвращает drill-down: `{category, tests: [{test_name, lower, upper, unit, ...}], count}`. Все тесты в указанной категории с точными именами для использования в query.

## Инструмент

**`query_lab_ranges(test_name, category)`**

Параметры-фильтры (все опциональны):
- `test_name` (str | None) — **точное** имя теста. Скопируй из вывода `describe_lab_ranges(category=...)`.
- `category` (str | None) — фильтр по категории, например `"lipids"`, `"thyroid"`. Валидные имена — из `describe_lab_ranges()`.

Без фильтров возвращает все 347 тестов.
