# Build dri-minerals-overlay.json — best of both worlds:
# machine-verified values from parsers + rich metadata from manual transcription.
# Run: python3 build-minerals-overlay.py

import json
import os
import re
from datetime import datetime, timezone

SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))

MANUAL = os.path.join(SCRIPT_DIR, "dri-minerals.json")
TRACE_PARSED = os.path.join(SCRIPT_DIR, "dri-minerals-parsed.json")
CALCIUM_PARSED = os.path.join(SCRIPT_DIR, "dri-calcium-iom-2011-parsed.json")
NAK_PARSED = os.path.join(SCRIPT_DIR, "dri-macrominerals-absolute-parsed.json")
NCBI_CROSSCHECK = os.path.join(SCRIPT_DIR, "dri-p-mg-ncbi-crosscheck.json")

OVERLAY_OUT = os.path.join(SCRIPT_DIR, "dri-minerals-overlay.json")

SOURCE_URLS = {
    "iom-dri-2011": ["https://nap.nationalacademies.org/catalog/13050/"],
    "iom-dri-1997": ["https://nap.nationalacademies.org/catalog/5776/",
                     "https://www.ncbi.nlm.nih.gov/books/NBK222881/table/ttt00057_1/"],
    "msd-manual-dri": ["https://www.msdmanuals.com/professional/multimedia/table/guidelines-for-daily-intake-of-trace-minerals"],
    "msd-consumer-minerals": ["https://www.msdmanuals.com/home/disorders-of-nutrition/minerals/overview-of-minerals"],
}


def load_json(path):
    with open(path) as f:
        return json.load(f)


def extract_manual_metadata(manual):
    """Extract per-nutrient metadata from manual dri-minerals.json."""
    meta = {}
    for n in manual["nutrients"]:
        name = n["name"]
        entry = {
            "category": n.get("category"),
        }
        for field in ("ul", "ul_unit", "ul_note", "note"):
            if field in n:
                entry[field] = n[field]
        # Per-group notes
        per_group_notes = {}
        for g in n.get("groups", []):
            if g.get("note"):
                per_group_notes[g["group"]] = g["note"]
        if per_group_notes:
            entry["per_group_notes"] = per_group_notes
        meta[name] = entry
    return meta


def build_trace_mineral(nutrient, manual_meta):
    """Take parsed trace mineral groups, metadata from parsed (machine-verified) with manual fallback."""
    name = nutrient["name"]
    entry = {
        "name": name,
        "unit": nutrient["unit"],
        "category": nutrient.get("category") or manual_meta.get("category"),
        "source_id": "msd-manual-dri",
        "source_urls": SOURCE_URLS["msd-manual-dri"],
        "groups": [],
    }
    # UL metadata: prefer machine-parsed, fall back to manual
    for field in ("ul", "ul_unit", "ul_note"):
        if field in nutrient and nutrient[field] is not None:
            entry[field] = nutrient[field]
        elif field in manual_meta:
            entry[field] = manual_meta[field]
    # note: manual-only (interpretive text, no machine source)
    if "note" in manual_meta:
        entry["note"] = manual_meta["note"]

    # Attach per-group notes from manual if any
    pg_notes = manual_meta.get("per_group_notes", {})

    for g in nutrient["groups"]:
        group_entry = {
            "group": g["group"],
            "sex": g.get("sex", "any"),
            "age_range": g.get("age_range", ""),
            "value": int(g["value"]) if g["value"] == int(g["value"]) else g["value"],
            "type": g.get("type", "RDA"),
        }
        # Map manual group_id (e.g. "pregnant") → age-suffixed overlay group_id
        for base_key, note_text in pg_notes.items():
            if g["group"].startswith(base_key):
                group_entry["note"] = note_text
                break
        entry["groups"].append(group_entry)

    return entry


def build_calcium(parsed, manual_meta):
    """Take IOM 2011 parsed Calcium, metadata from parsed (machine-verified) with manual fallback."""
    ca = parsed["nutrients"][0]
    entry = {
        "name": "Calcium",
        "unit": "mg",
        "category": ca.get("category") or manual_meta.get("category", "macromineral"),
        "source_id": "iom-dri-2011",
        "source_urls": SOURCE_URLS["iom-dri-2011"],
        "groups": [],
    }
    # UL metadata: prefer machine-parsed (ca), fall back to manual
    for field in ("ul", "ul_unit", "ul_note"):
        if field in ca and ca[field] is not None:
            entry[field] = ca[field]
        elif field in manual_meta:
            entry[field] = manual_meta[field]
    if "ul_groups" in ca:
        entry["ul_groups"] = ca["ul_groups"]

    pg_notes = manual_meta.get("per_group_notes", {})

    for g in ca["groups"]:
        group_entry = {
            "group": g["group"],
            "sex": g.get("sex", "any"),
            "age_range": g.get("age_range", ""),
            "value": g["value"],
            "type": g.get("type", "RDA"),
        }
        for base_key, note_text in pg_notes.items():
            if g["group"].startswith(base_key):
                group_entry["note"] = note_text
                break
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
        # ≤18 y maps to 14_18yr within pregnant
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


def build_p_mg(nutrient_name, ncbi_entries, manual_meta):
    """Build P or Mg entry from NCBI crosscheck ncbi_entries."""
    entries = [e for e in ncbi_entries if e["nutrient"] == nutrient_name]
    if not entries:
        raise ValueError(f"No NCBI entries for {nutrient_name}")

    entry = {
        "name": nutrient_name,
        "unit": "mg",
        "category": manual_meta.get("category", "macromineral"),
        "source_id": "iom-dri-1997",
        "source_urls": SOURCE_URLS["iom-dri-1997"],
        "groups": [],
    }
    has_ul_meta = any(f in manual_meta for f in ("ul", "ul_unit", "ul_note"))
    for field in ("ul", "ul_unit", "ul_note", "note"):
        if field in manual_meta:
            entry[field] = manual_meta[field]
    if has_ul_meta:
        entry["metadata_source"] = "manual_transcription"

    pg_notes = manual_meta.get("per_group_notes", {})

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
        # Attach per-group note — manual uses base key like "pregnant"
        for base_key, note_text in pg_notes.items():
            if group_id.startswith(base_key):
                group_entry["note"] = note_text
                break

        entry["groups"].append(group_entry)

    return entry


def build_na_k(nutrient, manual_meta):
    """Build Na/K from Consumer parser + manual metadata."""
    name = nutrient["name"]
    entry = {
        "name": name,
        "unit": nutrient["unit"],
        "category": manual_meta.get("category", "macromineral"),
        "source_id": "msd-consumer-minerals",
        "source_urls": SOURCE_URLS["msd-consumer-minerals"],
        "groups": [],
    }
    has_ul_meta = any(f in manual_meta for f in ("ul", "ul_unit", "ul_note"))
    for field in ("ul", "ul_unit", "ul_note", "note"):
        if field in manual_meta:
            entry[field] = manual_meta[field]
    if has_ul_meta:
        entry["metadata_source"] = "manual_transcription"

    if "note" not in entry:
        entry["note"] = "Adult values only. Source: MSD Manual Consumer. Age-specific breakdown not available in machine-readable form."

    for g in nutrient["groups"]:
        group_entry = {
            "group": g["group"],
            "sex": g.get("sex", "any"),
            "age_range": g.get("age_range", "adult"),
            "value": g["value"],
            "type": g.get("type", "AI"),
        }
        entry["groups"].append(group_entry)

    return entry


def main():
    print("Building dri-minerals-overlay.json")
    print("==================================")
    print()

    # Phase 1: load
    manual = load_json(MANUAL)
    trace = load_json(TRACE_PARSED)
    calcium = load_json(CALCIUM_PARSED)
    nak = load_json(NAK_PARSED)
    ncbi = load_json(NCBI_CROSSCHECK)

    manual_meta = extract_manual_metadata(manual)
    ncbi_entries = ncbi["ncbi_entries"]

    nutrients = []

    # Phase 2: build per nutrient

    # Trace minerals (9)
    print("Trace minerals:")
    trace_names = [n["name"] for n in trace["nutrients"]]
    for n in trace["nutrients"]:
        name = n["name"]
        meta = manual_meta.get(name, {})
        entry = build_trace_mineral(n, meta)
        nutrients.append(entry)
        print(f"  {name}: {len(entry['groups'])} groups")
    print(f"  Total: {sum(len(n['groups']) for n in nutrients)} groups")

    # Calcium
    print()
    print("Calcium:")
    ca_meta = manual_meta.get("Calcium", {})
    ca_entry = build_calcium(calcium, ca_meta)
    nutrients.append(ca_entry)
    print(f"  Calcium: {len(ca_entry['groups'])} groups, {len(ca_entry.get('ul_groups', []))} ul_groups")

    # Phosphorus and Magnesium from NCBI
    print()
    print("P and Mg from NCBI:")
    p_meta = manual_meta.get("Phosphorus", {})
    mg_meta = manual_meta.get("Magnesium", {})
    p_entry = build_p_mg("Phosphorus", ncbi_entries, p_meta)
    mg_entry = build_p_mg("Magnesium", ncbi_entries, mg_meta)
    nutrients.append(p_entry)
    nutrients.append(mg_entry)
    print(f"  Phosphorus: {len(p_entry['groups'])} groups")
    print(f"  Magnesium: {len(mg_entry['groups'])} groups")

    # Sodium and Potassium
    print()
    print("Na/K from Consumer:")
    na_meta = manual_meta.get("Sodium", {})
    k_meta = manual_meta.get("Potassium", {})
    for n in nak["nutrients"]:
        if n["name"] == "Sodium":
            entry = build_na_k(n, na_meta)
        elif n["name"] == "Potassium":
            entry = build_na_k(n, k_meta)
        else:
            continue
        nutrients.append(entry)
        print(f"  {entry['name']}: {len(entry['groups'])} groups")

    # Phase 3: write
    total_groups = sum(len(n["groups"]) for n in nutrients)
    present_groups = sum(1 for n in nutrients if len(n["groups"]) > 0)
    source_ids = sorted(set(n["source_id"] for n in nutrients))

    output = {
        "_meta": {
            "schema": "dri-minerals-overlay-v1",
            "description": "Merged DRI values for 14 minerals: machine-parsed values from IOM/NCBI/MSD sources at finest available granularity, combined with metadata from manual transcription. UL/UL_unit/UL_note — machine-verified (parsed-first) for trace minerals (MSD Professional) and Calcium (IOM 2011); manual_transcription for Phosphorus, Magnesium (NCBI, no ul in source), Potassium, Sodium (MSD Consumer, no ul in source).",
            "build_script": "data/build-minerals-overlay.py",
            "build_date": datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
            "sources": source_ids,
            "input_files": [
                "data/dri-minerals.json (metadata)",
                "data/dri-minerals-parsed.json (trace mineral values)",
                "data/dri-calcium-iom-2011-parsed.json (Ca values + ul_groups)",
                "data/dri-macrominerals-absolute-parsed.json (Na/K values)",
                "data/dri-p-mg-ncbi-crosscheck.json (P/Mg values via NCBI cross-verification)",
            ],
            "stats": {
                "nutrients": len(nutrients),
                "total_groups": total_groups,
                "granularity": "finest available per source: Ca 22 groups (IOM 2011), P/Mg 22 groups each (IOM 1997 via NCBI), trace minerals 16 groups each (MSD Professional), Na/K 2 adult groups (MSD Consumer)",
                "pregnancy_breastfeeding": "split into teen (14-18yr), adult 19-30yr, adult 31-50yr subgroups — matching IOM source granularity",
            },
        },
        "nutrients": nutrients,
    }

    with open(OVERLAY_OUT, "w") as f:
        json.dump(output, f, ensure_ascii=False, indent=2)
    print(f"\nWritten {OVERLAY_OUT}")
    print(f"  {len(nutrients)} nutrients, {total_groups} groups")
    print(f"  Sources: {', '.join(source_ids)}")

    # Quick verification: ensure every nutrient has required fields
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
