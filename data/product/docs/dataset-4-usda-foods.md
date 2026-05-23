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

**`describe_usda_foods()`** — без параметров. Возвращает: `nutrients[]` (25 имён), `food_categories[]`, `total_foods` (363). Вызови, если не знаешь точное имя nutrient или какие категории доступны.

## Инструмент

**`query_usda_foods(food_name_substring, nutrient, max_results)`**

Параметры-фильтры (все опциональны):
- `food_name_substring` (str | None) — поиск по подстроке в названии. "apple" найдёт "Apples, raw, with skin"
- `nutrient` (str | None) — имя нутриента для сортировки по убыванию (вернуть продукты, богатые этим нутриентом)
- `max_results` (int) — ограничение количества результатов (по умолчанию 50)

Если вызвать без фильтров — вернёт первые max_results продуктов.
