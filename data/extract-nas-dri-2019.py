# Extract Sodium and Potassium DRI (AI + CDRR) from NAS 2019 PDF Highlights.
# Source: National Academies 2019 — Dietary Reference Intakes for Sodium and Potassium
# Run: python3 extract-nas-dri-2019.py
#
# The PDF is a 4-page "Highlights" summary containing Tables 1 and 2
# with full age/sex/life-stage breakdown for both nutrients.

import json
import os
import re
from datetime import datetime, timezone

import pdfplumber

SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
EXTERNAL_DIR = os.path.join(SCRIPT_DIR, "external")

PDF_PATH = os.path.join(EXTERNAL_DIR, "nas-dri-sodium-potassium-2019.pdf")
OUT_PATH = os.path.join(SCRIPT_DIR, "dri-na-k-2019-parsed.json")

SOURCE_URL = "https://nap.nationalacademies.org/catalog/25353"
PDF_URL = "https://nap.nationalacademies.org/resource/25353/030519DRISodiumPotassium.pdf"


def extract_table_rows(text):
    """Extract data rows from pdfplumber-extracted text.
    The tables have column headers: Life-Stage Group | AI (mg/d) | UL | CDRR
    Data rows follow the pattern: age_range number ND? (value)?
    """
    rows = []
    for line in text.split("\n"):
        line = line.strip()
        if not line:
            continue
        # Skip header, notes, table captions
        if any(skip in line for skip in ["TABLE ", "NOTES:", "Life-Stage Group", "L i f e"]):
            continue
        rows.append(line)
    return rows


def parse_potassium_table(page_text):
    """Parse Potassium Table 1 from page 2."""
    groups = []
    # The table data starts after "TABLE 1:" header
    # Pattern: life_stage_group AI_value ND ND
    # Life-stage groups are indented under categories (Infants, Children, Males, etc.)
    current_category = None

    category_map = {
        "Infants": "infants",
        "Children": "children",
        "Males": "male",
        "Females": "female",
        "Pregnancy": "pregnant",
        "Lactation": "breastfeeding",
    }

    age_range_map = {
        "0–6 months": "0_6mo",
        "7–12 months": "7_12mo",
        "1–3 years": "1_3yr",
        "4–8 years": "4_8yr",
        "9–13 years": "9_13yr",
        "14–18 years": "14_18yr",
        "19–30 years": "19_30yr",
        "31–50 years": "31_50yr",
        "51–70 years": "51_70yr",
        ">70 years": "gt70yr",
    }

    sex_map = {
        "Infants": "any",
        "Children": "any",
        "Males": "male",
        "Females": "female",
        "Pregnancy": "female",
        "Lactation": "female",
    }

    data_pattern = re.compile(r"([>\d][\d–\-–\s>]*?\S+)\s+([\d,]+)")

    lines = page_text.split("\n")
    for line in lines:
        line = line.strip()
        if not line:
            continue

        # Detect category
        for cat_label in category_map:
            if line == cat_label:
                current_category = cat_label
                break

        if current_category is None:
            continue

        m = data_pattern.match(line)
        if not m:
            continue

        age_raw = m.group(1).strip()
        age_raw = re.sub(r"[–—−]", "–", age_raw)
        age_raw = re.sub(r"-", "–", age_raw)
        ai_val = int(m.group(2).replace(",", ""))

        group_id = age_range_map.get(age_raw)
        if group_id is None:
            continue

        prefix = category_map[current_category]
        full_group = f"{prefix}_{group_id}"
        sex = sex_map[current_category]

        groups.append({
            "group": full_group,
            "sex": sex,
            "age_range": age_raw,
            "value": ai_val,
            "type": "AI",
            "ul": None,
            "ul_note": "ND — not determined owing to lack of a toxicological indicator specific to excessive potassium intake",
        })

    return groups


def parse_sodium_table(page_text):
    """Parse Sodium Table 2 from page 3."""
    groups = []
    current_category = None

    category_map = {
        "Infants": "infants",
        "Children": "children",
        "Males": "male",
        "Females": "female",
        "Pregnancy": "pregnant",
        "Lactation": "breastfeeding",
    }

    age_range_map = {
        "0–6 months": "0_6mo",
        "7–12 months": "7_12mo",
        "1–3 years": "1_3yr",
        "4–8 years": "4_8yr",
        "9–13 years": "9_13yr",
        "14–18 years": "14_18yr",
        "19–30 years": "19_30yr",
        "31–50 years": "31_50yr",
        "51–70 years": "51_70yr",
        ">70 years": "gt70yr",
    }

    sex_map = {
        "Infants": "any",
        "Children": "any",
        "Males": "male",
        "Females": "female",
        "Pregnancy": "female",
        "Lactation": "female",
    }

    # Extract AI and CDRR values from rows
    # Pattern: age_range AI_number ND? CDRR_text_or_number
    # CDRR column: "Reduce intakes if above X,XXX mg/day" or "Reduce intakes if above X,XXX mg/dayd"
    cdrr_pattern = re.compile(r"Reduce intakes if above ([\d,]+)\s*mg/day")

    data_pattern = re.compile(r"([>\d][\d–\-–\s>]*?\S+)\s+([\d,]+)")

    lines = page_text.split("\n")
    for line in lines:
        line = line.strip()
        if not line:
            continue

        for cat_label in category_map:
            if line == cat_label:
                current_category = cat_label
                break

        if current_category is None:
            continue

        m = data_pattern.match(line)
        if not m:
            continue

        age_raw = m.group(1).strip()
        age_raw = re.sub(r"[–—−]", "–", age_raw)
        age_raw = re.sub(r"-", "–", age_raw)
        ai_val = int(m.group(2).replace(",", ""))

        group_id = age_range_map.get(age_raw)
        if group_id is None:
            continue

        prefix = category_map[current_category]
        full_group = f"{prefix}_{group_id}"
        sex = sex_map[current_category]

        # Extract CDRR value
        cdrr_val = None
        cdrr_match = cdrr_pattern.search(line)
        if cdrr_match:
            cdrr_val = int(cdrr_match.group(1).replace(",", ""))

        # Check for "ad" suffix on AI (indicates values extrapolated from adult)
        ai_note = None
        if "a" in line.split(age_raw)[-1].split(str(ai_val))[0] if False else False:
            pass

        note = f"CDRR: {cdrr_val} mg/day" if cdrr_val else "CDRR: not determined"

        groups.append({
            "group": full_group,
            "sex": sex,
            "age_range": age_raw,
            "value": ai_val,
            "type": "AI",
            "cdrr": cdrr_val,
            "ul": None,
            "ul_note": "ND — not determined; no UL established for sodium. CDRR replaces UL concept.",
            "note": note,
        })

    return groups


def main():
    print("NAS 2019 DRI PDF Parser — Sodium and Potassium")
    print("==============================================")
    print()

    with pdfplumber.open(PDF_PATH) as pdf:
        # Table 1 (Potassium) is on page 2 (index 1)
        # Table 2 (Sodium) is on page 3 (index 2)
        k_text = pdf.pages[1].extract_text()
        na_text = pdf.pages[2].extract_text()

    # Parse Potassium
    k_groups = parse_potassium_table(k_text)
    print(f"Potassium: {len(k_groups)} groups")

    # Parse Sodium
    na_groups = parse_sodium_table(na_text)
    print(f"Sodium: {len(na_groups)} groups")

    # Build output
    nutrients = [
        {
            "name": "Potassium",
            "unit": "mg",
            "category": "macromineral",
            "source_id": "nas-dri-2019",
            "source_urls": [SOURCE_URL, PDF_URL],
            "ul": None,
            "ul_unit": None,
            "ul_note": "ND — not determined. NAS 2019: insufficient evidence to establish UL for potassium.",
            "note": "AI values from NAS 2019 DRI for Sodium and Potassium. No UL or CDRR established for potassium.",
            "groups": k_groups,
        },
        {
            "name": "Sodium",
            "unit": "mg",
            "category": "macromineral",
            "source_id": "nas-dri-2019",
            "source_urls": [SOURCE_URL, PDF_URL],
            "ul": None,
            "ul_unit": None,
            "ul_note": "ND — not determined. No UL for sodium. CDRR (Chronic Disease Risk Reduction) replaces UL: 2300 mg/day for adults ≥14 yr; lower for children.",
            "note": "AI values from NAS 2019 DRI for Sodium and Potassium. CDRR (not UL) is the relevant upper intake benchmark for sodium.",
            "groups": na_groups,
        },
    ]

    output = {
        "_meta": {
            "source_id": "nas-dri-2019",
            "source_file": "data/external/nas-dri-sodium-potassium-2019.pdf",
            "source_urls": [SOURCE_URL, PDF_URL],
            "extraction_date": datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
            "extraction_script": "data/extract-nas-dri-2019.py",
            "extraction_method": "pdfplumber-text-strategy",
            "extracted_by": "agent",
            "source_claims": {
                "presumed_date": "2019",
                "presumed_author": "National Academies of Sciences, Engineering, and Medicine",
            },
            "note": "Values extracted from the 4-page Highlights PDF (not full report). Contains Tables 1 and 2 with complete DRI for Potassium and Sodium: AI by age/sex/life-stage, UL (ND for both), CDRR (Sodium only). This is the current authoritative DRI for these nutrients in US/Canada, superseding IOM 2005.",
            "stats": {
                "potassium_groups": len(k_groups),
                "sodium_groups": len(na_groups),
            },
        },
        "nutrients": nutrients,
    }

    with open(OUT_PATH, "w") as f:
        json.dump(output, f, ensure_ascii=False, indent=2)
    print(f"Written to {OUT_PATH}")

    # Print summary
    for n in nutrients:
        print()
        print(f"{n['name']}: {len(n['groups'])} groups")
        for g in n["groups"]:
            cdrr_str = f"  CDRR={g.get('cdrr')}" if g.get('cdrr') else ""
            print(f"  {g['group']:30s} {g['age_range']:15s} {g['value']:5d} mg [{g['type']}]{cdrr_str}")


if __name__ == "__main__":
    main()
