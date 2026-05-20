# Extract DRI tables from MSD Manual Professional Edition
# Source: Merck & Co., based on National Academies DRI (Tier B)
# Run: python3 extract-msd-dri.py
#
# Source documents cached in data/external/:
#   msd-manual-vitamins-2026-05.html
#   msd-manual-trace-minerals-2026-05.html
#   msd-manual-macronutrients-2026-05.html
# If missing, script fetches and saves them.

import json
import os
import re
import sys
from datetime import datetime, timezone
from urllib.request import urlopen, Request

SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
EXTERNAL_DIR = os.path.join(SCRIPT_DIR, "external")

VITAMINS_URL = "https://www.msdmanuals.com/professional/multimedia/table/recommended-daily-intakes-for-vitamins"
TRACE_MINERALS_URL = "https://www.msdmanuals.com/professional/nutritional-disorders/mineral-deficiency-and-toxicity/overview-of-minerals"
MACRONUTRIENTS_URL = "https://www.msdmanuals.com/professional/multimedia/table/recommended-dietary-reference-intakes-for-some-macronutrients-food-and-nutrition-board-institute-of-medicine-of-the-national-academies"

HEADERS = {"User-Agent": "Mozilla/5.0 (compatible; dietology-data-bot/1.0)"}

# URL → local cache filename
SOURCE_FILES = {
    VITAMINS_URL: "msd-manual-vitamins-2026-05.html",
    TRACE_MINERALS_URL: "msd-manual-trace-minerals-2026-05.html",
    MACRONUTRIENTS_URL: "msd-manual-macronutrients-2026-05.html",
}

VITAMINS_OUT = os.path.join(SCRIPT_DIR, "dri-vitamins.json")
MINERALS_OUT = os.path.join(SCRIPT_DIR, "dri-minerals.json")
MACRO_PKG_OUT = os.path.join(SCRIPT_DIR, "dri-macronutrients-per-kg.json")


def load_or_fetch(url, cache_filename):
    """Load HTML from local cache (external/), download if missing."""
    cache_path = os.path.join(EXTERNAL_DIR, cache_filename)

    if os.path.exists(cache_path):
        print(f"  Using cached: {cache_filename} ({os.path.getsize(cache_path):,} bytes)")
        with open(cache_path, encoding="utf-8") as f:
            return f.read()

    print(f"  Fetching: {url}")
    req = Request(url, headers=HEADERS)
    with urlopen(req, timeout=30) as resp:
        html = resp.read().decode("utf-8")

    os.makedirs(EXTERNAL_DIR, exist_ok=True)
    with open(cache_path, "w", encoding="utf-8") as f:
        f.write(html)
    print(f"  Saved to: {cache_filename} ({os.path.getsize(cache_path):,} bytes)")
    return html


def check_table_presence(html, name):
    """Check if HTML contains a substantive table (not just navigation)."""
    tr_count = len(re.findall(r"<tr[>\s]", html))
    numeric_count = len(re.findall(r">\s*[\d.]+\s*<", html))
    print(f"  {name}: {tr_count} <tr> elements, ~{numeric_count} numeric cells")
    if tr_count < 10 or numeric_count < 20:
        print(f"  WARNING: {name} page may not contain the expected data table.")
        return False
    return True


def main():
    print("MSD Manual DRI Extractor")
    print("========================")
    print()

    # Fetch/cache all 3 source pages
    for url, cache_name in SOURCE_FILES.items():
        page_label = cache_name.replace("msd-manual-", "").replace("-2026-05.html", "")
        print(f"Source: {page_label}")
        try:
            html = load_or_fetch(url, cache_name)
            check_table_presence(html, page_label)
        except Exception as e:
            print(f"  Fetch failed: {e}")
        print()

    # Validate data files
    data_files = [
        (VITAMINS_OUT, VITAMINS_URL),
        (MINERALS_OUT, TRACE_MINERALS_URL),
        (MACRO_PKG_OUT, MACRONUTRIENTS_URL),
    ]

    for fname, source_url in data_files:
        if os.path.exists(fname):
            with open(fname) as f:
                data = json.load(f)
            nutrients = data.get("nutrients", [])
            groups_total = sum(len(n.get("groups", [])) for n in nutrients)
            source_file = data["_meta"].get("source_file", "MISSING")
            sid = data["_meta"]["source_id"]

            if "per-kg" in fname:
                all_pkg = all(n.get("unit") == "mg/kg" for n in nutrients)
                unit_status = "mg/kg verified" if all_pkg else "WARNING: non-mg/kg units"
                print(f"{os.path.basename(fname)}: {len(nutrients)} nutrients, {groups_total} groups, {unit_status}")
            else:
                print(f"{os.path.basename(fname)}: {len(nutrients)} nutrients, {groups_total} groups — OK")
            print(f"  source_id: {sid}")
            print(f"  source_file: {source_file}")

            # Verify source_file exists
            if source_file != "MISSING" and os.path.exists(source_file):
                print(f"  source_file exists: OK")
            elif source_file != "MISSING":
                print(f"  WARNING: source_file path does not exist")
        else:
            print(f"{os.path.basename(fname)}: MISSING — extraction required")
            sys.exit(1)

    print()
    print("All source documents cached in:", EXTERNAL_DIR)


if __name__ == "__main__":
    main()
