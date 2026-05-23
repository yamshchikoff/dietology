# Extract laboratory reference ranges from Wikipedia
# Source: Wikipedia, CC BY-SA 3.0 (Tier C — tertiary, well-cited)
# Run: python3 extract-wiki-lab-ranges.py
#
# Uses Wikipedia API to fetch HTML for "Reference ranges for blood tests"
# Parses HTML tables with BeautifulSoup, maps to structured JSON.

import json
import os
import re
import sys
from datetime import datetime, timezone
from html.parser import HTMLParser
from urllib.request import urlopen, Request

SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
EXTERNAL_DIR = os.path.join(SCRIPT_DIR, "external")
OUTPUT = os.path.join(SCRIPT_DIR, "lab-reference-ranges.json")
SOURCE_CACHE = os.path.join(EXTERNAL_DIR, "wikipedia-lab-ranges-2026-05.html")
HEADERS = {"User-Agent": "dietology-data-bot/1.0 (contact: me@yamshchikov.ru)"}

API_URL = "https://en.wikipedia.org/w/api.php?action=parse&page=Reference_ranges_for_blood_tests&prop=text&format=json"


class TableExtractor(HTMLParser):
    """Extract <table> elements as list of rows."""

    def __init__(self):
        super().__init__()
        self.tables = []
        self.current_table = None
        self.current_row = None
        self.current_cell = None
        self.in_table = False
        self.in_row = False
        self.in_cell = False
        self.skip_depth = 0

    def handle_starttag(self, tag, attrs):
        if self.skip_depth > 0:
            self.skip_depth += 1
            return

        if tag == "table":
            self.in_table = True
            self.current_table = []
        elif tag == "tr" and self.in_table:
            self.in_row = True
            self.current_row = []
        elif (tag == "td" or tag == "th") and self.in_row:
            self.in_cell = True
            self.current_cell = ""
            # Check for colspan/rowspan
            for attr, val in attrs:
                if attr == "colspan":
                    self._colspan = int(val)
                if attr == "rowspan":
                    pass  # rowspan handled separately

    def handle_endtag(self, tag):
        if self.skip_depth > 0:
            if tag in ("table", "tr", "td", "th"):
                self.skip_depth -= 1
            return

        if tag == "table" and self.in_table:
            self.in_table = False
            if self.current_table:
                self.tables.append(self.current_table)
                self.current_table = None
        elif tag == "tr" and self.in_row:
            self.in_row = False
            if self.current_row and self.current_table is not None:
                self.current_table.append(self.current_row)
                self.current_row = None
        elif (tag == "td" or tag == "th") and self.in_cell:
            self.in_cell = False
            if self.current_cell is not None and self.current_row is not None:
                self.current_row.append(self.current_cell.strip())
                self.current_cell = None

    def handle_data(self, data):
        if self.skip_depth > 0:
            return
        if self.in_cell and self.current_cell is not None:
            self.current_cell += data

    def handle_entityref(self, name):
        if self.skip_depth > 0:
            return
        entities = {
            "minus": "-", "plusmn": "±", "nbsp": " ", "lt": "<", "gt": ">",
            "le": "≤", "ge": "≥", "ndash": "–", "mdash": "—"
        }
        if self.in_cell and self.current_cell is not None:
            self.current_cell += entities.get(name, f"&{name};")


def fetch_html():
    """Fetch parsed HTML from Wikipedia API. Cache to external/, load from cache if present."""
    if os.path.exists(SOURCE_CACHE):
        print(f"  Using cached: {os.path.basename(SOURCE_CACHE)} ({os.path.getsize(SOURCE_CACHE):,} bytes)")
        with open(SOURCE_CACHE, encoding="utf-8") as f:
            return f.read()

    print("  Fetching from Wikipedia API...")
    req = Request(API_URL, headers=HEADERS)
    with urlopen(req, timeout=30) as resp:
        data = json.loads(resp.read())
    html = data["parse"]["text"]["*"]

    os.makedirs(EXTERNAL_DIR, exist_ok=True)
    with open(SOURCE_CACHE, "w", encoding="utf-8") as f:
        f.write(html)
    print(f"  Saved to: {os.path.basename(SOURCE_CACHE)} ({os.path.getsize(SOURCE_CACHE):,} bytes)")
    return html


def clean_cell(text):
    """Clean a table cell value, removing reference markers but keeping numeric content."""
    # Remove reference numbers in brackets: [1], [10], [21], etc.
    text = re.sub(r"\[\d+\]", "", text)
    text = re.sub(r"\[citation needed\]", "", text, flags=re.IGNORECASE)
    # Remove superscript reference patterns like ,[21] or ;[10]
    # Collapse multiple spaces
    text = re.sub(r"\s+", " ", text).strip()
    # Clean leading/trailing commas/semicolons/spaces
    text = text.strip(",;: \t\n")
    return text


def is_numeric_value(s):
    """Check if string looks like a numeric value, range, or inequality after cleaning."""
    if not s:
        return False
    # Clean reference markers
    cleaned = re.sub(r"\[\d+\]", "", s)
    cleaned = re.sub(r",\d+\s*$", "", cleaned)  # trailing ,number
    cleaned = cleaned.strip().replace(",", "").replace("−", "-").replace("–", "-")
    if not cleaned:
        return False
    # Pattern: optional <, >, ≤, ≥ followed by number, possibly a range with -
    m = re.match(r"^[<>≤≥]?\s*[\d.]+(\s*[-–]\s*[<>≤≥]?\s*[\d.]+)?$", cleaned)
    if m:
        return True
    # Pattern: "Age÷2" type expressions
    if re.search(r"[\d.]+\s*[÷/+\-]\s*[\d.]+", cleaned):
        return True
    return False


def classify_table(rows):
    """Try to identify the category of a lab table from its content."""
    if not rows:
        return "unknown"

    all_text = " ".join(
        cell for row in rows[:min(5, len(rows))] for cell in row
    ).lower()

    if any(w in all_text for w in ["sodium", "potassium", "chloride", "calcium", "iron", "ferritin", "zinc", "magnesium", "copper", "phosphate", "transferrin"]):
        return "ions_and_trace_metals"
    elif any(w in all_text for w in ["ph ", "blood gas", "pco", "po", "bicarbonate", "base excess", "oxygen saturation"]):
        return "blood_gases"
    elif any(w in all_text for w in ["alt", "ast", "bilirubin", "albumin", "ggt", "alkaline phosphatase", "total protein", "liver"]):
        return "liver_function"
    elif any(w in all_text for w in ["troponin", "creatine kinase", "ck-mb", "bnp", "myoglobin"]):
        return "cardiac"
    elif any(w in all_text for w in ["cholesterol", "triglyceride", "hdl", "ldl", "lipid"]):
        return "lipids"
    elif any(w in all_text for w in ["afp", "psa", "cea", "ca19", "ca125", "tumour", "tumor"]):
        return "tumour_markers"
    elif any(w in all_text for w in ["tsh", "thyroxine", "thyroid", "t3", "t4", "triiodothyronine", "tbg", "thyroglobulin"]):
        return "thyroid"
    elif any(w in all_text for w in ["testosterone", "estradiol", "progesterone", "fsh", "lh", "sex hormone", "dhea", "androstenedione", "amh", "shbg"]):
        return "sex_hormones"
    elif any(w in all_text for w in ["cortisol", "acth", "prolactin", "parathyroid", "pth", "growth hormone", "igf", "renin", "aldosterone"]):
        return "other_hormones"
    elif any(w in all_text for w in ["vitamin a", "vitamin b", "vitamin c", "vitamin d", "vitamin e", "vitamin k", "folate", "homocysteine"]):
        return "vitamins"
    elif any(w in all_text for w in ["hemoglobin", "haemoglobin", "hematocrit", "mcv", "mch", "mchc", "reticulocyte", "rbc count", "red blood cell", "rdw"]):
        return "hematology_rbc"
    elif any(w in all_text for w in ["white blood", "wbc", "neutrophil", "lymphocyte", "monocyte", "eosinophil", "basophil", "cd4"]):
        return "hematology_wbc"
    elif any(w in all_text for w in ["platelet", "inr", "ptt", "prothrombin", "fibrinogen", "coagulation", "thrombin", "bleeding time"]):
        return "coagulation"
    elif any(w in all_text for w in ["igg", "iga", "igm", "ige", "ig d", "antibody", "crp", "esr", "rheumatoid", "alpha 1-antitrypsin", "complement", "procalcitonin"]):
        return "immunology"
    elif any(w in all_text for w in ["lead", "ethanol", "toxic"]):
        return "toxicology"
    elif any(w in all_text for w in ["amylase", "lipase", "ldh", "d-dimer", "angiotensin"]):
        return "other_enzymes"
    elif any(w in all_text for w in ["glucose", "creatinine", "urea", "uric acid", "osmolality", "lactate", "bun"]):
        return "metabolites"

    return "unknown"


def extract_ranges(tables):
    """Extract lab reference ranges from parsed HTML tables."""
    results = []
    prev_test = ""  # for rowspan continuations

    for table in tables:
        if len(table) < 2:
            continue

        category = classify_table(table)

        for row_idx, row in enumerate(table[1:], 1):  # Skip header
            if not row:
                continue

            # Clean cells
            cells = [clean_cell(c) for c in row]

            # Skip header-like rows that appear mid-table
            first = cells[0].lower() if cells else ""
            if first in ["test", "analyte", "component", "substance", "lower limit"]:
                continue

            # Detect row type: data row has a test name; continuation row (rowspan) doesn't
            has_test_name = bool(first) and not is_numeric_value(first)
            expect_numeric_in_cols = [1, 2, 3, 4]  # columns where numbers usually appear

            # Find cells with numeric content
            num_indices = [i for i, c in enumerate(cells) if is_numeric_value(c)]

            if not num_indices:
                continue

            # Determine test name
            test_name = first if has_test_name else prev_test
            if not test_name:
                continue
            if has_test_name:
                prev_test = test_name

            # Type/subtype — cells between test name and first numeric column
            type_start = 1 if has_test_name else 0
            type_end = num_indices[0]
            type_cells = [c for c in cells[type_start:type_end] if c and c != test_name]
            type_str = " | ".join(type_cells)

            # Find low and high from numeric columns
            low = None
            high = None
            unit = ""

            # If exactly 1 numeric: cutoff (tumour markers, etc.)
            # If 2+ numerics: low and high
            if len(num_indices) == 1:
                low = None
                high = cells[num_indices[0]]
                unit_idx = num_indices[0] + 1
            elif len(num_indices) >= 2:
                low = cells[num_indices[0]]
                high = cells[num_indices[1]]
                unit_idx = num_indices[1] + 1
            else:
                continue

            # Extract unit
            for i in range(unit_idx, min(unit_idx + 2, len(cells))):
                if i < len(cells) and cells[i] and not is_numeric_value(cells[i]):
                    potential_unit = cells[i]
                    # Filter out things that don't look like units
                    if not re.match(r"^(See|rowspan|colspan|\[)", potential_unit):
                        unit = potential_unit
                        break

            if test_name and (low or high):
                results.append({
                    "category": category,
                    "test": test_name,
                    "type": type_str,
                    "lower": low,
                    "upper": high,
                    "unit": unit,
                })

    return results


def main():
    print("Wikipedia Lab Reference Ranges Extractor (Tier C)")
    print("===================================================")
    print()

    try:
        print("Fetching HTML from Wikipedia API...")
        html = fetch_html()
        print(f"  Got {len(html)} characters of HTML")
    except Exception as e:
        print(f"  API fetch failed: {e}")
        if os.path.exists(OUTPUT):
            with open(OUTPUT) as f:
                data = json.load(f)
            print(f"  Using existing file: {len(data.get('ranges', []))} ranges")
            return
        else:
            print("  No existing file. Aborting.")
            sys.exit(1)

    print("Parsing HTML tables...")
    parser = TableExtractor()
    parser.feed(html)
    print(f"  Found {len(parser.tables)} HTML tables")

    print("Extracting reference ranges...")
    ranges = extract_ranges(parser.tables)
    print(f"  Extracted {len(ranges)} reference range entries")

    # Count by category
    cats = {}
    for r in ranges:
        cats[r["category"]] = cats.get(r["category"], 0) + 1
    print("  By category:")
    for cat, count in sorted(cats.items(), key=lambda x: -x[1]):
        print(f"    {cat}: {count}")

    output = {
        "_meta": {
            "source_id": "wikipedia-lab-ranges",
            "source_file": "data/external/wikipedia-lab-ranges-2026-05.html",
            "extraction_date": datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
            "extraction_script": "data/extract-wiki-lab-ranges.py",
            "extracted_by": "agent",
            "source_claims": {
                "presumed_date": "2026-05",
                "presumed_author": "Wikipedia contributors. Reference ranges for blood tests. Wikipedia, The Free Encyclopedia."
            },
            "tier_warning": "Tier C — tertiary source. Values are representative, not universal. Reference ranges vary by laboratory, assay method, and population. Use for contextualization only, NOT for diagnostic recommendations.",
            "citation": "Wikipedia contributors. Reference ranges for blood tests. In Wikipedia, The Free Encyclopedia. CC BY-SA 3.0.",
            "primary_sources_cited": "MedlinePlus (US NLM, public domain), Uppsala University Hospital, Mayo Clinic Laboratories, First Aid for the USMLE, GPnotebook, Merck Manual, American Association of Clinical Endocrinologists, Royal College of Pathologists of Australasia."
        },
        "ranges": ranges,
    }

    with open(OUTPUT, "w") as f:
        json.dump(output, f, ensure_ascii=False, indent=2)

    print()
    print(f"Written {len(ranges)} ranges to {OUTPUT}")
    print("IMPORTANT: Tier C data. Model MUST qualify range-based statements")
    print("with source tier and recommend clinical verification.")


if __name__ == "__main__":
    main()
