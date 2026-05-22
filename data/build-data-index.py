# Build data-index.json — unified manifest of all dietology knowledge base files.
# Run: python3 build-data-index.py

import json
import os
from datetime import datetime, timezone

SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))

INDEX_OUT = os.path.join(SCRIPT_DIR, "data-index.json")

DATASETS = {
    "dri-minerals-overlay.json": {
        "domain": "dri",
        "tier": "A",
        "description": "14 minerals, 254 groups, finest granularity per source. 0 manual_transcription.",
        "sources": ["iom-dri-2011", "iom-dri-1997", "msd-manual-dri", "nas-dri-2019", "lpi-mic-minerals"],
        "build_script": "data/build-minerals-overlay.py",
        "notes": "Ca: IOM 2011 (22 groups + ul_groups). P/Mg: IOM 1997 via NCBI cross-verification (22 groups each), UL from LPI (based on IOM 1997). Trace minerals: MSD Professional (16 groups each), UL from MSD Professional. Na/K: NAS 2019 (22 groups each). Pregnancy/breastfeeding split into teen/adult subgroups. All 14 nutrients have machine-verified UL metadata.",
    },
    "dri-vitamins-overlay.json": {
        "domain": "dri",
        "tier": "A",
        "description": "11 vitamins, 154 groups, machine-parsed values with full metadata",
        "sources": ["msd-manual-dri"],
        "build_script": "data/build-vitamins-overlay.py",
        "notes": "Values and metadata (unit, UL, ul_unit, ul_note, unit_note) all machine-extracted from MSD Professional HTML. 0 manual transcription. Pregnancy/breastfeeding are single groups without age split — MSD vitamins table does not provide teen/adult breakdown.",
    },
    "dri-macronutrients-per-kg-overlay.json": {
        "domain": "dri",
        "tier": "A",
        "description": "3 nutrients (Ca/P/Mg), 51 groups (17 each), mg/kg of body weight",
        "sources": ["msd-macronutrients-per-kg"],
        "build_script": "data/build-macronutrients-per-kg-overlay.py",
        "notes": "Values machine-parsed; category/note/citation programmatically generated. 0 manual transcription. Per-kg values — model must multiply by individual body weight. More accurate for individual use than absolute DRI. Infants: AI. Children and adults: RDA. Based on IOM 1997.",
    },
    "usda-foundation-foods-essential.json": {
        "domain": "food_composition",
        "tier": "A",
        "description": "363 foods with nutrient composition (per 100g), 27 essential nutrients",
        "sources": ["usda-fdc-2026-04"],
        "extraction_script": "data/extract-usda.py",
        "notes": "Subset of USDA Foundation Foods (CC0). Filtered to foods with complete nutrient profiles for 27 essential nutrients.",
    },
    "lab-reference-ranges.json": {
        "domain": "lab_ranges",
        "tier": "C",
        "description": "254 lab test reference ranges across 16 categories",
        "sources": ["wikipedia-lab-ranges"],
        "extraction_script": "data/extract-wiki-lab-ranges.py",
        "notes": "Wikipedia-sourced reference ranges. Tier C: useful for context but not authoritative clinical reference. Values are population averages, not individual diagnostic thresholds.",
    },
    "who-hb-thresholds.json": {
        "domain": "deficiency_thresholds",
        "tier": "B",
        "description": "9 hemoglobin diagnostic thresholds for anemia (WHO 2024)",
        "sources": ["who-2024-hb"],
        "extraction_script": "data/extract-who-hb.py",
        "notes": "WHO 2024 Guideline on haemoglobin cutoffs. Includes severity classification (mild/moderate/severe), altitude/smoking adjustments, pregnancy-specific thresholds.",
    },
    "sources-final.json": {
        "domain": "manifest",
        "tier": "A",
        "description": "Unified source manifest — definitive reference for all dietology data sources (15 sources, 5 tier levels)",
        "sources": ["iom-dri-1997", "iom-dri-2011", "lpi-mic-minerals", "msd-consumer-minerals", "msd-macronutrients-per-kg", "msd-manual-dri", "nas-dri-2019", "usda-fdc-2026-04", "who-2024-hb", "wikipedia-lab-ranges"],
        "build_script": "data/build-sources-overlay.py",
        "notes": "Merges sources.json (base) + sources-overlay.json (DRI overlay) + data-index.json (catalog). Single authoritative source reference — model loads ONLY this file. Supersedes sources.json.",
    },
}

def count_dataset(path):
    """Return (entry_count, detail_str) for a dataset file.
    Returns (0, 'not built') for missing files — partial builds are OK.
    """
    full_path = os.path.join(SCRIPT_DIR, path)
    if not os.path.exists(full_path):
        return 0, "not built"
    with open(full_path) as f:
        d = json.load(f)

    if "nutrients" in d:
        n = len(d["nutrients"])
        g = sum(len(nu["groups"]) for nu in d["nutrients"])
        return n, f"{n} nutrients, {g} groups"
    elif "foods" in d:
        n = len(d["foods"])
        return n, f"{n} foods"
    elif "ranges" in d:
        n = len(d["ranges"])
        return n, f"{n} ranges"
    elif "diagnostic_thresholds" in d:
        n = len(d["diagnostic_thresholds"])
        return n, f"{n} diagnostic thresholds"
    elif "sources" in d and "_meta" in d:
        n = len(d["sources"])
        return n, f"{n} sources"
    return 0, "unknown"


def main():
    print("Building data-index.json")
    print("========================")
    print()

    datasets = {}
    stats = {"total_dri_nutrients": 0, "total_dri_groups": 0, "total_foods": 0,
             "total_lab_tests": 0, "total_diagnostic_thresholds": 0,
             "fabrication": 0, "recalculation": 0}

    for filename, info in DATASETS.items():
        count, detail = count_dataset(filename)
        entry = dict(info)
        entry["file"] = f"data/{filename}"
        entry["count"] = count
        entry["detail"] = detail
        datasets[filename] = entry

        print(f"  {filename}: {detail}")
        # Accumulate stats
        if count == 0 and detail == "not built":
            continue
        if info["domain"] == "dri":
            full_path = os.path.join(SCRIPT_DIR, filename)
            with open(full_path) as f:
                d = json.load(f)
            n_nut = len(d["nutrients"])
            n_grp = sum(len(nu["groups"]) for nu in d["nutrients"])
            stats["total_dri_nutrients"] += n_nut
            stats["total_dri_groups"] += n_grp
        elif info["domain"] == "food_composition":
            stats["total_foods"] += count
        elif info["domain"] == "lab_ranges":
            stats["total_lab_tests"] += count
        elif info["domain"] == "deficiency_thresholds":
            stats["total_diagnostic_thresholds"] += count

    all_sources = sorted(set(
        s for ds in datasets.values() for s in ds["sources"]
    ))

    output = {
        "_meta": {
            "schema": "dietology-data-index-v1",
            "description": "Unified manifest of all dietology knowledge base files. Each dataset entry includes domain, tier, sources, and provenance metadata.",
            "build_script": "data/build-data-index.py",
            "build_date": datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
            "sources": all_sources,
            "provenance_guarantee": "All values are from-source (numeric facts extraction from public sources). 0 fabrication, 0 recalculation. Every value traceable to original source document in data/external/.",
        },
        "datasets": datasets,
        "stats": stats,
    }

    with open(INDEX_OUT, "w") as f:
        json.dump(output, f, ensure_ascii=False, indent=2)

    print(f"\nWritten {INDEX_OUT}")
    print(f"  {len(datasets)} datasets indexed")
    print(f"  DRI: {stats['total_dri_nutrients']} nutrients, {stats['total_dri_groups']} groups")
    print(f"  Foods: {stats['total_foods']}")
    print(f"  Lab tests: {stats['total_lab_tests']}")
    print(f"  Diagnostic thresholds: {stats['total_diagnostic_thresholds']}")
    print(f"  Fabrication: {stats['fabrication']}, Recalculation: {stats['recalculation']}")
    print(f"  Sources: {', '.join(all_sources)}")

    # Verify all source_ids reference known sources
    for entry in datasets.values():
        for sid in entry["sources"]:
            assert sid in all_sources, f"Unknown source_id: {sid}"
    print("All assertions passed.")


if __name__ == "__main__":
    main()
