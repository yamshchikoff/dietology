# Датасет 8: WHO Распространённость диабета

Возраст-стандартизированная распространённость повышенной глюкозы натощак (≥7.0 ммоль/л) или приёма лекарств от диабета.

**Источник:** WHO Global Health Observatory (GHO)
**Уровень:** A — CC BY 4.0

## Размерности

- 210 стран (ISO3 codes)
- 1990–2022
- Пол: SEX_BTSX, SEX_MLE, SEX_FMLE
- Возрастные группы: AGEGROUP_YEARS18-PLUS, AGEGROUP_YEARS30-PLUS

**Кардинальность:** 41 580 записей

## Возвращаемые поля

- `country_code` — ISO3 код страны (строка)
- `year` — год (целое)
- `sex` — "SEX_BTSX", "SEX_MLE" или "SEX_FMLE"
- `agegroup` — "AGEGROUP_YEARS18-PLUS" или "AGEGROUP_YEARS30-PLUS"
- `value` — распространённость в % (число)
- `low` — нижняя граница 95% ДИ (число)
- `high` — верхняя граница 95% ДИ (число)
- `parent_region` — регион WHO (строка)

## Выбор возрастной группы

- **18+** — общепопуляционные сравнения
- **30+** — выше распространённость (старшая популяция). Используй при сравнении с гайдлайнами, скринирующими с 30+

## КРИТИЧЕСКИ: природа данных

Модельные оценки. Всегда цитируй `value` с `low`/`high`.

## Describe-инструмент

**`describe_who_diabetes()`** — без параметров. Возвращает: `countries[]` (210 ISO3), `years` {min, max}, `sexes[]`, `agegroups[]`, `total_records`. Вызови, если не знаешь ISO3-код или доступные agegroups.

## Инструмент

**`query_who_diabetes(country_code, year, sex, agegroup)`**

Параметры-фильтры (все опциональны):
- `country_code` (str | None) — ISO3 код
- `year` (int | None) — год (1990–2022)
- `sex` (str | None) — "SEX_BTSX", "SEX_MLE" или "SEX_FMLE"
- `agegroup` (str | None) — "AGEGROUP_YEARS18-PLUS" или "AGEGROUP_YEARS30-PLUS"

Без фильтров возвращает все записи — указывай хотя бы страну или год.
