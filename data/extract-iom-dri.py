# Programmatic PDF table parser for IOM DRI Calcium values
# Source: IOM 2011 Dietary Reference Intakes for Calcium and Vitamin D
# Run: python3 extract-iom-dri.py
#
# Extracts Calcium DRI from Table S-1 (page 23) of IOM 2011 PDF.
# Phosphorus and Magnesium DRI from IOM 1997 PDF are NOT extracted
# programmatically — PDF uses scrambled character rendering preventing
# reliable pdfplumber table extraction. Values manually transcribed.

import json
import os
import re
from datetime import datetime, timezone

import pdfplumber

SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
EXTERNAL_DIR = os.path.join(SCRIPT_DIR, "external")

IOM_2011_PDF = os.path.join(EXTERNAL_DIR, "iom-dri-calcium-vitamin-d-2011.pdf")
CALCIUM_OUT = os.path.join(SCRIPT_DIR, "dri-calcium-iom-2011-parsed.json")


def reconstruct_rows(table_data):
    """Join character-split text columns into one string per row."""
    rows = []
    for row in table_data:
        text = " ".join(c or "" for c in row)
        text = re.sub(r"\s+", " ", text).strip()
        rows.append(text)
    return rows


def parse_calcium_table(pdf_path):
    """Extract Calcium DRI from IOM 2011 PDF Table S-1 (page 23)."""
    with pdfplumber.open(pdf_path) as pdf:
        page = pdf.pages[22]  # Page 23, 0-indexed
        tables = page.find_tables({
            "vertical_strategy": "text",
            "horizontal_strategy": "text",
        })
        if not tables:
            raise ValueError("No table found on page 23 of IOM 2011 PDF")
        data = tables[0].extract()

    rows = reconstruct_rows(data)

    # Find the data section — starts after "Life Stage Group" header row
    # Header text has character-split artifacts: "Li fe Stag e Group"
    start_idx = None
    for i, row in enumerate(rows):
        nosp = row.replace(" ", "")
        if "LifeStage" in nosp and "Group" in nosp:
            start_idx = i + 2  # skip header + blank row
            break

    if start_idx is None:
        raise ValueError("Could not find 'Life Stage Group' header in table")

    # Match categories by removing all spaces from row
    # "In fa nts" → "Infants", "Ch il dren" → "Children"
    category_names = {
        "Infants": "Infants",
        "Children": "Children",
        "Males": "Males",
        "Females": "Females",
        "Pregnancy": "Pregnancy",
        "Lactation": "Lactation",
    }

    # Group map: keys are category + age (no spaces) → (group_id, sex)
    group_map = {
        ("Infants", "0to6mo"): ("infants_0_6mo", "any"),
        ("Infants", "6to12mo"): ("infants_7_12mo", "any"),
        ("Children", "1–3y"): ("children_1_3yr", "any"),
        ("Children", "4–8y"): ("children_4_8yr", "any"),
        ("Males", "9–13y"): ("male_9_13yr", "male"),
        ("Males", "14–18y"): ("male_14_18yr", "male"),
        ("Males", "19–30y"): ("male_19_30yr", "male"),
        ("Males", "31–50y"): ("male_31_50yr", "male"),
        ("Males", "51–70y"): ("male_51_70yr", "male"),
        ("Males", ">70y"): ("male_gt70yr", "male"),
        ("Females", "9–13y"): ("female_9_13yr", "female"),
        ("Females", "14–18y"): ("female_14_18yr", "female"),
        ("Females", "19–30y"): ("female_19_30yr", "female"),
        ("Females", "31–50y"): ("female_31_50yr", "female"),
        ("Females", "51–70y"): ("female_51_70yr", "female"),
        ("Females", ">70y"): ("female_gt70yr", "female"),
        ("Pregnancy", "14–18y"): ("pregnant_14_18yr", "female"),
        ("Pregnancy", "19–30y"): ("pregnant_19_30yr", "female"),
        ("Pregnancy", "31–50y"): ("pregnant_31_50yr", "female"),
        ("Lactation", "14–18y"): ("breastfeeding_14_18yr", "female"),
        ("Lactation", "19–30y"): ("breastfeeding_19_30yr", "female"),
        ("Lactation", "31–50y"): ("breastfeeding_31_50yr", "female"),
    }

    value_pattern = re.compile(r"([\d,]+)\s*m\s*g")

    groups = []
    current_category = None

    for row in rows[start_idx:]:
        row = row.strip()
        if not row:
            continue

        # Detect category: remove all spaces, check against known names
        nosp = row.replace(" ", "")
        if nosp in category_names:
            current_category = category_names[nosp]
            continue

        # Note row
        if nosp.startswith("NOTE:") or row.startswith("a "):
            continue

        if not current_category:
            continue

        # Extract age range: use the earliest boundary between "—" and first "mg"
        dash_pos = row.find("—")
        first_mg = re.search(r"\d[\d,]*\s*m\s*g", row)
        if not first_mg:
            continue
        mg_pos = first_mg.start()
        # For infants (AI present): first mg is before —, use mg position
        # For others (AI = —): dash is before first mg, use dash position
        boundary = min(dash_pos, mg_pos) if dash_pos >= 0 else mg_pos
        if boundary == -1:
            continue
        age_raw = row[:boundary].strip()
        # Remove all spaces from age: "0 t o 6 mo" → "0to6mo"
        age_key = age_raw.replace(" ", "")
        # Normalize dashes
        age_key = re.sub(r"[–—−]", "–", age_key)
        # Display version: "0to6mo" → "0 to 6 mo"
        age_display = re.sub(r"(\d)(to)(\d)", r"\1 \2 \3", age_key)
        age_display = re.sub(r"(>)(\d)", r"\1 \2", age_display)

        group_key = (current_category, age_key)
        if group_key not in group_map:
            print(f"  WARNING: unknown calcium group: {group_key}")
            continue

        group_id, sex = group_map[group_key]

        # Extract all mg values from the row
        values = value_pattern.findall(row)
        values = [int(v.replace(",", "")) for v in values]

        # Table S-1 columns: AI, EAR, RDA, UL
        # Infants: AI present, RDA = —
        # Others: AI = —, EAR and RDA present
        # UL always present
        ai_val = None
        rda_val = None
        ul_val = None

        if current_category == "Infants":
            # Two values: AI, UL (EAR and RDA are —)
            if len(values) >= 2:
                ai_val = values[0]
                ul_val = values[-1]
        elif current_category in ("Pregnancy", "Lactation"):
            # Three values: EAR, RDA, UL (AI = —)
            if len(values) >= 3:
                rda_val = values[1]
                ul_val = values[2]
        else:
            # Children/Males/Females: Three values: EAR, RDA, UL
            if len(values) >= 3:
                rda_val = values[1]
                ul_val = values[2]

        groups.append({
            "group": group_id,
            "sex": sex,
            "age_range": age_display,
            "ai": ai_val,
            "rda": rda_val,
            "ul": ul_val,
        })

    return groups


def main():
    print("IOM 2011 DRI PDF Parser — Calcium")
    print("===================================")
    print()

    groups = parse_calcium_table(IOM_2011_PDF)
    print(f"Extracted {len(groups)} age/sex groups")

    # Build nutrient entry with RDA and UL
    rda_groups = []
    ul_groups = []
    for g in groups:
        has_rda = g["rda"] is not None
        val = g["rda"] if has_rda else g["ai"]
        val_type = "RDA" if has_rda else "AI"
        rda_groups.append({
            "group": g["group"],
            "sex": g["sex"],
            "age_range": g["age_range"],
            "value": val,
            "type": val_type,
        })
        if g["ul"] is not None:
            ul_groups.append({
                "group": g["group"],
                "sex": g["sex"],
                "value": g["ul"],
            })

    nutrient = {
        "name": "Calcium",
        "unit": "mg",
        "category": "macromineral",
        "groups": rda_groups,
        "ul_groups": ul_groups,
    }

    # Build UL value for general use
    # UL varies: 1000 (infants), 1500 (infants 6-12mo), 2500 (most adults),
    # 3000 (adolescents/pregnant), 2000 (>50 yr)
    adult_ul_vals = [g["value"] for g in ul_groups
                     if g["group"].startswith(("male_19", "female_19", "male_31", "female_31"))]
    senior_ul_vals = [g["value"] for g in ul_groups
                      if g["group"].startswith(("male_51", "female_51", "male_gt", "female_gt"))]

    ul_default = max(set(adult_ul_vals), key=adult_ul_vals.count) if adult_ul_vals else 2500
    ul_senior = max(set(senior_ul_vals), key=senior_ul_vals.count) if senior_ul_vals else 2000
    nutrient["ul"] = ul_default
    nutrient["ul_unit"] = "mg"
    nutrient["ul_note"] = f"Adults 19–50 yr. UL {ul_senior} mg for adults >50 yr."

    output = {
        "_meta": {
            "source_id": "iom-dri-2011",
            "source_file": "data/external/iom-dri-calcium-vitamin-d-2011.pdf",
            "source_urls": [
                "https://nap.nationalacademies.org/catalog/13050/"
            ],
            "extraction_date": datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
            "extraction_script": "data/extract-iom-dri.py",
            "extraction_method": "pdfplumber-text-strategy",
            "extracted_by": "agent",
            "source_claims": {
                "presumed_date": "2011",
                "presumed_author": "Institute of Medicine, National Academies Press"
            },
            "note": "Calcium DRI values extracted from IOM 2011 PDF Table S-1 (page 23). This is the current authoritative source for Calcium, superseding IOM 1997. Infants: AI. All other groups: RDA. UL values included per age group.",
        },
        "nutrients": [nutrient],
    }

    with open(CALCIUM_OUT, "w") as f:
        json.dump(output, f, ensure_ascii=False, indent=2)
    print(f"Written to {CALCIUM_OUT}")

    # Print summary
    print()
    for g in rda_groups:
        print(f"  {g['group']:30s} {g['age_range']:15s} {g['value']:5d} mg [{g['type']}]")
    print()
    print(f"UL general: {ul_default} mg, UL >50 yr: {ul_senior} mg")


if __name__ == "__main__":
    main()
