#!/usr/bin/env python3
"""describe_dri_minerals — valid enum values for DRI minerals query filters.

Returns all valid nutrient names, group keys, and sexes from
dri-minerals-overlay.json.  Call before query_dri_minerals() when
you are unsure of the exact key to use for a filter.
"""

from describe_dri import describe_dri, output_json


def main():
    result = describe_dri("dri-minerals-overlay.json")
    result["dataset"] = "dri_minerals"
    output_json(result)


if __name__ == "__main__":
    main()
