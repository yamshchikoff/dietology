# Programmatic HTML table parser for MSD Manual DRI data
# Source: MSD Manual Professional Edition, based on National Academies DRI
# Run: python3 extract-msd-dri-parser.py
#
# Reads cached HTML from data/external/ and extracts structured DRI values.
# Outputs NEW files (dri-vitamins-parsed.json, dri-minerals-parsed.json)
# for comparison with manually-transcribed existing files.

import json
import os
import re
import sys
from datetime import datetime, timezone
from html.parser import HTMLParser

SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
EXTERNAL_DIR = os.path.join(SCRIPT_DIR, "external")

VITAMINS_HTML = os.path.join(EXTERNAL_DIR, "msd-manual-vitamins-2026-05.html")
TRACE_MINERALS_HTML = os.path.join(EXTERNAL_DIR, "msd-manual-trace-minerals-2026-05.html")

VITAMINS_OUT = os.path.join(SCRIPT_DIR, "dri-vitamins-parsed.json")
MINERALS_OUT = os.path.join(SCRIPT_DIR, "dri-minerals-parsed.json")

VITAMINS_EXISTING = os.path.join(SCRIPT_DIR, "dri-vitamins.json")
MINERALS_EXISTING = os.path.join(SCRIPT_DIR, "dri-minerals.json")


# ── HTML Table Parser ──────────────────────────────────────────────

class TableParser(HTMLParser):
    """Extract <table> elements as list of rows with cell text and bold flags."""

    def __init__(self):
        super().__init__()
        self.tables = []          # list of list of (text, is_bold) tuples per row
        self.current_table = None
        self.current_row = None
        self.current_cell = None
        self.current_is_bold = False
        self.in_table = False
        self.in_row = False
        self.in_cell = False
        self.in_bold = False
        self.skip_depth = 0

    def handle_starttag(self, tag, attrs):
        if self.skip_depth > 0:
            self.skip_depth += 1
            return
        if tag in ("b", "strong"):
            self.in_bold = True
        if tag == "table":
            self.in_table = True
            self.current_table = []
        elif tag == "tr" and self.in_table:
            self.in_row = True
            self.current_row = []
        elif tag in ("td", "th") and self.in_row:
            self.in_cell = True
            self.current_cell = ""
            self.current_is_bold = False

    def handle_endtag(self, tag):
        if self.skip_depth > 0:
            if tag in ("table", "tr", "td", "th", "b", "strong"):
                self.skip_depth -= 1
            return
        if tag in ("b", "strong"):
            self.in_bold = False
        if tag == "table" and self.in_table:
            self.in_table = False
            if self.current_table and len(self.current_table) > 0:
                self.tables.append(self.current_table)
                self.current_table = None
        elif tag == "tr" and self.in_row:
            self.in_row = False
            if self.current_row and self.current_table is not None:
                self.current_table.append(self.current_row)
                self.current_row = None
        elif tag in ("td", "th") and self.in_cell:
            self.in_cell = False
            if self.current_cell is not None and self.current_row is not None:
                text = self.current_cell.strip()
                self.current_row.append((text, self.current_is_bold))
                self.current_cell = None

    def handle_data(self, data):
        if self.skip_depth > 0:
            return
        if self.in_cell and self.current_cell is not None:
            self.current_cell += data
        # Track bold state — if we see data while in_bold, mark cell as bold
        if self.in_bold and self.in_cell:
            self.current_is_bold = True


def parse_html(filepath):
    """Parse an HTML file and return list of tables (each table = list of rows)."""
    with open(filepath, encoding="utf-8") as f:
        html = f.read()
    parser = TableParser()
    parser.feed(html)
    return parser.tables


# ── Value Cleaning ─────────────────────────────────────────────────

def clean_value(text, nutrient_name=""):
    """Clean a numeric cell value. Returns float or None (for ND)."""
    if not text:
        return None

    # Normalize whitespace and nbsp
    text = re.sub(r'\s+', ' ', text).strip()

    # "ND" = not determinable
    if text.upper() == 'ND':
        return None

    # Known problematic values that are footnote artifacts:
    # Iron breastfeeding 14-18: MSD HTML shows "109" = "10" + footnote "9"
    known_fixes = {
        ("Iron", "109"): 10.0,
        ("Iron", "108"): 10.0,  # in case footnote digit varies
    }
    key = (nutrient_name, text)
    if key in known_fixes:
        return known_fixes[key]

    # Strip dagger/double-dagger/section footnote markers
    text = re.sub(r'[‡†§]', '', text).strip()

    # Remove reference brackets
    text = re.sub(r'\[\d+\]', '', text).strip()

    try:
        return float(text)
    except ValueError:
        # Try stripping trailing non-numeric chars
        m = re.match(r'([\d.]+)', text)
        if m:
            return float(m.group(1))
        return None


def extract_unit(header_text):
    """Extract unit from header like 'Folate (mcg)' → 'mcg'."""
    m = re.search(r'\(([^)]+)\)', header_text)
    if m:
        unit = m.group(1).strip()
        # Normalize common patterns
        unit = re.sub(r'\s+', ' ', unit)
        # Handle "mg NE*" → "mg NE"
        unit = re.sub(r'[*].*', '', unit)
        return unit
    return ""


def clean_nutrient_name(header_text):
    """Clean nutrient name from header like 'Vitamin\xa0\xa0C (mg)' → 'Vitamin C'."""
    # Remove unit in parentheses
    text = re.sub(r'\s*\([^)]*\)', '', header_text)
    # Normalize whitespace and nbsp
    text = re.sub(r'[\s\xa0]+', ' ', text).strip()
    # Remove trailing asterisks/daggers
    text = re.sub(r'[*†‡§]+', '', text).strip()
    return text


# ── Vitamins Parser ────────────────────────────────────────────────

VITAMIN_HEADERS = [
    "Folate", "Niacin", "Riboflavin", "Thiamin",
    "Vitamin A", "Vitamin B6", "Vitamin B12",
    "Vitamin C", "Vitamin D", "Vitamin E", "Vitamin K",
]

VITAMIN_UNITS = {
    "Folate": "mcg DFE",
    "Niacin": "mg NE",
    "Riboflavin": "mg",
    "Thiamin": "mg",
    "Vitamin A": "mcg RAE",
    "Vitamin B6": "mg",
    "Vitamin B12": "mcg",
    "Vitamin C": "mg",
    "Vitamin D": "IU",
    "Vitamin E": "mg",
    "Vitamin K": "mcg",
}

VITAMIN_UL = {
    "Folate": 1000,
    "Niacin": 35,
    "Vitamin A": 3000,
    "Vitamin B6": 100,
    "Vitamin C": 2000,
    "Vitamin D": 4000,
    "Vitamin E": 1000,
}

# Map (category, age_range) → standard group_id and sex
VITAMIN_GROUP_MAP = {
    ("infants", "0–6 months"): ("infants_0_6mo", "any"),
    ("infants", "7–12 months"): ("infants_7_12mo", "any"),
    ("children", "1–3 years"): ("children_1_3yr", "any"),
    ("children", "4–8 years"): ("children_4_8yr", "any"),
    ("males", "9–13 years"): ("male_9_13yr", "male"),
    ("males", "14–18 years"): ("male_14_18yr", "male"),
    ("males", "19–70 years"): ("male_19_70yr", "male"),
    ("males", "> 70 years"): ("male_gt70yr", "male"),
    ("females", "9–13 years"): ("female_9_13yr", "female"),
    ("females", "14–18 years"): ("female_14_18yr", "female"),
    ("females", "19–70 years"): ("female_19_70yr", "female"),
    ("females", "> 70 years"): ("female_gt70yr", "female"),
    ("pregnant women", "19–50 years"): ("pregnant_19_50yr", "female"),
    ("breastfeeding women", "19–50 years"): ("breastfeeding_19_50yr", "female"),
}


def parse_vitamins(rows):
    """Parse vitamins table rows into structured nutrients."""
    nutrients = []
    current_category = None

    for row in rows:
        # Extract text from (text, is_bold) tuples
        texts = [cell[0] for cell in row]
        bolds = [cell[1] for cell in row]

        if not texts or not texts[0]:
            continue

        first = texts[0].strip().lower()

        # Skip header row
        if first.startswith("age"):
            continue

        # Category rows (single-cell with bold)
        if first in ("infants", "children", "males", "females",
                      "pregnant women", "breastfeeding women"):
            current_category = first
            continue

        # UL row — skip (we hardcode UL from known DRI)
        if "upper limit" in first:
            continue

        # Skip footer/note rows
        if any(skip in first for skip in ("note:", "*", "†", "‡", "§",
                                            "iu =", "data from")):
            continue

        # Must have a current category
        if not current_category:
            continue

        age_range = texts[0].strip()
        group_key = (current_category, age_range)

        if group_key not in VITAMIN_GROUP_MAP:
            print(f"  WARNING: unknown vitamin group: {group_key}")
            continue

        group_id, sex = VITAMIN_GROUP_MAP[group_key]

        # Values start from column 1 (after age)
        for i, name in enumerate(VITAMIN_HEADERS):
            col_idx = i + 1
            if col_idx >= len(texts):
                continue
            val = clean_value(texts[col_idx], name)
            is_bold = bolds[col_idx] if col_idx < len(bolds) else False
            value_type = "AI" if is_bold else "RDA"

            # Infants: all values are AI (despite bold inconsistency in source)
            if current_category == "infants":
                value_type = "AI"

            # Find or create nutrient
            nutrient = None
            for n in nutrients:
                if n["name"] == name:
                    nutrient = n
                    break
            if nutrient is None:
                nutrient = {
                    "name": name,
                    "unit": VITAMIN_UNITS.get(name, ""),
                    "ul": VITAMIN_UL.get(name),
                    "ul_unit": VITAMIN_UNITS.get(name, ""),
                    "groups": [],
                }
                nutrients.append(nutrient)

            nutrient["groups"].append({
                "group": group_id,
                "sex": sex,
                "age_range": age_range,
                "value": val,
                "type": value_type,
            })

    return nutrients


# ── Trace Minerals Parser ──────────────────────────────────────────

TRACE_MINERAL_HEADERS = [
    "Chromium", "Copper", "Fluoride", "Iodine", "Iron",
    "Manganese", "Molybdenum", "Selenium", "Zinc",
]

TRACE_MINERAL_UNITS = {
    "Chromium": "mcg",
    "Copper": "mcg",
    "Fluoride": "mg",
    "Iodine": "mcg",
    "Iron": "mg",
    "Manganese": "mg",
    "Molybdenum": "mcg",
    "Selenium": "mcg",
    "Zinc": "mg",
}

TRACE_MINERAL_UL = {
    "Chromium": None,
    "Copper": 10000,
    "Fluoride": 10,
    "Iodine": 1100,
    "Iron": 45,
    "Manganese": 11,
    "Molybdenum": 2000,
    "Selenium": 400,
    "Zinc": 40,
}

# Map (category, age) → standard group_id and sex
TRACE_GROUP_MAP = {
    ("infants", "0.0–6 mo"): ("infants_0_6mo", "any"),
    ("infants", "7 mo–1 yr"): ("infants_7_12mo", "any"),
    ("children", "1–3"): ("children_1_3yr", "any"),
    ("children", "4–8"): ("children_4_8yr", "any"),
    ("males", "9–13"): ("male_9_13yr", "male"),
    ("males", "14–18"): ("male_14_18yr", "male"),
    ("males", "19–30"): ("male_19_30yr", "male"),
    ("males", "31–50"): ("male_31_50yr", "male"),
    ("males", "51+"): ("male_gt50yr", "male"),
    ("females", "9–13"): ("female_9_13yr", "female"),
    ("females", "14–18"): ("female_14_18yr", "female"),
    ("females", "19–30"): ("female_19_30yr", "female"),
    ("females", "31–50"): ("female_31_50yr", "female"),
    ("females", "51+"): ("female_gt50yr", "female"),
    ("pregnant 14–18", ""): ("pregnant_14_18yr", "female"),
    ("pregnant 19–30", ""): ("pregnant_19_30yr", "female"),
    ("pregnant 31–50", ""): ("pregnant_31_50yr", "female"),
    ("breastfeeding 14–18", ""): ("breastfeeding_14_18yr", "female"),
    ("breastfeeding 19–30", ""): ("breastfeeding_19_30yr", "female"),
    ("breastfeeding 31–50", ""): ("breastfeeding_31_50yr", "female"),
}


def parse_trace_minerals(rows):
    """Parse trace minerals table rows into structured nutrients."""
    nutrients = []
    current_category = None

    for row in rows:
        texts = [cell[0] for cell in row]
        bolds = [cell[1] for cell in row]

        if not texts or not texts[0]:
            continue

        first = texts[0].strip().lower()

        # Skip header rows
        if first.startswith("category") or "recommended daily" in first:
            continue

        # Category rows
        if first in ("infants", "children", "males", "females"):
            current_category = first
            continue

        # Pregnant/breastfeeding have combined category+age in first cell
        if first.startswith("pregnant ") or first.startswith("breastfeeding "):
            # These rows have category in col 0 and age_range is empty (merged)
            # But some rows have age in col 0 (e.g. "Pregnant 14–18")
            parts = first.split(None, 1)
            current_category = parts[0]  # "pregnant" or "breastfeeding"
            # Age is embedded in first cell: "Pregnant 14–18" → age = "14–18"
            age_range = parts[1] if len(parts) > 1 else ""
            # For these rows, values start at col 1 (col 0 is merged category+age)
            col_offset = 1
        else:
            # Normal data row — age in col 0 or col 1
            if current_category is None:
                continue
            # Check if texts[0] looks like an age range
            age_cell = texts[0].strip()
            if re.match(r'^[\d.]+', age_cell):
                age_range = age_cell
                col_offset = 1
            elif len(texts) > 1 and re.match(r'^[\d.]+', texts[1].strip()):
                age_range = texts[1].strip()
                col_offset = 2
            else:
                continue

        # Build group key
        if current_category in ("pregnant", "breastfeeding"):
            group_key = (f"{current_category} {age_range}", "")
        else:
            group_key = (current_category, age_range)

        if group_key not in TRACE_GROUP_MAP:
            print(f"  WARNING: unknown trace mineral group: {group_key}")
            continue

        group_id, sex = TRACE_GROUP_MAP[group_key]

        # Values from col_offset onward
        for i, name in enumerate(TRACE_MINERAL_HEADERS):
            col_idx = col_offset + i
            if col_idx >= len(texts):
                continue
            val = clean_value(texts[col_idx], name)
            if val is None:
                continue
            is_bold = bolds[col_idx] if col_idx < len(bolds) else False
            value_type = "AI" if is_bold else "RDA"

            # Infants: all values are AI
            if current_category == "infants":
                value_type = "AI"

            nutrient = None
            for n in nutrients:
                if n["name"] == name:
                    nutrient = n
                    break
            if nutrient is None:
                nutrient = {
                    "name": name,
                    "unit": TRACE_MINERAL_UNITS.get(name, ""),
                    "category": "trace_mineral",
                    "ul": TRACE_MINERAL_UL.get(name),
                    "ul_unit": TRACE_MINERAL_UNITS.get(name, ""),
                    "groups": [],
                }
                if TRACE_MINERAL_UL.get(name) is None:
                    nutrient["ul_note"] = "No UL established"
                nutrients.append(nutrient)

            nutrient["groups"].append({
                "group": group_id,
                "sex": sex,
                "age_range": age_range,
                "value": val,
                "type": value_type,
            })

    return nutrients


# ── Comparison ─────────────────────────────────────────────────────

def compare(parsed_file, existing_file, label):
    """Compare parsed vs existing JSON. Match groups by nutrient + age_range + sex (semantic), not group_id."""
    with open(parsed_file) as f:
        parsed = json.load(f)
    with open(existing_file) as f:
        existing = json.load(f)

    parsed_nutrients = {n["name"]: n for n in parsed.get("nutrients", [])}
    existing_nutrients = {n["name"]: n for n in existing.get("nutrients", [])}

    print(f"\n{'='*60}")
    print(f"Comparison: {label}")
    print(f"{'='*60}")

    total_parsed = sum(len(n.get("groups", [])) for n in parsed.get("nutrients", []))
    total_existing = sum(len(n.get("groups", [])) for n in existing.get("nutrients", []))
    print(f"  Parsed: {len(parsed.get('nutrients',[]))} nutrients, {total_parsed} groups")
    print(f"  Existing: {len(existing.get('nutrients',[]))} nutrients, {total_existing} groups")

    all_values_match = True
    value_mismatches = []

    for name in sorted(set(parsed_nutrients) & set(existing_nutrients)):
        pn = parsed_nutrients[name]
        en = existing_nutrients[name]

        # Build lookup by (age_range, sex) for semantic comparison
        p_by_key = {(g["age_range"], g["sex"]): g["value"] for g in pn.get("groups", [])}
        e_by_key = {(g["age_range"], g["sex"]): g["value"] for g in en.get("groups", [])}

        # Check matching keys
        common = set(p_by_key) & set(e_by_key)
        for key in sorted(common):
            if p_by_key[key] != e_by_key[key]:
                value_mismatches.append((name, key, p_by_key[key], e_by_key[key]))
                all_values_match = False

        # Keys in parsed but not in existing (different granularity — expected)
        only_parsed = set(p_by_key) - set(e_by_key)
        only_existing = set(e_by_key) - set(p_by_key)

    if all_values_match:
        print("  ALL OVERLAPPING VALUES MATCH — no numeric discrepancies.")
    else:
        print(f"  {len(value_mismatches)} VALUE MISMATCHES (same age/sex, different numbers):")
        for name, key, pv, ev in value_mismatches:
            delta = pv - ev if isinstance(pv, (int, float)) and isinstance(ev, (int, float)) else ""
            print(f"    {name} {key}: parsed={pv}, existing={ev} Δ={delta}")

    # Report structural differences (group granularity)
    only_in_parsed = set(parsed_nutrients) - set(existing_nutrients)
    only_in_existing = set(existing_nutrients) - set(parsed_nutrients)
    if only_in_parsed:
        print(f"  Only in parsed: {sorted(only_in_parsed)}")
    if only_in_existing:
        print(f"  Only in existing: {sorted(only_in_existing)}")

    # Count groups with different granularity
    granularity_diff = 0
    for name in set(parsed_nutrients) & set(existing_nutrients):
        pg = len(parsed_nutrients[name].get("groups", []))
        eg = len(existing_nutrients[name].get("groups", []))
        if pg != eg:
            granularity_diff += 1
    if granularity_diff:
        print(f"  Nutrients with different group count: {granularity_diff} (parser preserves MSD granularity)")

    return all_values_match


# ── Main ───────────────────────────────────────────────────────────

def build_meta(source_id, source_file, html_filename):
    return {
        "source_id": source_id,
        "source_file": source_file,
        "extraction_date": datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
        "extraction_script": "data/extract-msd-dri-parser.py",
        "extraction_method": "html-parser",
        "extracted_by": "agent",
        "source_claims": {
            "presumed_date": "2024",
            "presumed_author": "Merck & Co., Inc., based on National Academies of Sciences, Engineering, and Medicine DRI reports"
        },
        "source_urls": [
            f"file://{os.path.abspath(html_filename)}"
        ],
    }


def main():
    print("MSD Manual DRI HTML Parser")
    print("==========================")
    print()

    # ── Vitamins ──
    print("Parsing vitamins...")
    tables = parse_html(VITAMINS_HTML)
    # Find the substantive table (most rows with numeric content)
    target_table = None
    for t in tables:
        numeric_count = sum(
            1 for row in t for cell_text, _ in row
            if cell_text and re.match(r'^[\d.]+$', cell_text.strip())
        )
        if numeric_count > 50:
            target_table = t
            break
    if target_table is None:
        target_table = tables[0] if tables else []

    vitamins = parse_vitamins(target_table)
    total_groups = sum(len(n.get("groups", [])) for n in vitamins)
    print(f"  Extracted {len(vitamins)} nutrients, {total_groups} group entries")

    output = {
        "_meta": build_meta("msd-manual-dri",
                            "data/external/msd-manual-vitamins-2026-05.html",
                            VITAMINS_HTML),
        "nutrients": vitamins,
    }
    with open(VITAMINS_OUT, "w") as f:
        json.dump(output, f, ensure_ascii=False, indent=2)
    print(f"  Written to {VITAMINS_OUT}")

    # ── Trace Minerals ──
    print()
    print("Parsing trace minerals...")
    tables = parse_html(TRACE_MINERALS_HTML)
    # Find the substantive table (most rows)
    target_table = max(tables, key=lambda t: len(t)) if tables else []
    if not target_table and tables:
        target_table = tables[0]

    minerals = parse_trace_minerals(target_table)
    total_groups = sum(len(n.get("groups", [])) for n in minerals)
    print(f"  Extracted {len(minerals)} nutrients, {total_groups} group entries")

    output = {
        "_meta": build_meta("msd-manual-dri",
                            "data/external/msd-manual-trace-minerals-2026-05.html",
                            TRACE_MINERALS_HTML),
        "_meta_note": "Trace minerals extracted from 'Guidelines for Daily Intake of Trace Minerals' table. Bold = AI, Regular = RDA. This file contains ONLY trace minerals — Ca/P/Mg/Na/K are in dri-minerals.json (absolute) and dri-macronutrients-per-kg.json (per-kg).",
        "nutrients": minerals,
    }
    with open(MINERALS_OUT, "w") as f:
        json.dump(output, f, ensure_ascii=False, indent=2)
    print(f"  Written to {MINERALS_OUT}")

    # ── Compare with existing ──
    vitamins_ok = compare(VITAMINS_OUT, VITAMINS_EXISTING, "Vitamins")
    minerals_ok = compare(MINERALS_OUT, MINERALS_EXISTING, "Trace Minerals")

    print()
    if vitamins_ok and minerals_ok:
        print("RESULT: All parsed values match existing data.")
    else:
        print("RESULT: Discrepancies found — review above.")


if __name__ == "__main__":
    main()
