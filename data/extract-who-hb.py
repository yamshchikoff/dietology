# Extract WHO 2024 Haemoglobin Cutoffs for Anaemia
# Source: WHO Guideline (Tier B), CC BY-NC-SA 3.0 IGO
# Run: python3 extract-who-hb.py
#
# WHO IRIS requires JavaScript to download PDF.
# This script validates the existing who-hb-thresholds.json.
# When WHO provides direct PDF download, add pdfplumber extraction.

import json
import os
import sys
from urllib.request import urlopen, Request

SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))

WHO_PUBLICATION_URL = "https://www.who.int/publications/i/item/9789240088542"
WHO_IRIS_PDF = "https://iris.who.int/bitstream/handle/10665/376196/9789240088542-eng.pdf"
OUTPUT = os.path.join(SCRIPT_DIR, "who-hb-thresholds.json")

HEADERS = {"User-Agent": "Mozilla/5.0 (compatible; dietology-data-bot/1.0)"}


def validate_output():
    with open(OUTPUT) as f:
        data = json.load(f)

    source_id = data["_meta"]["source_id"]
    assert source_id == "who-2024-hb", f"Bad source_id: {source_id}"

    thresholds = data["diagnostic_thresholds"]
    severity = data["severity_classification"]
    legacy = data["legacy_comparison"]

    print(f"Diagnostic thresholds: {len(thresholds)} groups")
    print(f"Severity classifications: {len(severity)} groups")
    print(f"Legacy comparisons: {len(legacy)} entries")

    # Validate key 2024 changes
    infants = [t for t in thresholds if "6_23" in t["group"]][0]
    assert infants["hb_cutoff_g_per_l"] == 105, "Infant cutoff should be 105 g/L (2024 update)"

    preg_t2 = [t for t in thresholds if "second_trimester" in t["group"]][0]
    assert preg_t2["hb_cutoff_g_per_l"] == 105, "Second trimester cutoff should be 105 g/L"

    men = [t for t in thresholds if t["group"] == "men_15_plus"][0]
    assert men["hb_cutoff_g_per_l"] == 130, "Male cutoff should be 130 g/L"

    # Validate severity
    for s in severity:
        assert s["normal_low"] > s["mild_low"], \
            f"normal > mild for {s['group']}: {s['normal_low']} > {s['mild_low']}"
        assert s["mild_low"] >= s["moderate_low"], \
            f"mild >= moderate for {s['group']}"
        assert s["moderate_low"] >= s["severe_below"], \
            f"moderate >= severe for {s['group']}: {s['moderate_low']} >= {s['severe_below']}"
        # severe_below is exclusive (< value); moderate_low is inclusive (>= value)
        # they can be equal at boundary

    print("All validations passed.")
    return True


def main():
    print("WHO 2024 Haemoglobin Guideline Extractor")
    print("========================================")
    print()
    print(f"Publication: {WHO_PUBLICATION_URL}")
    print(f"PDF: {WHO_IRIS_PDF}")
    print()
    print("WHO IRIS uses DSpace (JavaScript SPA) — direct PDF download")
    print("requires browser. After manual download, save to:")
    print(f"  {os.path.join(SCRIPT_DIR, 'external', 'who-2024-hb-guideline.pdf')}")
    print()
    print("Current data extracted from WHO 2024 guideline tables via")
    print("structured table transcription (WebFetch of published values).")
    print()

    if not os.path.exists(OUTPUT):
        print(f"ERROR: {OUTPUT} not found — extraction required")
        sys.exit(1)

    print(f"Validating {os.path.basename(OUTPUT)}...")
    if validate_output():
        print()
        print("Status: JSON data valid and current.")
        print("License note: WHO CC BY-NC-SA 3.0 IGO. Numeric facts are not")
        print("copyrightable. Attribution to WHO 2024 guideline in _meta block.")

    # Check if PDF was downloaded manually
    pdf_path = os.path.join(SCRIPT_DIR, "external", "who-2024-hb-guideline.pdf")
    if os.path.exists(pdf_path):
        print(f"PDF found: {pdf_path}")
        print("Run pdfplumber extraction to verify table data (future).")
    else:
        print(f"PDF not found: {pdf_path}")
        print("Download manually from https://www.who.int/publications/i/item/9789240088542")


if __name__ == "__main__":
    main()
