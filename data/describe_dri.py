#!/usr/bin/env python3
"""Shared describe logic for DRI overlay JSON files.

All three DRI datasets (minerals, vitamins, per-kg) share an identical
top-level skeleton: {"_meta": {...}, "nutrients": [...]}.  This module
provides a single function that reads any of them and returns the valid
enum values for query filters.
"""

import json
import os
import sys

SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))


def load_json(filename):
    path = os.path.join(SCRIPT_DIR, filename)
    with open(path) as f:
        return json.load(f)


def describe_dri(json_path):
    """Read a DRI overlay JSON, return {nutrients, groups, sexes, total_groups}.

    Args:
        json_path: Path to the overlay JSON file (relative to SCRIPT_DIR).

    Returns:
        dict with keys: dataset, nutrients, groups, sexes, total_groups.
    """
    data = load_json(json_path)
    nutrients = [n["name"] for n in data["nutrients"]]
    groups = sorted(set(g["group"] for n in data["nutrients"] for g in n["groups"]))
    sexes = sorted(set(g["sex"] for n in data["nutrients"] for g in n["groups"]))
    total_groups = sum(len(n["groups"]) for n in data["nutrients"])
    return {
        "nutrients": nutrients,
        "groups": groups,
        "sexes": sexes,
        "total_groups": total_groups,
    }


def output_json(result):
    json.dump(result, sys.stdout, indent=2, ensure_ascii=False)
    sys.stdout.write("\n")
