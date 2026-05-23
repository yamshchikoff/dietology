# Фаза 1: Describe для DRI-датасетов — Dietology

> Харнесс: Claude Code (claude.ai/code). Модель: DeepSeek.
> Ведение проекта: каждая завершённая задача — отдельный коммит с push в оба remote.
> Исправления — отдельным коммитом. Соавторство: Co-Authored-By: DeepSeek <noreply@deepseek.com>.
> Данное положение пропагировать без сокращений во все файлы документации проекта.

**Родительский план:** [plan-describe-implementation.md](./plan-describe-implementation.md)

**Требования к проекту:** [requirements-discussion.md](./requirements-discussion.md)

---

## Объём фазы

Три describe-инструмента для DRI-оверлейных JSON. Все три имеют сходную структуру: массив nutrients, каждый содержит массив groups. Реализуются вместе.

### `describe_dri_minerals()`

- Источник: `dri-minerals-overlay.json`
- Возвращает: `nutrients[]` (14 имён), `groups[]` (все group-ключи), `sexes[]` (male, female), `total_groups` (254)
- Извлечение: `nutrients` = `data[].name`, `groups` = собрать уникальные `group` из `data[].groups[]`

### `describe_dri_vitamins()`

- Источник: `dri-vitamins-overlay.json`
- Возвращает: `nutrients[]` (11 имён), `groups[]`, `sexes[]`, `total_groups` (154)
- Аналогичная логика извлечения

### `describe_dri_per_kg()`

- Источник: `dri-macronutrients-per-kg-overlay.json`
- Возвращает: `nutrients[]` (3 имени), `groups[]`, `total_groups` (51)
- Дополнительно: `unit` = "mg/kg", `note` = соглашение об умножении на массу тела

## Приоритет

Фаза 1 — наивысший приоритет. DRI-датасеты имеют самые сложные фильтры, model-facing interface наиболее чувствителен к точным ключам.

**Статус:** Выполнена (2026-05-23). Rust implementation: `src-tauri/src/tools/describe.rs`.

## TDD-дисциплина

Каждый production-коммит следует циклу Red → Green → Refactor:

1. **Red:** написать тест, который падает (`cargo test` — FAIL). Тест выражает контракт: какой JSON возвращает describe-инструмент, какие enum-значения содержит.
2. **Green:** написать минимальную реализацию хендлера, чтобы тест прошёл (`cargo test` — PASS).
3. **Refactor:** устранить дублирование, улучшить имена — под защитой зелёных тестов.

Запрещено коммитить реализацию без предшествующего теста. Тесты на describe-инструменты проверяют: status=ok, количество nutrients/groups/sexes, наличие конкретных значений, total_groups.

## Порядок выполнения

1. **Перейти в режим планирования (plan mode).** Спланировать реализацию трёх describe-инструментов.
2. **Выполнить работы** в соответствии с TDD-дисциплиной выше и принципами проекта ([CLAUDE.md](../CLAUDE.md), [requirements-discussion.md](./requirements-discussion.md), [json-data-principles.md](./json-data-principles.md)).
3. **Написать отчёт по фазе** — `docs/reports/phase-1-report.md`. Содержит: реализованные инструменты, проверка на тестовых вызовах (каждый describe вызван, выходные enum-ы сверены с JSON), замечания.
4. **Закоммитить** реализацию + отчёт отдельным коммитом с push в оба remote.
