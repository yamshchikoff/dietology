# Extract WHO GHO epidemiological statistics from OData API JSON dumps.
# Source: WHO Global Health Observatory (GHO) via https://ghoapi.azureedge.net/
# License: CC BY 4.0 (WHO GHO data)
# Run: python3 extract-who-gho.py

import json
import os
import sys
from datetime import datetime, timezone

SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))

# Input → Output mapping
DATASETS = {
    "who-NUTRITION_ANAEMIA_NONPREGNANT_PREV": {
        "input": "external/who-NUTRITION_ANAEMIA_NONPREGNANT_PREV.json",
        "output": "who-anaemia-nonpregnant-prevalence.json",
        "indicator_name": "Anaemia prevalence in non-pregnant women (%)",
        "indicator_code": "NUTRITION_ANAEMIA_NONPREGNANT_PREV",
        "description": "Prevalence of anaemia in women of reproductive age (15–49), non-pregnant. By country, year, and severity (total/mild/moderate/severe).",
        "dimensions": ["SEX", "SEVERITY"],
        "gho_url": "https://www.who.int/data/gho/data/indicators/indicator-details/GHO/prevalence-of-anaemia-in-non-pregnant-women-estimates-technical-brief-(-)",
    },
    "who-NCD_BMI_25A": {
        "input": "external/who-NCD_BMI_25A.json",
        "output": "who-bmi-overweight-prevalence.json",
        "indicator_name": "Prevalence of overweight (BMI ≥25) among adults (%)",
        "indicator_code": "NCD_BMI_25A",
        "description": "Age-standardized prevalence of overweight (BMI ≥25 kg/m²) among adults aged 18+. By country, year, and sex.",
        "dimensions": ["SEX", "AGEGROUP"],
        "gho_url": "https://www.who.int/data/gho/data/indicators/indicator-details/GHO/prevalence-of-overweight-among-adults-bmi-greater-or-equal-25-(age-standardized-estimate)(-)",
    },
    "who-NCD_DIABETES_PREVALENCE_AGESTD": {
        "input": "external/who-NCD_DIABETES_PREVALENCE_AGESTD.json",
        "output": "who-diabetes-prevalence.json",
        "indicator_name": "Prevalence of diabetes, age-standardized (%)",
        "indicator_code": "NCD_DIABETES_PREVALENCE_AGESTD",
        "description": "Age-standardized prevalence of raised fasting blood glucose (≥7.0 mmol/L) or on medication for diabetes among adults. By country, year, sex, and age group (18+ / 30+).",
        "dimensions": ["SEX", "AGEGROUP"],
        "gho_url": "https://www.who.int/data/gho/data/indicators/indicator-details/GHO/prevalence-of-raised-fasting-blood-glucose-among-adults-aged-18-years-(age-standardized-estimate)",
    },
}

# Country code → ISO3 mapping sourced from WHO GHO API metadata
COUNTRY_CODES = {}  # populated from data


def load_json(path):
    with open(path) as f:
        return json.load(f)


def extract_dataset(config):
    """Extract structured records from a WHO GHO OData JSON dump."""
    input_path = os.path.join(SCRIPT_DIR, config["input"])
    if not os.path.exists(input_path):
        print(f"  SKIP: {input_path} not found")
        return None

    raw = load_json(input_path)
    records = raw["value"]
    print(f"  {len(records):,} raw records")

    # Build country code mapping
    for r in records:
        COUNTRY_CODES[r["SpatialDim"]] = r.get("ParentLocation", "")

    # Group by country and year
    data = []
    for r in records:
        entry = {
            "country_code": r["SpatialDim"],
            "year": r["TimeDim"],
            "value": r["NumericValue"],
            "low": r["Low"],
            "high": r["High"],
            "parent_region": r.get("ParentLocation", ""),
            "parent_region_code": r.get("ParentLocationCode", ""),
        }
        # Add dimension values
        if r.get("Dim1Type"):
            entry[r["Dim1Type"].lower()] = r.get("Dim1", "")
        if r.get("Dim2Type"):
            entry[r["Dim2Type"].lower()] = r.get("Dim2", "")

        data.append(entry)

    # Sort by country, year, then dimensions
    dim_keys = [d.lower() for d in config["dimensions"]]
    data.sort(key=lambda x: (
        x["country_code"],
        x["year"],
        *[x.get(k, "") for k in dim_keys],
    ))

    output = {
        "_meta": {
            "source_id": "who-gho",
            "source_urls": [
                config["gho_url"],
                "https://ghoapi.azureedge.net/api/" + config["indicator_code"],
            ],
            "extraction_date": datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
            "extraction_script": "data/extract-who-gho.py",
            "extraction_method": "OData API JSON dump → structured extraction",
            "extracted_by": "agent",
            "license": "CC BY 4.0",
            "license_note": "WHO GHO data is published under CC BY 4.0. Attribution: World Health Organization, Global Health Observatory.",
            "source_claims": {
                "presumed_date": "2022–2024 (various updates per record; see Date field in raw records)",
                "presumed_author": "World Health Organization, Global Health Observatory",
            },
            "source_file": f"data/{config['input']}",
            "indicator_code": config["indicator_code"],
            "indicator_name": config["indicator_name"],
            "description": config["description"],
            "record_count": len(data),
            "notes": "Values are modelled estimates with 95% confidence intervals (low/high). Not raw survey data — WHO uses statistical modelling for comparability across countries and years.",
        },
        "data": data,
    }

    return output


def main():
    print("WHO GHO Epidemiological Data Extractor")
    print("=======================================")
    print()

    for dataset_id, config in DATASETS.items():
        print(f"{dataset_id}:")
        output = extract_dataset(config)
        if output is None:
            continue

        output_path = os.path.join(SCRIPT_DIR, config["output"])
        with open(output_path, "w") as f:
            json.dump(output, f, ensure_ascii=False, indent=2)

        size_kb = os.path.getsize(output_path) / 1024
        print(f"  → {config['output']} ({len(output['data']):,} records, {size_kb:.0f} KB)")

    print()
    print(f"Country codes mapped: {len(COUNTRY_CODES)}")


if __name__ == "__main__":
    main()
