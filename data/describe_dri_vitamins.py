#!/usr/bin/env python3
"""describe_dri_vitamins — valid enum values for DRI vitamins query filters.

Returns all valid nutrient names, group keys, and sexes from
dri-vitamins-overlay.json.  Call before query_dri_vitamins() when
you are unsure of the exact key to use for a filter.
"""

from describe_dri import describe_dri, output_json


def main():
    result = describe_dri("dri-vitamins-overlay.json")
    result["dataset"] = "dri_vitamins"
    output_json(result)


if __name__ == "__main__":
    main()
