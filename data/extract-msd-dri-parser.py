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

CONSUMER_MINERALS_HTML = os.path.join(EXTERNAL_DIR, "msd-manual-consumer-minerals-2026-05.html")
MACRONUTRIENTS_HTML = os.path.join(EXTERNAL_DIR, "msd-manual-macronutrients-2026-05.html")
MACROMINERALS_ABSOLUTE_OUT = os.path.join(SCRIPT_DIR, "dri-macrominerals-absolute-parsed.json")
MACRONUTRIENTS_PER_KG_OUT = os.path.join(SCRIPT_DIR, "dri-macronutrients-per-kg-parsed.json")

VITAMINS_EXISTING = os.path.join(SCRIPT_DIR, "dri-vitamins.json")
MINERALS_EXISTING = os.path.join(SCRIPT_DIR, "dri-minerals.json")

# Real MSD Manual URLs for each source HTML file
SOURCE_URLS = {
    "msd-manual-vitamins-2026-05.html":
        "https://www.msdmanuals.com/professional/multimedia/table/recommended-daily-intakes-for-vitamins",
    "msd-manual-trace-minerals-2026-05.html":
        "https://www.msdmanuals.com/professional/multimedia/table/guidelines-for-daily-intake-of-trace-minerals",
    "msd-manual-consumer-minerals-2026-05.html":
        "https://www.msdmanuals.com/home/disorders-of-nutrition/minerals/overview-of-minerals",
    "msd-manual-macronutrients-2026-05.html":
        "https://www.msdmanuals.com/professional/multimedia/table/recommended-dietary-reference-intakes-for-some-macronutrients-food-and-nutrition-board-institute-of-medicine-of-the-national-academies",
}


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


def extract_footnote(filepath):
    """Extract footnote/note text from after the last </table> in an HTML file."""
    with open(filepath, encoding="utf-8") as f:
        html = f.read()
    table_end = html.rfind("</table>")
    if table_end < 0:
        return ""
    after = html[table_end + len("</table>"):]
    # Strip HTML tags
    text = re.sub(r"<[^>]+>", " ", after)
    # Collapse whitespace
    text = re.sub(r"\s+", " ", text).strip()
    # Truncate at first obvious structural boundary
    cutoffs = []
    for marker in ["View sub-sections", "Drugs Mentioned",
                   "All rights reserved", "Was This Helpful",
                   "Copyright", "Merck & Co"]:
        idx = text.find(marker)
        if idx >= 0:
            cutoffs.append(idx)
    if cutoffs:
        text = text[:min(cutoffs)].strip()
    return text


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

VITAMIN_UL_NOTES = {
    "Folate": "Applies to synthetic folic acid from supplements and fortified foods",
    "Riboflavin": "ND — not determinable",
    "Thiamin": "ND — not determinable",
    "Vitamin B12": "ND — not determinable",
    "Vitamin K": "ND — not determinable",
}

VITAMIN_UNIT_NOTES = {
    "Niacin": "1 NE = 1 mg niacin or 60 mg dietary tryptophan",
    "Vitamin D": "200 IU = 5 mcg cholecalciferol",
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
                if VITAMIN_UL_NOTES.get(name):
                    nutrient["ul_note"] = VITAMIN_UL_NOTES[name]
                if VITAMIN_UNIT_NOTES.get(name):
                    nutrient["unit_note"] = VITAMIN_UNIT_NOTES[name]
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


# ── Consumer Minerals Parser (Na/K Adult AI) ────────────────────────

# Minerals to extract from Consumer table — only Na/K (Ca/P/Mg from IOM PDFs)
CONSUMER_TARGET_MINERALS = {
    "sodium": "Sodium",
    "potassium": "Potassium",
}

CONSUMER_UNITS = {
    "Sodium": "mg",
    "Potassium": "mg",
}


def parse_rdai_text(text):
    """Parse narrative RDA/AI text into (value_mg, sex_or_None) tuples.

    Handles: '1,500 milligrams', '3.4 grams for men 2.6 grams for women',
    '320 milligrams for women 420 milligrams for men'.
    """
    results = []
    # Remove commas from numbers like "1,500"
    text = re.sub(r'(\d),(\d)', r'\1\2', text)

    # Find number + unit + optional sex specifier
    # Use lookahead for sex boundary — some HTML has no space between sex and next value
    pattern = r'([\d.]+)\s*(grams?|milligrams?|micrograms?)\s*(?:for\s+(men|women|males?|females?)(?=\s|\d|$))?'

    for match in re.finditer(pattern, text, re.IGNORECASE):
        value = float(match.group(1))
        unit = match.group(2).lower()
        sex_word = match.group(3)

        # Convert to mg
        if unit.startswith('gram'):
            value *= 1000
        elif unit.startswith('microgram'):
            value /= 1000

        if sex_word and sex_word[0].lower() in ('m', 'b'):
            sex = 'male'
        elif sex_word and sex_word[0].lower() in ('w', 'f', 'g'):
            sex = 'female'
        else:
            sex = None

        results.append((value, sex))

    return results


def parse_consumer_minerals(rows):
    """Parse Consumer minerals table rows for Na and K adult AI values."""
    nutrients = []

    for row in rows:
        texts = [cell[0] for cell in row]
        if not texts or len(texts) < 4:
            continue

        mineral_name = texts[0].strip().lower()

        if mineral_name not in CONSUMER_TARGET_MINERALS:
            continue

        display_name = CONSUMER_TARGET_MINERALS[mineral_name]
        rdai_text = texts[3].strip()
        parsed = parse_rdai_text(rdai_text)

        groups = []
        for value, sex in parsed:
            value = int(value) if value == int(value) else value
            if sex is None:
                groups.append({
                    "group": "adult_male",
                    "sex": "male",
                    "age_range": "adult",
                    "value": value,
                    "type": "AI",
                })
                groups.append({
                    "group": "adult_female",
                    "sex": "female",
                    "age_range": "adult",
                    "value": value,
                    "type": "AI",
                })
            else:
                groups.append({
                    "group": f"adult_{sex}",
                    "sex": sex,
                    "age_range": "adult",
                    "value": value,
                    "type": "AI",
                })

        nutrients.append({
            "name": display_name,
            "unit": CONSUMER_UNITS[display_name],
            "category": "macromineral",
            "groups": groups,
        })

    return nutrients


# ── Macronutrients Per-kg Parser (Ca/P/Mg) ──────────────────────────

MACRONUTRIENT_HEADERS = ["Calcium", "Phosphorus", "Magnesium"]
MACRONUTRIENT_COL_OFFSET = 4  # Column 4 = first nutrient

# Map (category, age_html) → (group_id, sex, type)
MACRONUTRIENT_GROUP_MAP = {
    ("Infants", "0.0–0.5"): ("infants_0_0.5yr", "any", "AI"),
    ("Infants", "0.5–1.0"): ("infants_0.5_1yr", "any", "AI"),
    ("Children", "1–3"): ("children_1_3yr", "any", "RDA"),
    ("Children", "4–6"): ("children_4_6yr", "any", "RDA"),
    ("Children", "7–10"): ("children_7_10yr", "any", "RDA"),
    ("Males", "11–14"): ("male_11_14yr", "male", "RDA"),
    ("Males", "15–18"): ("male_15_18yr", "male", "RDA"),
    ("Males", "19–24"): ("male_19_24yr", "male", "RDA"),
    ("Males", "25–50"): ("male_25_50yr", "male", "RDA"),
    ("Males", "51+"): ("male_51plus_yr", "male", "RDA"),
    ("Females", "11–14"): ("female_11_14yr", "female", "RDA"),
    ("Females", "15–18"): ("female_15_18yr", "female", "RDA"),
    ("Females", "19–24"): ("female_19_24yr", "female", "RDA"),
    ("Females", "25–50"): ("female_25_50yr", "female", "RDA"),
    ("Females", "51+"): ("female_51plus_yr", "female", "RDA"),
    ("Pregnant", "—"): ("pregnant", "female", "RDA"),
    ("Breastfeeding", "1st year"): ("breastfeeding", "female", "RDA"),
}


def parse_macronutrients_per_kg(rows):
    """Parse macronutrients per-kg table rows for Ca/P/Mg values."""
    nutrients = []
    current_category = None

    for row in rows:
        texts = [cell[0] for cell in row]
        if not texts or len(texts) < 2:
            continue

        first = texts[0].strip()
        age = texts[1].strip() if len(texts) > 1 else ""

        # Skip header and footnote rows
        if first.lower().startswith(("category", "*", "data from")):
            continue

        # Update current_category when first cell names a category
        if first in ("Infants", "Children", "Males", "Females",
                      "Pregnant", "Breastfeeding"):
            current_category = first
            # Fall through — this row may also carry data (age col)

        if not current_category or not age:
            continue

        group_key = (current_category, age)
        if group_key not in MACRONUTRIENT_GROUP_MAP:
            print(f"  WARNING: unknown macronutrient group: {group_key}")
            continue

        group_id, sex, value_type = MACRONUTRIENT_GROUP_MAP[group_key]

        for i, name in enumerate(MACRONUTRIENT_HEADERS):
            col_idx = MACRONUTRIENT_COL_OFFSET + i
            if col_idx >= len(texts):
                continue
            val = clean_value(texts[col_idx], name)
            if val is None:
                continue

            nutrient = None
            for n in nutrients:
                if n["name"] == name:
                    nutrient = n
                    break
            if nutrient is None:
                nutrient = {
                    "name": name,
                    "unit": "mg/kg",
                    "groups": [],
                }
                nutrients.append(nutrient)

            nutrient["groups"].append({
                "group": group_id,
                "sex": sex,
                "age_range": age,
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

def build_meta(source_id, source_file, html_filename, source_url=None, note=None):
    if source_url is None:
        html_basename = os.path.basename(html_filename)
        source_url = SOURCE_URLS.get(html_basename, f"file://{os.path.abspath(html_filename)}")
    meta = {
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
        "source_urls": [source_url],
    }
    if note:
        meta["note"] = note
    return meta


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

    vitamins_footnote = extract_footnote(VITAMINS_HTML)
    output = {
        "_meta": build_meta("msd-manual-dri",
                            "data/external/msd-manual-vitamins-2026-05.html",
                            VITAMINS_HTML),
        "_meta_note": f"Bold values = AI, regular type = RDA. UL = Tolerable Upper Intake Level. ND = Not Determinable. Source table footnote: {vitamins_footnote}",
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

    minerals_footnote = extract_footnote(TRACE_MINERALS_HTML)
    output = {
        "_meta": build_meta("msd-manual-dri",
                            "data/external/msd-manual-trace-minerals-2026-05.html",
                            TRACE_MINERALS_HTML),
        "_meta_note": f"Trace minerals extracted from 'Guidelines for Daily Intake of Trace Minerals' table. Bold = AI, Regular = RDA. This file contains ONLY trace minerals — Ca/P/Mg/Na/K are in dri-minerals.json (absolute) and dri-macronutrients-per-kg.json (per-kg). Source table footnote: {minerals_footnote}",
        "nutrients": minerals,
    }
    with open(MINERALS_OUT, "w") as f:
        json.dump(output, f, ensure_ascii=False, indent=2)
    print(f"  Written to {MINERALS_OUT}")

    # ── Consumer Minerals (Na/K Adult AI) ──
    print()
    print("Parsing Consumer minerals table (Na/K adult AI)...")
    tables = parse_html(CONSUMER_MINERALS_HTML)
    target_table = tables[0] if tables else []

    consumer_minerals = parse_consumer_minerals(target_table)
    total_groups = sum(len(n.get("groups", [])) for n in consumer_minerals)
    print(f"  Extracted {len(consumer_minerals)} nutrients, {total_groups} group entries")

    output = {
        "_meta": build_meta("msd-consumer-minerals",
                            "data/external/msd-manual-consumer-minerals-2026-05.html",
                            CONSUMER_MINERALS_HTML),
        "_meta_note": "Na and K adult AI values extracted from MSD Manual Consumer 'Overview of Minerals' table. Consumer version contains only adult RDA/AI values — no pediatric or adolescent age breakdown. Na: 1500 mg AI for all adults. K: 3400 mg AI for men, 2600 mg AI for women. Both are AI (Adequate Intake) — no RDA established for these nutrients.",
        "nutrients": consumer_minerals,
    }
    with open(MACROMINERALS_ABSOLUTE_OUT, "w") as f:
        json.dump(output, f, ensure_ascii=False, indent=2)
    print(f"  Written to {MACROMINERALS_ABSOLUTE_OUT}")

    # ── Macronutrients Per-kg (Ca/P/Mg) ──
    print()
    print("Parsing Macronutrients per-kg table (Ca/P/Mg)...")
    tables = parse_html(MACRONUTRIENTS_HTML)
    target_table = tables[0] if tables else []

    macro_nutrients = parse_macronutrients_per_kg(target_table)
    total_groups = sum(len(n.get("groups", [])) for n in macro_nutrients)
    print(f"  Extracted {len(macro_nutrients)} nutrients, {total_groups} group entries")

    output = {
        "_meta": build_meta("msd-macronutrients-per-kg",
                            "data/external/msd-manual-macronutrients-2026-05.html",
                            MACRONUTRIENTS_HTML),
        "_meta_note": "Ca/P/Mg per-kg values extracted from MSD Manual 'Recommended Dietary Reference Intakes for Some Macronutrients' table. Source: Institute of Medicine 1997. All values in mg/kg of body weight. Infants: AI. Children and adults: RDA. Model must multiply by individual body weight.",
        "nutrients": macro_nutrients,
    }
    with open(MACRONUTRIENTS_PER_KG_OUT, "w") as f:
        json.dump(output, f, ensure_ascii=False, indent=2)
    print(f"  Written to {MACRONUTRIENTS_PER_KG_OUT}")

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
