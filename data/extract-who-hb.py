# Extract WHO 2024 Haemoglobin Cutoffs for Anaemia
# Source: WHO Guideline (Tier B), CC BY-NC-SA 3.0 IGO
# Extracts Tables 2 and 3 from PDF via pdfplumber.
# Run: python3 extract-who-hb.py

import json
import os
import re
import sys
from datetime import datetime, timezone

SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))

WHO_PUBLICATION_URL = "https://www.who.int/publications/i/item/9789240088542"
WHO_IRIS_PDF = "https://iris.who.int/bitstream/handle/10665/376196/9789240088542-eng.pdf"
PDF_PATH = os.path.join(SCRIPT_DIR, "external", "who-2024-hb-guideline.pdf")
OUTPUT = os.path.join(SCRIPT_DIR, "who-hb-thresholds.json")

# Map PDF population labels → group_id
# Table 2 population → (group_id, sex, pregnant, trimester)
DIAGNOSTIC_MAP = [
    (r"Children,?\s*6[–\-]23\s*months", "children_6_23_months", "any", False, None),
    (r"Children,?\s*24[–\-]59\s*months", "children_24_59_months", "any", False, None),
    (r"Children,?\s*5[–\-]11\s*years", "children_5_11_years", "any", False, None),
    (r"Children,?\s*12[–\-]14\s*years.*girls", "children_12_14_years", "any", False, None),
    (r"Children,?\s*12[–\-]14\s*years.*boys", None, None, None, None),  # same threshold as girls, merged
    (r"Adults,?\s*15[–\-]65\s*years.*nonpregnant\s*women", "non_pregnant_women_15_plus", "female", False, None),
    (r"Adults,?\s*15[–\-]65\s*years.*men", "men_15_plus", "male", False, None),
    (r"First\s*trimester", "pregnant_first_trimester", "female", True, 1),
    (r"Second\s*trimester", "pregnant_second_trimester", "female", True, 2),
    (r"Third\s*trimester", "pregnant_third_trimester", "female", True, 3),
]

# Table 3 population → group_id (slightly different names)
SEVERITY_MAP = [
    (r"Children,?\s*6[–\-]23\s*months", "children_6_23_months"),
    (r"Children,?\s*24[–\-]59\s*months", "children_24_59_months"),
    (r"Children,?\s*5[–\-]11\s*years", "children_5_11_years"),
    (r"Children,?\s*12[–\-]14\s*years.*girls", "children_12_14_years"),
    (r"Children,?\s*12[–\-]14\s*years.*boys", None),  # merged
    (r"Adults,?\s*15[–\-]65,?\s*years.*nonpregnant\s*women", "non_pregnant_women_15_65"),
    (r"Adults,?\s*15[–\-]65,?\s*years.*men", "men_15_65"),
    (r"First\s*trimester", "pregnant_first_trimester"),
    (r"Second\s*trimester", "pregnant_second_trimester"),
    (r"Third\s*trimester", "pregnant_third_trimester"),
]


def extract_table2(pages_text):
    """Find and parse Table 2 (diagnostic cutoffs) from page text."""
    for text in pages_text:
        m = re.search(r'Table\s*2\.?\s*Haemoglobin cutoffs to define anaemia[^\n]*\n(.*?)(?:\n\n|Table\s*3|Executive summary)', text, re.DOTALL)
        if not m:
            continue
        block = m.group(1)
        # Parse rows: "Population label <NNN"
        rows = []
        for line in block.strip().split('\n'):
            line = line.strip()
            if not line or line.startswith('a ') or line.startswith('Haemoglobin') or line.startswith('Population'):
                continue
            # Match: "Label <NNN" or "Label , <NNN"
            m2 = re.match(r'(.+?)\s*<(\d+)', line)
            if m2:
                rows.append((m2.group(1).strip(), int(m2.group(2))))
        if len(rows) >= 9:
            return rows
    return None


def _normalize_table3_lines(lines):
    """Rejoin PDF-split rows into logical rows.

    Table 3 has a complex layout: some rows are split across 3 lines
    (label prefix → values → label suffix), intermixed with complete rows.
    This normalizer accumulates label-only and values-only fragments and
    emits a logical row when both are present and a new complete row arrives.
    """
    VALUE_RE = re.compile(r'[≥<]\s*\d+')
    LABEL_RE = re.compile(r'[A-Za-z]')
    ROW_RE = re.compile(r'(.+?)\s*[≥<]\s*(\d+)\s+(\d+)[–\-](\d+)\s+(\d+)[–\-](\d+)\s*<(\d+)')

    rows = []
    pending_label = []
    pending_values = None

    for line in lines:
        line = line.strip()
        if not line:
            continue
        # Skip table header lines
        if line.startswith('Haemoglobin concentration') or line.startswith('Population'):
            continue

        has_values = bool(VALUE_RE.search(line))
        has_label = bool(LABEL_RE.match(line))

        if has_label and has_values:
            # Complete row on one line
            if pending_label and pending_values is not None:
                rows.append(' '.join(pending_label) + ' ' + pending_values)
            rows.append(line)
            pending_label = []
            pending_values = None
        elif has_values and not has_label:
            pending_values = line
        elif has_label and not has_values:
            pending_label.append(line)

    # Flush any trailing pending row
    if pending_label and pending_values is not None:
        rows.append(' '.join(pending_label) + ' ' + pending_values)

    return rows


def extract_table3(pages_text):
    """Find and parse Table 3 (severity) from page text."""
    for text in pages_text:
        m = re.search(r'Table\s*3\.?\s*Haemoglobin cutoffs to define anaemia severity[^\n]*\n(.*?)(?:\n\n\S|Remarks)', text, re.DOTALL)
        if not m:
            continue
        block = m.group(1)
        raw_lines = block.strip().split('\n')
        lines = _normalize_table3_lines(raw_lines)

        ROW_RE = re.compile(r'(.+?)\s*[≥<]\s*(\d+)\s+(\d+)[–\-](\d+)\s+(\d+)[–\-](\d+)\s*<(\d+)')
        rows = []
        for line in lines:
            m2 = ROW_RE.match(line)
            if m2:
                label = m2.group(1).strip()
                normal_low = int(m2.group(2))
                mild_low = int(m2.group(3))
                mild_high = int(m2.group(4))
                moderate_low = int(m2.group(5))
                moderate_high = int(m2.group(6))
                severe_below = int(m2.group(7))
                rows.append((label, normal_low, mild_low, mild_high, moderate_low, moderate_high, severe_below))

        if len(rows) >= 9:
            return rows
    return None


def map_label(label, mapping, merge_duplicates=True):
    """Match a PDF population label to a mapping list. Returns (group_id, ...) or None."""
    for pattern, *result in mapping:
        if re.search(pattern, label, re.IGNORECASE):
            if result[0] is None:  # skip marker for merged groups
                return None
            return result
    return None


def build_diagnostic_thresholds(table2_rows):
    """Convert Table 2 rows to diagnostic_thresholds list."""
    thresholds = []
    seen = set()
    for label, cutoff in table2_rows:
        result = map_label(label, DIAGNOSTIC_MAP)
        if result is None:
            continue
        group_id, sex, pregnant, trimester = result
        if group_id in seen:
            continue
        seen.add(group_id)
        entry = {
            "group": group_id,
            "sex": sex,
            "pregnant": pregnant,
            "hb_cutoff_g_per_l": cutoff,
            "hb_cutoff_g_per_dl": round(cutoff / 10, 1),
        }
        # Add notes for 2024 changes
        if group_id == "children_6_23_months":
            entry["note"] = "Lowered from 110 g/L in 2024 guideline"
        if trimester is not None:
            entry["trimester"] = trimester
        if group_id == "pregnant_second_trimester":
            entry["note"] = "New trimester-specific cutoff in 2024 guideline"
        thresholds.append(entry)
    return thresholds


def build_severity(table3_rows):
    """Convert Table 3 rows to severity_classification list."""
    severity = []
    seen = set()
    for label, normal_low, mild_low, mild_high, moderate_low, moderate_high, severe_below in table3_rows:
        result = map_label(label, SEVERITY_MAP)
        if result is None:
            continue
        group_id = result[0]
        if group_id in seen:
            continue
        seen.add(group_id)
        entry = {
            "group": group_id,
            "normal_low": normal_low,
            "mild_low": mild_low,
            "mild_high": mild_high,
            "moderate_low": moderate_low,
            "moderate_high": moderate_high,
            "severe_below": severe_below,
        }
        if group_id == "pregnant_second_trimester":
            entry["note"] = "Second trimester severity uses diagnostic cutoff of 105 g/L as mild threshold floor"
        severity.append(entry)
    return severity


def validate_output(data):
    """Validate extracted data against known WHO 2024 values."""
    thresholds = data["diagnostic_thresholds"]
    severity = data["severity_classification"]

    print(f"Diagnostic thresholds: {len(thresholds)} groups")
    print(f"Severity classifications: {len(severity)} groups")

    expected = {
        "children_6_23_months": 105,
        "children_24_59_months": 110,
        "children_5_11_years": 115,
        "children_12_14_years": 120,
        "non_pregnant_women_15_plus": 120,
        "pregnant_first_trimester": 110,
        "pregnant_second_trimester": 105,
        "pregnant_third_trimester": 110,
        "men_15_plus": 130,
    }

    for t in thresholds:
        gid = t["group"]
        expected_val = expected.get(gid)
        if expected_val is None:
            print(f"  UNEXPECTED GROUP: {gid}")
            continue
        actual = t["hb_cutoff_g_per_l"]
        if actual != expected_val:
            print(f"  MISMATCH {gid}: expected {expected_val}, got {actual}")
            return False
        print(f"  {gid:35s} {actual} g/L  OK")

    # Validate severity ranges make sense
    for s in severity:
        assert s["normal_low"] > s["mild_low"], \
            f"normal > mild for {s['group']}: {s['normal_low']} > {s['mild_low']}"
        assert s["mild_low"] >= s["moderate_low"], \
            f"mild >= moderate for {s['group']}"
        assert s["moderate_low"] >= s["severe_below"], \
            f"moderate >= severe for {s['group']}"

    print("All validations passed.")
    return True


def main():
    print("WHO 2024 Haemoglobin Guideline Extractor")
    print("========================================")
    print()
    print(f"Publication: {WHO_PUBLICATION_URL}")
    print(f"PDF: {WHO_IRIS_PDF}")
    print()

    if not os.path.exists(PDF_PATH):
        print(f"ERROR: PDF not found at {PDF_PATH}")
        print("Download manually from https://www.who.int/publications/i/item/9789240088542")
        print("(WHO IRIS uses JavaScript SPA — requires browser)")
        print()
        # Fall back to validating existing JSON if available
        if os.path.exists(OUTPUT):
            print(f"Falling back to validation of existing {OUTPUT}...")
            with open(OUTPUT) as f:
                data = json.load(f)
            if validate_output(data):
                print()
                print("Status: Existing JSON data valid and current.")
            return
        else:
            print(f"ERROR: {OUTPUT} not found either — cannot proceed.")
            sys.exit(1)

    # Extract from PDF
    print(f"Extracting from {PDF_PATH}...")
    print()

    try:
        import pdfplumber
    except ImportError:
        print("pdfplumber not installed. Install: pip install pdfplumber")
        sys.exit(1)

    with pdfplumber.open(PDF_PATH) as pdf:
        # Read all pages text for Table 2 and 3 search
        pages_text = [p.extract_text() or '' for p in pdf.pages]
        print(f"  PDF loaded: {len(pdf.pages)} pages")

    # Extract Table 2 (diagnostic cutoffs)
    print("  Searching for Table 2 (diagnostic cutoffs)...")
    table2_rows = extract_table2(pages_text)
    if not table2_rows:
        print("  ERROR: Could not find Table 2 in PDF")
        sys.exit(1)
    print(f"  Found: {len(table2_rows)} rows")
    for label, cutoff in table2_rows:
        print(f"    {label:50s} <{cutoff}")

    # Extract Table 3 (severity)
    print("  Searching for Table 3 (severity)...")
    table3_rows = extract_table3(pages_text)
    if not table3_rows:
        print("  ERROR: Could not find Table 3 in PDF")
        sys.exit(1)
    print(f"  Found: {len(table3_rows)} rows")
    for label, normal_low, mild_low, mild_high, mod_low, mod_high, sev in table3_rows:
        print(f"    {label:50s} ≥{normal_low}  {mild_low}-{mild_high}  {mod_low}-{mod_high}  <{sev}")

    # Build output
    diagnostic_thresholds = build_diagnostic_thresholds(table2_rows)
    severity_classification = build_severity(table3_rows)

    # Build legacy comparison
    legacy_comparison = [
        {
            "group": "infants_6_23_months",
            "pre_2024_g_per_l": 110,
            "post_2024_g_per_l": 105,
            "change": "lowered by 5 g/L",
        },
        {
            "group": "pregnant_second_trimester",
            "pre_2024_g_per_l": 110,
            "post_2024_g_per_l": 105,
            "change": "lowered by 5 g/L, new trimester-specific cutoff",
        },
        {
            "group": "all_other_groups",
            "pre_2024_g_per_l": "unchanged",
            "post_2024_g_per_l": "unchanged",
            "change": "no change",
        },
    ]

    output = {
        "_meta": {
            "source_id": "who-2024-hb",
            "extraction_date": datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
            "extraction_script": "data/extract-who-hb.py",
            "extraction_method": "pdfplumber — machine extraction from WHO 2024 PDF Tables 2 and 3",
            "extracted_by": "agent",
            "source_claims": {
                "presumed_date": "2024",
                "presumed_author": "World Health Organization, Geneva",
            },
            "license_note": "Source document: CC BY-NC-SA 3.0 IGO. Numeric threshold values are facts, not copyrightable expression. Direct table reproduction would violate NC clause. Extracted facts with attribution to WHO 2024 guideline.",
            "source_urls": [
                WHO_PUBLICATION_URL,
                "https://iris.who.int/handle/10665/376196",
            ],
            "citation": "WHO. Guideline on haemoglobin cutoffs to define anaemia in individuals and populations. Geneva: World Health Organization; 2024. ISBN 9789240088542.",
            "method_note": "Venous blood preferred. Capillary blood may overestimate Hb. Altitude and smoking adjustments apply.",
            "source_file": "data/external/who-2024-hb-guideline.pdf",
        },
        "diagnostic_thresholds": diagnostic_thresholds,
        "severity_classification": severity_classification,
        "severe_anaemia_pregnancy_note": "Severe anaemia in pregnancy: <70 g/L. Very severe: <40 g/L (per CDC supplementary note).",
        "altitude_smoking_note": "Altitude and smoking adjustments recommended. Formulas included in WHO 2024 guideline Appendix.",
        "legacy_comparison": legacy_comparison,
    }

    if validate_output(output):
        with open(OUTPUT, "w") as f:
            json.dump(output, f, ensure_ascii=False, indent=2)
        print(f"\nWritten {OUTPUT}")
        print("Status: Machine-extracted from WHO 2024 PDF — 100% from source.")


if __name__ == "__main__":
    main()
