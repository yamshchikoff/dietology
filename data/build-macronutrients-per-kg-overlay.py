# Build dri-macronutrients-per-kg-overlay.json — production overlay for 3 macronutrients.
# All values are machine-parsed from MSD Manual Professional HTML table.
# No manual transcription dependencies.
# Run: python3 build-macronutrients-per-kg-overlay.py

import json
import os
from datetime import datetime, timezone

SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))

PARSED = os.path.join(SCRIPT_DIR, "dri-macronutrients-per-kg-parsed.json")
OVERLAY_OUT = os.path.join(SCRIPT_DIR, "dri-macronutrients-per-kg-overlay.json")

SOURCE_URL = "https://www.msdmanuals.com/professional/multimedia/table/recommended-dietary-reference-intakes-for-some-macronutrients-food-and-nutrition-board-institute-of-medicine-of-the-national-academies"

CITATION = "Institute of Medicine. Dietary Reference Intakes for Calcium, Phosphorus, Magnesium, Vitamin D, and Fluoride. Washington, DC: National Academy Press; 1997."


def load_json(path):
    with open(path) as f:
        return json.load(f)


def main():
    print("Building dri-macronutrients-per-kg-overlay.json")
    print("================================================")
    print()

    parsed = load_json(PARSED)

    nutrients = []
    for n in parsed["nutrients"]:
        name = n["name"]

        entry = {
            "name": name,
            "unit": n["unit"],  # always mg/kg
            "category": "macromineral",
            "source_id": "msd-macronutrients-per-kg",
            "source_urls": [SOURCE_URL],
            "groups": [],
        }

        for g in n["groups"]:
            val = g["value"]
            age_range = g.get("age_range", "")
            if not age_range or age_range in ("—", "–", "-"):
                age_range = None
            group_entry = {
                "group": g["group"],
                "sex": g.get("sex", "any"),
                "age_range": age_range,
                "value": int(val) if val == int(val) else val,
                "type": g.get("type", "RDA"),
            }
            entry["groups"].append(group_entry)

        nutrients.append(entry)

    total_groups = sum(len(n["groups"]) for n in nutrients)

    output = {
        "_meta": {
            "schema": "dri-macronutrients-per-kg-overlay-v1",
            "description": "Per-kg DRI values for Ca, P, Mg: machine-parsed values from MSD Manual Professional HTML table. 0 manual transcription dependencies.",
            "build_script": "data/build-macronutrients-per-kg-overlay.py",
            "build_date": datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
            "sources": ["msd-macronutrients-per-kg"],
            "note": "Per-kg values from MSD Manual 'Recommended Dietary Reference Intakes for Some Macronutrients' table. ALL values in mg/kg — model MUST multiply by individual body weight (kg). No reference-weight computation applied. Infants: AI. Children and adults: RDA.",
            "citation": CITATION,
            "input_files": [
                "data/dri-macronutrients-per-kg-parsed.json (machine-parsed values)",
            ],
            "stats": {
                "nutrients": len(nutrients),
                "total_groups": total_groups,
                "granularity": "17 groups each: 2 infants, 3 children (1-3, 4-6, 7-10yr), 6 males (9-13 through >70), 6 females (9-13 through >70). Pregnancy/breastfeeding: single groups (not age-split).",
                "convention": "All values in mg/kg of body weight. Multiply by individual body weight for absolute daily intake. Infants: AI. Children and adults: RDA.",
            },
        },
        "nutrients": nutrients,
    }

    with open(OVERLAY_OUT, "w") as f:
        json.dump(output, f, ensure_ascii=False, indent=2)

    print(f"Written {OVERLAY_OUT}")
    print(f"  {len(nutrients)} nutrients, {total_groups} groups")

    for n in nutrients:
        print(f"  {n['name']:15s} {len(n['groups']):2d} groups  {n['unit']}  category={n.get('category', '-')}")

    # Verify
    for n in nutrients:
        assert n.get("name"), "Missing name"
        assert n.get("unit"), f"{n['name']}: missing unit"
        assert n.get("source_id"), f"{n['name']}: missing source_id"
        for g in n["groups"]:
            assert g.get("value") is not None, f"{n['name']}/{g['group']}: missing value"
            assert g.get("type") in ("AI", "RDA"), f"{n['name']}/{g['group']}: bad type"

    print("All assertions passed.")


if __name__ == "__main__":
    main()
