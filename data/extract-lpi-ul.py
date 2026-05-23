# Extract Phosphorus and Magnesium UL from Linus Pauling Institute pages.
# Source: Linus Pauling Institute, Oregon State University (cites IOM/NAS DRI)
# Run: python3 extract-lpi-ul.py
#
# Extracts UL tables from:
#   https://lpi.oregonstate.edu/mic/minerals/phosphorus
#   https://lpi.oregonstate.edu/mic/minerals/magnesium

import json
import os
import re
from datetime import datetime, timezone
from html import unescape

SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
EXTERNAL_DIR = os.path.join(SCRIPT_DIR, "external")

P_HTML = os.path.join(EXTERNAL_DIR, "lpi-phosphorus-ul.html")
MG_HTML = os.path.join(EXTERNAL_DIR, "lpi-magnesium-ul.html")
OUT_PATH = os.path.join(SCRIPT_DIR, "dri-p-mg-ul-parsed.json")

P_URL = "https://lpi.oregonstate.edu/mic/minerals/phosphorus"
MG_URL = "https://lpi.oregonstate.edu/mic/minerals/magnesium"


def extract_tables(html):
    """Extract all HTML tables as list of (header_row, data_rows)."""
    tables = re.findall(r'(?s)<table[^>]*>(.*?)</table>', html)
    results = []
    for t in tables:
        rows = re.findall(r'(?s)<tr[^>]*>(.*?)</tr>', t)
        parsed = []
        for r in rows:
            cells = re.findall(r'(?s)<t[hd][^>]*>(.*?)</t[hd]>', r)
            text = [unescape(re.sub(r'<[^>]+>', ' ', c).strip()) for c in cells]
            text = [re.sub(r'\s+', ' ', t).strip() for t in text]
            parsed.append([t for t in text if t])
        if parsed:
            results.append(parsed)
    return results


def parse_ul_value(cell):
    """Extract numeric UL value from a cell like '4,000 (4.0 g)' or '350' or 'Not possible to establish'."""
    cell = cell.strip()
    if 'not possible' in cell.lower() or 'nd' in cell.lower():
        return None
    # Extract first number
    m = re.search(r'([\d,]+)', cell.replace(',', ''))
    if m:
        return int(m.group(1).replace(',', ''))
    return None


def parse_phosphorus_ul(html):
    """Parse Phosphorus UL from LPI page."""
    tables = extract_tables(html)
    ul_rows = {}

    # Find the UL table — has 'Tolerable Upper Intake Level' in header or nearby
    for t in tables:
        header_text = str(t[0]).lower() if t else ''
        if 'ul' in header_text or 'tolerable' in header_text:
            for row in t[1:]:
                if len(row) >= 2:
                    age_raw = row[0].strip()
                    ul_val = parse_ul_value(row[1])
                    ul_rows[age_raw] = ul_val
            break

    # Map to group IDs
    age_map = {
        "Infants 0-12 months": "infants",
        "Children 1-3 years": "children_1_3yr",
        "Children 4-8 years": "children_4_8yr",
        "Children 9-13 years": "children_9_13yr",
        "Adolescents 14-18 years": "adolescent_14_18yr",
        "Adults 19-70 years": "adult_19_70yr",
        "Adults 71 years and older": "adult_gt70yr",
        "Pregnancy": "pregnant",
        "Breastfeeding": "breastfeeding",
    }

    result = {}
    for age_raw, val in ul_rows.items():
        key = age_map.get(age_raw)
        if key:
            result[key] = val

    return result


def parse_magnesium_ul(html):
    """Parse Magnesium UL from LPI page."""
    tables = extract_tables(html)
    ul_rows = {}

    for t in tables:
        header_text = str(t[0]).lower() if t else ''
        if 'ul' in header_text or 'tolerable' in header_text:
            for row in t[1:]:
                if len(row) >= 2:
                    age_raw = row[0].strip()
                    ul_val = parse_ul_value(row[1])
                    ul_rows[age_raw] = ul_val
            break

    age_map = {
        "Infants 0-12 months": "infants",
        "Children 1-3 years": "children_1_3yr",
        "Children 4-8 years": "children_4_8yr",
        "Children 9-13 years": "children_9_13yr",
        "Adolescents 14-18 years": "adolescent_14_18yr",
        "Adults 19 years and older": "adult",
    }

    result = {}
    for age_raw, val in ul_rows.items():
        key = age_map.get(age_raw)
        if key:
            result[key] = val

    return result


def main():
    print("LPI UL Parser — Phosphorus and Magnesium")
    print("=========================================")
    print()

    with open(P_HTML) as f:
        p_html = f.read()
    with open(MG_HTML) as f:
        mg_html = f.read()

    p_ul = parse_phosphorus_ul(p_html)
    mg_ul = parse_magnesium_ul(mg_html)

    print(f"Phosphorus UL: {len(p_ul)} age groups")
    for k, v in p_ul.items():
        print(f"  {k}: {v}")

    print(f"\nMagnesium UL: {len(mg_ul)} age groups")
    for k, v in mg_ul.items():
        print(f"  {k}: {v}")

    output = {
        "_meta": {
            "source_id": "lpi-mic-minerals",
            "source_file": "data/external/lpi-phosphorus-ul.html, data/external/lpi-magnesium-ul.html",
            "source_urls": [P_URL, MG_URL],
            "extraction_date": datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
            "extraction_script": "data/extract-lpi-ul.py",
            "extraction_method": "html-table-parser",
            "extracted_by": "agent",
            "auto_generated": True,
            "warning": "AUTO-GENERATED FILE. DO NOT EDIT MANUALLY. Run the extraction script to regenerate.",
            "source_claims": {
                "presumed_date": "2025",
                "presumed_author": "Linus Pauling Institute, Oregon State University",
                "based_on": "Institute of Medicine DRI reports (1997 for P/Mg)",
            },
            "note": "UL values from Linus Pauling Institute Micronutrient Information Center. LPI is an academic peer-reviewed source that reproduces IOM DRI values. Tier B — authoritative secondary, machine-readable. UL for Magnesium applies to supplemental magnesium only, not food sources. UL for Phosphorus: 4000 mg (19-70 yr), 3000 mg (>70 yr).",
        },
        "phosphorus_ul": p_ul,
        "magnesium_ul": mg_ul,
    }

    with open(OUT_PATH, "w") as f:
        json.dump(output, f, ensure_ascii=False, indent=2)
    print(f"\nWritten to {OUT_PATH}")


if __name__ == "__main__":
    main()
