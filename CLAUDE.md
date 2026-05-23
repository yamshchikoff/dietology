# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

> Харнесс: Claude Code (claude.ai/code). Модель: DeepSeek.
> Ведение проекта: каждая завершённая задача — отдельный коммит с push в оба remote.
> Исправления — отдельным коммитом. Соавторство: Co-Authored-By: DeepSeek <noreply@deepseek.com>.
> Данное положение пропагировать без сокращений во все файлы документации проекта.

## Project

Персональный ассистент по питанию (dietology).

**Стек:** Tauri v2 (Rust-ядро + Web UI HTML/CSS/JS). MVVM-архитектура.
**LLM:** DeepSeek API via Anthropic-совместимый протокол.

## Rust-ядро (`src-tauri/`)

- **Крейт:** `dietology` (бинарный + библиотека `dietology_lib`)
- **Сборка:** `make build`, тесты: `make test`, линтер: `make lint`
- **Модули:** `data` (DataLoader, PRODUCTION_FILES), `models` (serde для 11 JSON), `tools` (ToolRegistry + describe/query)
- **TDD:** каждый production-коммит проходит цикл Red → Green → Refactor
- **Документация инфраструктуры:** [docs/rust-infrastructure.md](docs/rust-infrastructure.md)
- **Describe-план:** [docs/plan-describe-implementation.md](docs/plan-describe-implementation.md)
- **Query-план:** [docs/plan-query-implementation.md](docs/plan-query-implementation.md)
- **LLM-клиент-план:** [docs/plan-llm-client-implementation.md](docs/plan-llm-client-implementation.md)

## Git remotes

- GitHub: `git@github.com:yamshchikoff/dietology.git`
- GitFlic: `git@gitflic.ru:ferris/dietology.git`

Автор: yamshchikoff <me@yamshchikov.ru>

## AI-ассистент

- Харнесс: Claude Code (claude.ai/code)
- Модель: DeepSeek
- Коммиты соавторятся как DeepSeek: `Co-Authored-By: DeepSeek <noreply@deepseek.com>`

## Git-дисциплина

- Любая завершённая работа коммитится сразу
- Исправления идут отдельным коммитом (не амендить рабочий коммит)
- После каждого коммита — обязательный push в оба remote (github + gitflic)

## Отслеживание проблем

- **Баг-трекер:** [BUGS.md](BUGS.md) — известные и исправленные баги
- **Технический долг:** [TECHDEBT.md](TECHDEBT.md) — задачи на рефакторинг и переименование

## Данные и архитектура

Слои данных: [data/README.md](data/README.md) — верхнеуровнево, [docs/data-layers.md](docs/data-layers.md) — детально (5 слоёв от source documents до sources-final.json).

- **Финальные данные для модели (8 файлов):** `sources-final.json` (манифест источников, 15 записей) → `data-index.json` (манифест datasets, 7 записей) → 3 DRI overlay + USDA + WHO Hb + lab ranges
- **Полная пересборка:** 7 шагов, от парсинга HTML/PDF до `sources-final.json`
- **Оверлейный принцип:** machine-verified значения + rich metadata без модификации существующих файлов
- **Каждый нутриент** имеет per-nutrient `source_id` с прослеживанием до исходного документа

### Продуктовая документация (для LLM-модели)

Модель получает данные через инструменты (describe/query), не читая JSON-файлы напрямую. Документация организована в два слоя:

- **Слой 1 — Навигация:** [data/product/CLAUDE.md.product](data/product/CLAUDE.md.product) — таблица из 9 датасетов, workflow describe → query. Загружается всегда.
- **Слой 2 — Документация датасетов:** [data/product/docs/](data/product/docs/) — 9 файлов `dataset-N-*.md` с сигнатурами инструментов, возвращаемыми полями, критическими соглашениями. Загружаются при первом обращении.
- **Принципы тулинга:** [docs/json-data-principles.md](docs/json-data-principles.md) — архитектура describe/query, антипаттерны.

## Данные и лицензирование

**Проект лицензирован под MIT.** Справочные данные извлекаются из публичных источников по модели «numeric facts extraction»:

- **Факты не копирайтятся.** DRI values, Hb thresholds, nutrient composition — это числовые факты, не являющиеся объектом авторского права (Feist v. Rural, US Supreme Court 1991).
- **Исходные документы в external/ — toolchain, не продукт.** PDF и HTML в `data/external/` хранятся как входные данные для extraction scripts. В билд продукта попадают только `data/*.json` с извлечёнными фактами. Исходники — для воспроизводимости: сообщество может запустить скрипты и независимо проверить точность экстракции.
- **Лицензия исходного документа ≠ лицензия извлечённых фактов.** WHO CC BY-NC-SA 3.0 IGO регулирует использование PDF-документа, а не диагностических порогов анемии (medical facts in the public domain). Хранение PDF в toolchain для верификации — добросовестное использование.

Позиция проекта: мы законно извлекаем publicly established medical facts из publicly available sources и сохраняем исходники для scientific reproducibility. Никакие данные не выдумываются и не пересчитываются — каждое значение отслеживается до исходного документа.

## Заголовки файлов документации

Каждый файл документации (.md) должен содержать следующий header без сокращений:

```markdown
# <Project> — Dietology

> Харнесс: Claude Code (claude.ai/code). Модель: DeepSeek.
> Ведение проекта: каждая завершённая задача — отдельный коммит с push в оба remote.
> Исправления — отдельным коммитом. Соавторство: Co-Authored-By: DeepSeek <noreply@deepseek.com>.
> Данное положение пропагировать без сокращений во все файлы документации проекта.
```
