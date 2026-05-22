#!/usr/bin/env python3
"""Unified build script for the Dietology data pipeline.

Runs all extraction and build steps in correct order, handling the
circular dependency between data-index.json and sources-final.json.

Usage:
    python3 data/build-all.py              # full rebuild
    python3 data/build-all.py --help       # show options
    python3 data/build-all.py --dri-only   # DRI data only (skip USDA/WHO/Wikipedia)
"""

import os
import sys
import subprocess
import time
from datetime import datetime, timezone

SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
os.chdir(SCRIPT_DIR)  # All scripts expect CWD = data/


def run(script, description):
    """Run a Python script relative to SCRIPT_DIR. Returns elapsed seconds."""
    path = os.path.join(SCRIPT_DIR, script)
    print(f"\n{'='*60}")
    print(f"  {description}")
    print(f"  {script}")
    print(f"{'='*60}")
    start = time.time()
    result = subprocess.run([sys.executable, path], capture_output=False)
    elapsed = time.time() - start
    if result.returncode != 0:
        print(f"  FAILED (exit code {result.returncode})")
        sys.exit(result.returncode)
    print(f"  OK ({elapsed:.1f}s)")
    return elapsed


def main():
    dri_only = "--dri-only" in sys.argv
    if "--help" in sys.argv or "-h" in sys.argv:
        print(__doc__)
        print("Options:")
        print("  --dri-only    Skip USDA, WHO Hb, and Wikipedia extractions")
        print("  --help, -h    Show this help")
        sys.exit(0)

    print("Dietology Full Build")
    print("====================")
    print(f"Started: {datetime.now(timezone.utc).strftime('%Y-%m-%dT%H:%M:%SZ')}")
    print(f"Working directory: {SCRIPT_DIR}")
    print()

    total_start = time.time()

    # ── Phase 1: Extraction from source documents ──
    print("── Phase 1: Extraction from source documents ──")

    run("extract-msd-dri-parser.py",
        "Step 1/9 — Parse MSD Manual HTML: vitamins + trace minerals + per-kg macronutrients + NCBI crosscheck")

    run("extract-iom-dri.py",
        "Step 2/9 — Parse IOM 2011 PDF: Calcium DRI")

    run("extract-nas-dri-2019.py",
        "Step 3/9 — Parse NAS 2019 PDF: Sodium + Potassium DRI")

    run("extract-lpi-ul.py",
        "Step 4/9 — Parse LPI HTML: Phosphorus + Magnesium UL")

    # ── Phase 2: Overlay builds ──
    print("\n── Phase 2: Overlay builds ──")

    run("build-minerals-overlay.py",
        "Step 5/9 — Build minerals overlay (14 nutrients, 254 groups)")

    run("build-vitamins-overlay.py",
        "Step 6/9 — Build vitamins overlay (11 nutrients, 154 groups)")

    run("build-macronutrients-per-kg-overlay.py",
        "Step 7/9 — Build per-kg macronutrients overlay (3 nutrients, 51 groups)")

    # ── Phase 3: Additional data (non-DRI) ──
    if not dri_only:
        print("\n── Phase 3: Additional data extractions ──")

        if os.path.exists(os.path.join(SCRIPT_DIR, "external/usda-foundation-foods-2026-04.zip")):
            run("extract-usda.py",
                "USDA — Extract food composition data (363 foods)")
        else:
            print("  USDA: source zip not found in external/, skipping")

        if os.path.exists(os.path.join(SCRIPT_DIR, "external/who-2024-hb-guideline.pdf")):
            run("extract-who-hb.py",
                "WHO Hb — Validate hemoglobin thresholds (9 groups)")
        else:
            print("  WHO Hb: source PDF not found, skipping")

        run("extract-wiki-lab-ranges.py",
            "Wikipedia — Fetch lab reference ranges (254 tests)")
    else:
        print("\n── Phase 3: Skipped (--dri-only) ──")

    # ── Phase 4: Manifest builds (handle circular dependency) ──
    print("\n── Phase 4: Manifest builds ──")

    data_index_path = os.path.join(SCRIPT_DIR, "data-index.json")
    sources_final_path = os.path.join(SCRIPT_DIR, "sources-final.json")

    if not os.path.exists(data_index_path):
        print("\n  First build detected — bootstrapping circular dependency...")
        # Bootstrap: build sources-final.json without data-index.json
        run("build-sources-overlay.py",
            "Step 9b — Bootstrap sources-final.json (no data-index.json yet)")
        # Now build data-index.json
        run("build-data-index.py",
            "Step 8 — Build data-index.json (7 datasets)")
        # Rebuild sources-final.json with real data-index stats
        run("build-sources-overlay.py",
            "Step 9 — Final sources-final.json (with real stats)")
    else:
        run("build-data-index.py",
            "Step 8 — Build data-index.json (7 datasets)")
        run("build-sources-overlay.py",
            "Step 9 — Build sources-final.json (17 sources)")

    total_elapsed = time.time() - total_start
    print(f"\n{'='*60}")
    print(f"BUILD COMPLETE ({total_elapsed:.1f}s)")
    print(f"{'='*60}")

    # Quick summary
    import json
    with open(data_index_path) as f:
        di = json.load(f)
    stats = di["stats"]
    print(f"  DRI: {stats['total_dri_nutrients']} nutrients, {stats['total_dri_groups']} groups")
    if not dri_only:
        print(f"  Foods: {stats['total_foods']}")
        print(f"  Lab tests: {stats['total_lab_tests']}")
        print(f"  Diagnostic thresholds: {stats['total_diagnostic_thresholds']}")
    print(f"  Fabrication: {stats['fabrication']}, Recalculation: {stats['recalculation']}")
    print(f"  Sources: {', '.join(di['_meta']['sources'])}")


if __name__ == "__main__":
    main()
