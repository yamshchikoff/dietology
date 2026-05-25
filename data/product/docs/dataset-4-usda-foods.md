# Датасет 4: USDA Состав продуктов

Состав нутриентов для 363 сырых продуктов. Все значения на 100 г съедобной порции.

**Источник:** USDA FoodData Central Foundation Foods
**Уровень:** A — CC0 (общественное достояние)

## Продукты

363 наименования — сырые ингредиенты: овощи, фрукты, злаки, мясо, молочные продукты, бобовые, орехи, масла. Не брендированные продукты.

## Нутриенты (25)

**Проксиматы (8):** Energy, Protein, Total lipid (fat), Fatty acids (total saturated), Fatty acids (total trans), Carbohydrate (by difference), Fiber (total dietary), Cholesterol

**Минералы (7):** Calcium (Ca), Iron (Fe), Magnesium (Mg), Phosphorus (P), Potassium (K), Sodium (Na), Zinc (Zn)

**Витамины (10):** Vitamin C (total ascorbic acid), Vitamin D (D2+D3 IU), Vitamin E (alpha-tocopherol), Vitamin K (phylloquinone), Thiamin, Riboflavin, Niacin, Vitamin B-6, Vitamin B-12, Folate (total)

## Возвращаемые поля

Для каждого продукта — объект с полями:
- `food_name` — название продукта (строка)
- `food_category` — категория (строка, например "Vegetables", "Fruits", "Dairy")
- `fdc_id` — идентификатор FoodData Central (целое число)
- Плюс 25 числовых полей с именами нутриентов (см. выше). Отсутствующие значения — null.

## Соглашение о единицах

**Все значения на 100 г съедобной порции.** Для получения фактического потребления:
```
факт = value × (потреблённый вес в граммах / 100)
```

## Describe-инструмент

**`describe_usda_foods(category?)`**

- **Без аргументов** — возвращает индекс: `nutrients[]` (25 имён), `food_categories[]`, `total_foods` (363).
- **С аргументом `category`** — возвращает drill-down: `{category, foods: [{food_name, fdc_id}], count}`. Все продукты в указанной категории с точными именами для использования в query.

## Инструмент

**`query_usda_foods(food_name, nutrient, max_results)`**

Параметры (все опциональны):
- `food_name` (str | None) — **точное** имя продукта. Скопируй из вывода `describe_usda_foods(category=...)`.
- `nutrient` (str | None) — имя нутриента для сортировки по убыванию. Валидные имена — из `describe_usda_foods()`.
- `max_results` (int) — ограничение количества результатов (по умолчанию 50).

Если вызвать без `food_name` — вернёт первые max_results продуктов.
