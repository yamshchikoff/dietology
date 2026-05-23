use dietology_lib::models::{dri, datasets, manifest};

// ============ DRI overlay tests ============

#[test]
fn test_dri_group_deserialization() {
    let json = r#"{
        "group": "male_19_30yr",
        "sex": "male",
        "age_range": "19-30",
        "value": 1000.0,
        "type": "RDA"
    }"#;
    let g: dri::DriGroup = serde_json::from_str(json).unwrap();
    assert_eq!(g.group, "male_19_30yr");
    assert_eq!(g.sex, Some("male".to_string()));
    assert_eq!(g.value, 1000.0);
    assert_eq!(g.dri_type, "RDA");
}

#[test]
fn test_dri_nutrient_deserialization() {
    let json = r#"{
        "name": "Calcium",
        "unit": "mg",
        "category": "macromineral",
        "source_id": "iom-dri-2011",
        "source_urls": ["https://example.com"],
        "groups": [
            {"group": "male_19_30yr", "sex": "male", "age_range": "19-30", "value": 1000.0, "type": "RDA"}
        ],
        "ul": 2500,
        "ul_unit": "mg",
        "ul_note": "Tolerable upper intake level",
        "note": "Important for bone health"
    }"#;
    let n: dri::DriNutrient = serde_json::from_str(json).unwrap();
    assert_eq!(n.name, "Calcium");
    assert_eq!(n.unit, "mg");
    assert_eq!(n.category, Some("macromineral".to_string()));
    assert_eq!(n.groups.len(), 1);
    assert_eq!(n.ul, Some(2500.0));
    assert_eq!(n.ul_unit, Some("mg".to_string()));
}

#[test]
fn test_dri_overlay_deserialization() {
    let json = r#"{
        "nutrients": [
            {
                "name": "Calcium", "unit": "mg", "category": "macromineral",
                "source_id": "iom-dri-2011",
                "source_urls": ["https://example.com"],
                "groups": [
                    {"group": "male_19_30yr", "sex": "male", "age_range": "19-30", "value": 1000.0, "type": "RDA"}
                ]
            }
        ]
    }"#;
    let overlay: dri::DriOverlay = serde_json::from_str(json).unwrap();
    assert_eq!(overlay.nutrients.len(), 1);
    assert_eq!(overlay.nutrients[0].name, "Calcium");
}

#[test]
fn test_dri_group_optional_fields_default_to_none() {
    let json = r#"{
        "group": "infants_0_6mo",
        "sex": "any",
        "age_range": "0-6 mo",
        "value": 200,
        "type": "AI"
    }"#;
    let g: dri::DriGroup = serde_json::from_str(json).unwrap();
    assert_eq!(g.ul, None);
    assert_eq!(g.ul_note, None);
    assert_eq!(g.note, None);
}

#[test]
fn test_dri_invalid_json_returns_error() {
    let json = r#"{"not_a_valid_dri": true}"#;
    let result: Result<dri::DriGroup, _> = serde_json::from_str(json);
    assert!(result.is_err());
}

// ============ USDA Foods tests ============

#[test]
fn test_usda_food_deserialization() {
    let json = r#"{
        "name": "Hummus, commercial",
        "category": "Legumes and Legume Products",
        "fdcId": 321358,
        "nutrients": {
            "Vitamin C, total ascorbic acid": {"amount": 0.0, "unit": "mg"}
        }
    }"#;
    let food: datasets::Food = serde_json::from_str(json).unwrap();
    assert_eq!(food.name, "Hummus, commercial");
    assert_eq!(food.fdc_id, 321358);
    assert_eq!(food.nutrients.len(), 1);
}

#[test]
fn test_usda_foods_deserialization() {
    let json = r#"{
        "foods": [
            {"name": "Apple", "category": "Fruits", "fdcId": 1, "nutrients": {}}
        ]
    }"#;
    let foods: datasets::UsdaFoods = serde_json::from_str(json).unwrap();
    assert_eq!(foods.foods.len(), 1);
}

// ============ WHO Hb Thresholds tests ============

#[test]
fn test_hb_diagnostic_threshold_deserialization() {
    let json = r#"{
        "group": "children_6_23_months",
        "sex": "any",
        "pregnant": false,
        "hb_cutoff_g_per_l": 105,
        "hb_cutoff_g_per_dl": 10.5,
        "note": "Lowered from 110 g/L"
    }"#;
    let dt: datasets::HbDiagnosticThreshold = serde_json::from_str(json).unwrap();
    assert_eq!(dt.group, "children_6_23_months");
    assert_eq!(dt.hb_cutoff_g_per_l, 105.0);
    assert!(!dt.pregnant);
}

#[test]
fn test_who_hb_thresholds_deserialization() {
    let json = r#"{
        "diagnostic_thresholds": [],
        "severity_classification": []
    }"#;
    let hb: datasets::WhoHbThresholds = serde_json::from_str(json).unwrap();
    assert_eq!(hb.diagnostic_thresholds.len(), 0);
}

// ============ WHO GHO Epidemiology tests ============

#[test]
fn test_epi_record_deserialization() {
    let json = r#"{
        "country_code": "AFG",
        "year": 1990,
        "value": 56.5,
        "low": 47.4,
        "high": 65.2,
        "parent_region": null,
        "parent_region_code": null,
        "sex": "SEX_FMLE"
    }"#;
    let r: datasets::EpiRecord = serde_json::from_str(json).unwrap();
    assert_eq!(r.country_code, "AFG");
    assert_eq!(r.year, 1990);
    assert!(r.parent_region.is_none());
}

#[test]
fn test_epi_data_deserialization() {
    let json = r#"{"data": []}"#;
    let epi: datasets::WhoEpiData = serde_json::from_str(json).unwrap();
    assert_eq!(epi.data.len(), 0);
}

// ============ Lab Reference Ranges tests ============

#[test]
fn test_lab_range_deserialization() {
    let json = r#"{
        "category": "ions_and_trace_metals",
        "test": "total serum iron - female",
        "type": "26, 50",
        "low": null,
        "high": "170",
        "unit": "µg/dL"
    }"#;
    let lr: datasets::LabRange = serde_json::from_str(json).unwrap();
    assert_eq!(lr.category, "ions_and_trace_metals");
    assert_eq!(lr.unit, "µg/dL");
    assert!(lr.low.is_none());
}

#[test]
fn test_lab_ranges_deserialization() {
    let json = r#"{"ranges": []}"#;
    let lr: datasets::LabReferenceRanges = serde_json::from_str(json).unwrap();
    assert_eq!(lr.ranges.len(), 0);
}

// ============ Manifest tests ============

#[test]
fn test_data_index_deserialization() {
    let json = r#"{
        "datasets": {
            "test.json": {
                "domain": "dri",
                "tier": "A",
                "description": "Test dataset",
                "sources": ["test-source"],
                "file": "data/test.json",
                "count": 14,
                "detail": "14 items"
            }
        },
        "stats": {
            "total_dri_nutrients": 28,
            "total_dri_groups": 459,
            "total_foods": 363,
            "total_lab_tests": 254,
            "total_diagnostic_thresholds": 9,
            "total_epi_records": 83320,
            "fabrication": 0,
            "recalculation": 0
        }
    }"#;
    let di: manifest::DataIndex = serde_json::from_str(json).unwrap();
    assert_eq!(di.datasets.len(), 1);
    assert_eq!(di.stats.total_dri_nutrients, 28);
    assert_eq!(di.stats.fabrication, 0);
}

#[test]
fn test_sources_final_deserialization() {
    let json = r#"{
        "schema_version": "1.0",
        "description": "Test",
        "sources": {},
        "stats": {"total": 17}
    }"#;
    let sf: manifest::SourcesFinal = serde_json::from_str(json).unwrap();
    assert_eq!(sf.schema_version, "1.0");
}
