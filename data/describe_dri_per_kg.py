#!/usr/bin/env python3
"""describe_dri_per_kg — valid enum values for DRI per-kg query filters.

Returns all valid nutrient names, group keys, and the fixed unit ("mg/kg")
from dri-macronutrients-per-kg-overlay.json.  Also surfaces the critical
convention note from _meta.

Call before query_dri_per_kg() when you are unsure of the exact key to
use for a filter.  Remember: ALL values are in mg/kg — multiply by the
individual's actual body weight to get absolute daily intake.
"""

from describe_dri import describe_dri, output_json


def main():
    result = describe_dri("dri-macronutrients-per-kg-overlay.json")
    result["dataset"] = "dri_per_kg"
    result["unit"] = "mg/kg"
    output_json(result)


if __name__ == "__main__":
    main()
