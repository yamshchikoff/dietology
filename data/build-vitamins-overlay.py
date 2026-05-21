# Build dri-vitamins-overlay.json — merge machine-parsed values
# with manual-transcription metadata for 11 vitamins.
# Run: python3 build-vitamins-overlay.py

import json
import os
from datetime import datetime, timezone

SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))

MANUAL = os.path.join(SCRIPT_DIR, "dri-vitamins.json")
PARSED = os.path.join(SCRIPT_DIR, "dri-vitamins-parsed.json")
OVERLAY_OUT = os.path.join(SCRIPT_DIR, "dri-vitamins-overlay.json")

SOURCE_URL = "https://www.msdmanuals.com/professional/multimedia/table/recommended-daily-intakes-for-vitamins"


def load_json(path):
    with open(path) as f:
        return json.load(f)


def main():
    print("Building dri-vitamins-overlay.json")
    print("=================================")
    print()

    manual = load_json(MANUAL)
    parsed = load_json(PARSED)

    # Build manual metadata lookup
    manual_meta = {}
    for n in manual["nutrients"]:
        manual_meta[n["name"]] = {
            "unit": n.get("unit"),
            "ul": n.get("ul"),
            "ul_unit": n.get("ul_unit"),
            "ul_note": n.get("ul_note"),
            "unit_note": n.get("unit_note"),
        }

    nutrients = []
    for n in parsed["nutrients"]:
        name = n["name"]
        meta = manual_meta.get(name, {})

        entry = {
            "name": name,
            "unit": meta.get("unit", n.get("unit")),
            "source_id": "msd-manual-dri",
            "source_urls": [SOURCE_URL],
            "groups": [],
        }

        # UL — prefer manual (has null+note combos), fall back to parsed
        entry["ul"] = meta.get("ul", n.get("ul"))
        if "ul_unit" in meta or "ul_unit" in n:
            entry["ul_unit"] = meta.get("ul_unit", n.get("ul_unit"))

        # String metadata fields — merge: manual first, then parsed
        for field in ("ul_note", "unit_note"):
            val = meta.get(field) or n.get(field)
            if val:
                entry[field] = val

        # Groups — from parsed (machine-verified values)
        for g in n["groups"]:
            val = g["value"]
            group_entry = {
                "group": g["group"],
                "sex": g.get("sex", "any"),
                "age_range": g.get("age_range", ""),
                "value": int(val) if val == int(val) else val,
                "type": g.get("type", "RDA"),
            }
            entry["groups"].append(group_entry)

        nutrients.append(entry)

    total_groups = sum(len(n["groups"]) for n in nutrients)

    output = {
        "_meta": {
            "schema": "dri-vitamins-overlay-v1",
            "description": "Merged DRI values for 11 vitamins: machine-parsed values from MSD Manual Professional HTML table, combined with metadata (unit_note, ul_note, proper unit names) from manual transcription.",
            "build_script": "data/build-vitamins-overlay.py",
            "build_date": datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
            "sources": ["msd-manual-dri"],
            "input_files": [
                "data/dri-vitamins.json (metadata: unit_note, ul_note, proper units)",
                "data/dri-vitamins-parsed.json (machine-parsed values)",
            ],
            "stats": {
                "nutrients": len(nutrients),
                "total_groups": total_groups,
                "granularity": "14 groups each: 2 infants, 2 children, 4 males, 4 females, 1 pregnant, 1 breastfeeding. Pregnancy/breastfeeding are NOT split by age — MSD vitamins table does not provide teen/adult sub-breakdown (unlike mineral tables).",
            },
        },
        "nutrients": nutrients,
    }

    with open(OVERLAY_OUT, "w") as f:
        json.dump(output, f, ensure_ascii=False, indent=2)

    print(f"Written {OVERLAY_OUT}")
    print(f"  {len(nutrients)} nutrients, {total_groups} groups")

    # Print per-nutrient summary
    for n in nutrients:
        notes = []
        if n.get("ul_note"):
            notes.append("ul_note")
        if n.get("unit_note"):
            notes.append("unit_note")
        note_str = f" ({', '.join(notes)})" if notes else ""
        print(f"  {n['name']:20s} {len(n['groups']):2d} groups  {n['unit']}{note_str}")

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
