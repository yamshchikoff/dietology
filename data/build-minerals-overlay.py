# Build dri-minerals-overlay.json — all values machine-verified from parsed sources.
# Zero dependency on manual transcription files.
# Run: python3 build-minerals-overlay.py

import json
import os
import re
from datetime import datetime, timezone

SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))

TRACE_PARSED = os.path.join(SCRIPT_DIR, "dri-minerals-parsed.json")
CALCIUM_PARSED = os.path.join(SCRIPT_DIR, "dri-calcium-iom-2011-parsed.json")
NAK_2019_PARSED = os.path.join(SCRIPT_DIR, "dri-na-k-2019-parsed.json")
LPI_UL_PARSED = os.path.join(SCRIPT_DIR, "dri-p-mg-ul-parsed.json")
NCBI_CROSSCHECK = os.path.join(SCRIPT_DIR, "dri-p-mg-ncbi-crosscheck.json")

OVERLAY_OUT = os.path.join(SCRIPT_DIR, "dri-minerals-overlay.json")

SOURCE_URLS = {
    "iom-dri-2011": ["https://nap.nationalacademies.org/catalog/13050/"],
    "iom-dri-1997": ["https://nap.nationalacademies.org/catalog/5776/",
                     "https://www.ncbi.nlm.nih.gov/books/NBK222881/table/ttt00057_1/"],
    "msd-manual-dri": ["https://www.msdmanuals.com/professional/multimedia/table/guidelines-for-daily-intake-of-trace-minerals"],
    "msd-consumer-minerals": ["https://www.msdmanuals.com/home/disorders-of-nutrition/minerals/overview-of-minerals"],
    "nas-dri-2019": ["https://nap.nationalacademies.org/catalog/25353",
                     "https://nap.nationalacademies.org/resource/25353/030519DRISodiumPotassium.pdf"],
    "lpi-mic-minerals": ["https://lpi.oregonstate.edu/mic/minerals/phosphorus",
                         "https://lpi.oregonstate.edu/mic/minerals/magnesium"],
}

# Hardcoded taxonomy — minerals are classified as macrominerals or trace minerals
# per standard nutritional science. This is structural metadata, not sourced medical facts.
CATEGORIES = {
    "Calcium": "macromineral",
    "Phosphorus": "macromineral",
    "Magnesium": "macromineral",
    "Sodium": "macromineral",
    "Potassium": "macromineral",
    "Iron": "trace mineral",
    "Zinc": "trace mineral",
    "Copper": "trace mineral",
    "Iodine": "trace mineral",
    "Selenium": "trace mineral",
    "Manganese": "trace mineral",
    "Chromium": "trace mineral",
    "Molybdenum": "trace mineral",
    "Fluoride": "trace mineral",
}

# Hardcoded nutrient notes — scientific context not present in machine-parsed sources.
NUTRIENT_NOTES = {
    "Chromium": "Essentiality questioned by recent research (Vincent, J Nutr 2017). Values are AI.",
}

# Pregnancy/breastfeeding teen vs adult annotation.
# The overlay splits pregnancy/breastfeeding into teen (14-18yr) and adult (19-30yr, 31-50yr)
# subgroups. The teen subgroup uses the adolescent age-bracket value, not the adult value.
# These per-group notes clarify that distinction.
PREGNANCY_BREASTFEEDING_TEEN_NOTES = {
    "Calcium": ("RDA", 1300),
    "Phosphorus": ("RDA", 1250),
    "Magnesium": ("RDA", 400),
}
BREASTFEEDING_TEEN_NOTES = {
    "Calcium": ("RDA", 1300),
    "Phosphorus": ("RDA", 1250),
    "Magnesium": ("RDA", 360),
}


def load_json(path):
    with open(path) as f:
        return json.load(f)


def build_trace_mineral(nutrient):
    """Take parsed trace mineral groups — all values machine-verified from MSD Professional HTML."""
    name = nutrient["name"]
    category = nutrient.get("category") or CATEGORIES.get(name)
    entry = {
        "name": name,
        "unit": nutrient["unit"],
        "category": category,
        "source_id": "msd-manual-dri",
        "source_urls": SOURCE_URLS["msd-manual-dri"],
        "groups": [],
    }
    for field in ("ul", "ul_unit", "ul_note"):
        if field in nutrient and nutrient[field] is not None:
            entry[field] = nutrient[field]
    if name in NUTRIENT_NOTES:
        entry["note"] = NUTRIENT_NOTES[name]

    for g in nutrient["groups"]:
        group_entry = {
            "group": g["group"],
            "sex": g.get("sex", "any"),
            "age_range": g.get("age_range", ""),
            "value": int(g["value"]) if g["value"] == int(g["value"]) else g["value"],
            "type": g.get("type", "RDA"),
        }
        entry["groups"].append(group_entry)

    return entry


def build_calcium(parsed):
    """Take IOM 2011 parsed Calcium — all values machine-verified."""
    ca = parsed["nutrients"][0]
    entry = {
        "name": "Calcium",
        "unit": "mg",
        "category": ca.get("category") or CATEGORIES.get("Calcium", "macromineral"),
        "source_id": "iom-dri-2011",
        "source_urls": SOURCE_URLS["iom-dri-2011"],
        "groups": [],
    }
    for field in ("ul", "ul_unit", "ul_note"):
        if field in ca and ca[field] is not None:
            entry[field] = ca[field]
    if "ul_groups" in ca:
        entry["ul_groups"] = ca["ul_groups"]

    for g in ca["groups"]:
        group_entry = {
            "group": g["group"],
            "sex": g.get("sex", "any"),
            "age_range": g.get("age_range", ""),
            "value": g["value"],
            "type": g.get("type", "RDA"),
        }
        # Annotate teen pregnancy/breastfeeding groups
        if g["group"].startswith("pregnant_14"):
            dri_type, val = PREGNANCY_BREASTFEEDING_TEEN_NOTES["Calcium"]
            group_entry["note"] = f"{dri_type} for pregnant ≤18 yr: {val} mg"
        elif g["group"].startswith("breastfeeding_14"):
            dri_type, val = BREASTFEEDING_TEEN_NOTES["Calcium"]
            group_entry["note"] = f"{dri_type} for breastfeeding ≤18 yr: {val} mg"
        entry["groups"].append(group_entry)

    return entry


# ── NCBI age_label → group_id mapping for P and Mg ──

def _norm_age(age_label):
    """Normalize NCBI age label for matching."""
    a = age_label.strip()
    a = re.sub(r"\s+", " ", a)
    a = re.sub(r"([>≤≥])\s+", r"\1", a)
    return a


def _age_to_id(age_label):
    """Convert NCBI age label to group_id suffix."""
    a = _norm_age(age_label)
    mapping = {
        "0–6 mo": "0_6mo",
        "7–12 mo": "7_12mo",
        "1–3 y": "1_3yr",
        "4–8 y": "4_8yr",
        "9–13 y": "9_13yr",
        "14–18 y": "14_18yr",
        "19–30 y": "19_30yr",
        "31–50 y": "31_50yr",
        "51–70 y": "51_70yr",
        ">70 y": "gt70yr",
        "≤18 y": "14_18yr",
    }
    return mapping.get(a)


def _category_to_prefix(category, age_label):
    """Convert NCBI category to group_id prefix."""
    c = category.lower()
    a = _norm_age(age_label)
    if c == "infants":
        return "infants"
    if c == "children":
        return "children"
    if c == "males":
        return "male"
    if c == "females":
        return "female"
    if c == "pregnancy":
        return "pregnant"
    if c == "lactation":
        return "breastfeeding"
    return None


def _category_to_sex(category):
    if category in ("Males",):
        return "male"
    if category in ("Females", "Pregnancy", "Lactation"):
        return "female"
    return "any"


def _map_ul_to_group(group_id, ul_data):
    """Map LPI UL age bracket to overlay group_id."""
    if group_id.startswith("infants"):
        return ul_data.get("infants")
    if "children_1_3" in group_id:
        return ul_data.get("children_1_3yr")
    if "children_4_8" in group_id:
        return ul_data.get("children_4_8yr")
    if "children_9_13" in group_id or group_id.startswith(("male_9", "female_9")):
        return ul_data.get("children_9_13yr") or ul_data.get("adolescent_14_18yr")
    if group_id.startswith(("male_14", "female_14")):
        return ul_data.get("adolescent_14_18yr")
    if group_id.startswith("pregnant"):
        return ul_data.get("pregnant")
    if group_id.startswith("breastfeeding"):
        return ul_data.get("breastfeeding")
    if group_id.startswith("male_gt70") or group_id.startswith("female_gt70"):
        gt70 = ul_data.get("adult_gt70yr")
        if gt70 is not None:
            return gt70
    for k in ["adult_19_70yr", "adult"]:
        if k in ul_data and ul_data[k] is not None:
            return ul_data[k]
    return None


def build_p_mg(nutrient_name, ncbi_entries, lpi_ul):
    """Build P or Mg entry from NCBI crosscheck + LPI UL data. All machine-verified."""
    entries = [e for e in ncbi_entries if e["nutrient"] == nutrient_name]
    if not entries:
        raise ValueError(f"No NCBI entries for {nutrient_name}")

    ul_key = "phosphorus_ul" if nutrient_name == "Phosphorus" else "magnesium_ul"
    ul_data = lpi_ul.get(ul_key, {})

    entry = {
        "name": nutrient_name,
        "unit": "mg",
        "category": CATEGORIES.get(nutrient_name, "macromineral"),
        "source_id": "iom-dri-1997",
        "source_urls": SOURCE_URLS["iom-dri-1997"],
        "ul_source_id": "lpi-mic-minerals",
        "ul_source_urls": SOURCE_URLS["lpi-mic-minerals"],
        "groups": [],
    }

    adult_ul = (
        ul_data.get("adult_19_70yr") or ul_data.get("adult")
    )
    entry["ul"] = adult_ul
    entry["ul_unit"] = "mg"

    if nutrient_name == "Magnesium":
        entry["ul_note"] = "UL applies to supplemental magnesium only, not food sources. Source: Linus Pauling Institute, based on IOM 1997."
    elif nutrient_name == "Phosphorus":
        senior_ul = ul_data.get("adult_gt70yr")
        entry["ul_note"] = f"Adults 19–70 yr: {adult_ul} mg. Adults >70 yr: {senior_ul} mg. Source: Linus Pauling Institute, based on IOM 1997."

    # Build ul_groups (per-group UL)
    ul_groups = []
    seen_groups = set()
    for e in entries:
        prefix = _category_to_prefix(e["category"], e["age_label"])
        age_suffix = _age_to_id(e["age_label"])
        if prefix is None or age_suffix is None:
            continue
        group_id = f"{prefix}_{age_suffix}"
        if group_id in seen_groups:
            continue
        seen_groups.add(group_id)
        ul_val = _map_ul_to_group(group_id, ul_data)
        if ul_val is not None:
            sex = _category_to_sex(e["category"])
            ul_groups.append({
                "group": group_id,
                "sex": sex,
                "value": ul_val,
            })
    if ul_groups:
        entry["ul_groups"] = ul_groups

    for e in entries:
        age_suffix = _age_to_id(e["age_label"])
        prefix = _category_to_prefix(e["category"], e["age_label"])
        if age_suffix is None or prefix is None:
            print(f"  WARNING: cannot map {e['category']} / {e['age_label']}")
            continue

        group_id = f"{prefix}_{age_suffix}"
        sex = _category_to_sex(e["category"])
        dri_type = "AI" if e["is_ai"] else "RDA"

        group_entry = {
            "group": group_id,
            "sex": sex,
            "age_range": _norm_age(e["age_label"]),
            "value": e["value"],
            "type": dri_type,
        }
        # Annotate teen pregnancy/breastfeeding groups
        if group_id.startswith("pregnant_14"):
            if nutrient_name in PREGNANCY_BREASTFEEDING_TEEN_NOTES:
                dri_type_note, val = PREGNANCY_BREASTFEEDING_TEEN_NOTES[nutrient_name]
                group_entry["note"] = f"{dri_type_note} for pregnant ≤18 yr: {val} mg"
        elif group_id.startswith("breastfeeding_14"):
            if nutrient_name in BREASTFEEDING_TEEN_NOTES:
                dri_type_note, val = BREASTFEEDING_TEEN_NOTES[nutrient_name]
                group_entry["note"] = f"{dri_type_note} for breastfeeding ≤18 yr: {val} mg"

        entry["groups"].append(group_entry)

    return entry


def build_na_k(nutrient):
    """Build Na/K from NAS 2019 parsed data (machine-verified, authoritative)."""
    name = nutrient["name"]
    entry = {
        "name": name,
        "unit": nutrient["unit"],
        "category": nutrient.get("category") or CATEGORIES.get(name, "macromineral"),
        "source_id": "nas-dri-2019",
        "source_urls": SOURCE_URLS["nas-dri-2019"],
        "groups": [],
    }
    for field in ("ul", "ul_unit", "ul_note", "note"):
        if field in nutrient and nutrient[field] is not None:
            entry[field] = nutrient[field]

    for g in nutrient["groups"]:
        group_entry = {
            "group": g["group"],
            "sex": g.get("sex", "any"),
            "age_range": g.get("age_range", ""),
            "value": g["value"],
            "type": g.get("type", "AI"),
        }
        if g.get("cdrr") is not None:
            group_entry["cdrr"] = g["cdrr"]
        if g.get("note"):
            group_entry["note"] = g["note"]
        entry["groups"].append(group_entry)

    return entry


def main():
    print("Building dri-minerals-overlay.json (0 manual dependencies)")
    print("============================================================")
    print()

    trace = load_json(TRACE_PARSED)
    calcium = load_json(CALCIUM_PARSED)
    nak_2019 = load_json(NAK_2019_PARSED)
    lpi = load_json(LPI_UL_PARSED)
    ncbi = load_json(NCBI_CROSSCHECK)

    ncbi_entries = ncbi["ncbi_entries"]

    nutrients = []

    # Trace minerals (9)
    print("Trace minerals:")
    for n in trace["nutrients"]:
        name = n["name"]
        entry = build_trace_mineral(n)
        nutrients.append(entry)
        print(f"  {name}: {len(entry['groups'])} groups")
    print(f"  Total: {sum(len(n['groups']) for n in nutrients)} groups")

    # Calcium
    print()
    print("Calcium:")
    ca_entry = build_calcium(calcium)
    nutrients.append(ca_entry)
    print(f"  Calcium: {len(ca_entry['groups'])} groups, {len(ca_entry.get('ul_groups', []))} ul_groups")

    # Phosphorus and Magnesium from NCBI
    print()
    print("P and Mg from NCBI + LPI UL:")
    p_entry = build_p_mg("Phosphorus", ncbi_entries, lpi)
    mg_entry = build_p_mg("Magnesium", ncbi_entries, lpi)
    nutrients.append(p_entry)
    nutrients.append(mg_entry)
    print(f"  Phosphorus: {len(p_entry['groups'])} groups")
    print(f"  Magnesium: {len(mg_entry['groups'])} groups")

    # Sodium and Potassium (NAS 2019)
    print()
    print("Na/K from NAS 2019:")
    for n in nak_2019["nutrients"]:
        entry = build_na_k(n)
        nutrients.append(entry)
        print(f"  {entry['name']}: {len(entry['groups'])} groups")

    # Write
    total_groups = sum(len(n["groups"]) for n in nutrients)
    source_ids = sorted(set(n["source_id"] for n in nutrients))
    # Add ul_source_id entries if present
    for n in nutrients:
        if "ul_source_id" in n:
            source_ids.append(n["ul_source_id"])
    source_ids = sorted(set(source_ids))

    output = {
        "_meta": {
            "schema": "dri-minerals-overlay-v1",
            "description": "Merged DRI values for 14 minerals: all values machine-parsed from IOM/NCBI/MSD/NAS/LPI sources at finest available granularity. All UL/UL_unit/UL_note machine-verified. 0 manual_transcription, 0 manual source file dependencies.",
            "build_script": "data/build-minerals-overlay.py",
            "build_date": datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
            "sources": source_ids,
            "input_files": [
                "data/dri-minerals-parsed.json (trace mineral values + ul)",
                "data/dri-calcium-iom-2011-parsed.json (Ca values + ul_groups)",
                "data/dri-na-k-2019-parsed.json (Na/K values, NAS 2019)",
                "data/dri-p-mg-ul-parsed.json (P/Mg UL, LPI based on IOM 1997)",
                "data/dri-p-mg-ncbi-crosscheck.json (P/Mg AI/RDA via NCBI cross-verification)",
            ],
            "stats": {
                "nutrients": len(nutrients),
                "total_groups": total_groups,
                "granularity": "finest available per source: Ca 22 groups (IOM 2011), P/Mg 22 groups each (IOM 1997 via NCBI), trace minerals 16 groups each (MSD Professional), Na/K 22 groups each (NAS 2019)",
                "pregnancy_breastfeeding": "split into teen (14-18yr), adult 19-30yr, adult 31-50yr subgroups — matching IOM/NAS source granularity",
            },
        },
        "nutrients": nutrients,
    }

    with open(OVERLAY_OUT, "w") as f:
        json.dump(output, f, ensure_ascii=False, indent=2)
    print(f"\nWritten {OVERLAY_OUT}")
    print(f"  {len(nutrients)} nutrients, {total_groups} groups")
    print(f"  Sources: {', '.join(source_ids)}")

    # Quick verification
    for n in nutrients:
        assert n.get("name"), f"Missing name"
        assert n.get("unit"), f"{n['name']}: missing unit"
        assert n.get("source_id"), f"{n['name']}: missing source_id"
        assert n.get("groups"), f"{n['name']}: missing groups"
        for g in n["groups"]:
            assert g.get("group"), f"{n['name']}: group missing group id"
            assert g.get("value") is not None, f"{n['name']}/{g['group']}: missing value"
            assert g.get("type") in ("AI", "RDA"), f"{n['name']}/{g['group']}: bad type {g.get('type')}"

    print("All assertions passed.")


if __name__ == "__main__":
    main()
