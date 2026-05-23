# Extract essential nutrients from USDA Foundation Foods JSON
# Source: USDA FoodData Central, CC0 (Public Domain)
# Run: python3 extract-usda.py

import json
import zipfile
import os
from datetime import datetime, timezone

ESSENTIAL_NUTRIENTS = [
    "Energy",
    "Protein",
    "Total lipid (fat)",
    "Fatty acids, total saturated",
    "Fatty acids, total trans",
    "Carbohydrate, by difference",
    "Fiber, total dietary",
    "Sugars, total including NLEA",
    "Calcium, Ca",
    "Iron, Fe",
    "Magnesium, Mg",
    "Phosphorus, P",
    "Potassium, K",
    "Sodium, Na",
    "Zinc, Zn",
    "Vitamin A, IU",
    "Vitamin C, total ascorbic acid",
    "Vitamin D (D2 + D3), International Units",
    "Vitamin E (alpha-tocopherol)",
    "Vitamin K (phylloquinone)",
    "Thiamin",
    "Riboflavin",
    "Niacin",
    "Vitamin B-6",
    "Vitamin B-12",
    "Folate, total",
    "Cholesterol",
]

SRC = os.path.join(os.path.dirname(os.path.abspath(__file__)), "external", "usda-foundation-foods-2026-04.zip")
DST = os.path.join(os.path.dirname(os.path.abspath(__file__)), "usda-foundation-foods-essential.json")

with zipfile.ZipFile(SRC) as zf:
    name = zf.namelist()[0]
    raw = json.loads(zf.read(name))

foods = raw["FoundationFoods"]
out = []

for food in foods:
    if not isinstance(food, dict):
        continue
    item = {
        "name": food.get("description", ""),
        "category": (food.get("foodCategory") or {}).get("description", ""),
        "fdcId": food.get("fdcId"),
        "nutrients": {},
    }
    for n in food.get("foodNutrients", []):
        name = n["nutrient"]["name"]
        if name in ESSENTIAL_NUTRIENTS:
            amount = n.get("amount")
            if amount is not None and isinstance(amount, (int, float)) and amount < 0:
                amount = 0.0
            item["nutrients"][name] = {
                "amount": amount,
                "unit": n["nutrient"].get("unitName", ""),
            }
    if item["nutrients"]:
        out.append(item)

wrapped = {
    "_meta": {
        "source_id": "usda-fdc-2026-04",
        "extraction_date": datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
        "extraction_script": "data/extract-usda.py",
        "extracted_by": "agent",
        "auto_generated": True,
        "warning": "AUTO-GENERATED FILE. DO NOT EDIT MANUALLY. Run the extraction script to regenerate.",
        "source_claims": {
            "presumed_date": "2026-04",
            "presumed_author": "USDA FoodData Central, Agricultural Research Service"
        }
    },
    "foods": out
}

with open(DST, "w") as f:
    json.dump(wrapped, f, ensure_ascii=False, indent=2)

print(f"Extracted {len(out)} foods to {DST} ({os.path.getsize(DST)} bytes)")
