mod common;
/// Basic unit tests for reading order algorithm
///
/// These tests verify that the ReadingOrderPredictor produces reasonable
/// ordering and post-processing (captions, footnotes, merges) by:
/// 1. Loading assembled page elements from baseline
/// 2. Running Rust ReadingOrderPredictor
/// 3. Validating output properties (ordering, caption/footnote assignments)
use common::baseline_loaders::load_all_assembled_elements;
use docling_pdf_ml::pipeline::{ReadingOrderConfig, ReadingOrderPredictor};
use docling_pdf_ml::PageElement;
use std::collections::HashMap;

/// Helper function to create page dimensions map from elements
/// Uses default US Letter size (612x792) since baseline data doesn't include page dimensions
fn create_page_dimensions(elements: &[PageElement]) -> HashMap<usize, (f32, f32)> {
    let mut dimensions = HashMap::new();
    for elem in elements {
        dimensions.entry(elem.page_no()).or_insert((612.0, 792.0));
    }
    dimensions
}

#[test]
fn test_reading_order_arxiv_basic() {
    // 1. Load all assembled elements
    let elements =
        load_all_assembled_elements("arxiv_2206.01062").expect("Failed to load arxiv elements");

    println!("Loaded {} elements from arxiv", elements.len());

    // 2. Create predictor and run reading order
    let config = ReadingOrderConfig::default();
    let predictor = ReadingOrderPredictor::new(config);
    let page_dimensions = create_page_dimensions(&elements);
    let ordered_cids = predictor.predict(&elements, &page_dimensions);

    println!("Reading order produced {} ordered cids", ordered_cids.len());

    // 3. Basic validation
    assert_eq!(
        ordered_cids.len(),
        elements.len(),
        "Reading order should return same number of elements"
    );

    // Check all cids are present (no duplicates or missing)
    let mut seen_cids = std::collections::HashSet::new();
    for (i, &cid) in ordered_cids.iter().enumerate() {
        if !seen_cids.insert(cid) {
            // Find the first occurrence
            let first_pos = ordered_cids.iter().position(|&c| c == cid).unwrap();
            panic!(
                "Duplicate cid {cid} in reading order at positions {first_pos} and {i}. Ordered cids: {ordered_cids:?}"
            );
        }
    }

    // All elements should be in the output
    for elem in &elements {
        let cid = elem.cluster().id;
        assert!(
            seen_cids.contains(&cid),
            "Element cid {cid} missing from reading order"
        );
    }

    println!(
        "✓ Reading order validation passed: {} elements correctly ordered",
        ordered_cids.len()
    );
}

#[test]
fn test_reading_order_code_formula_basic() {
    // 1. Load all assembled elements
    let elements = load_all_assembled_elements("code_and_formula")
        .expect("Failed to load code_and_formula elements");

    println!("Loaded {} elements from code_and_formula", elements.len());

    // 2. Create predictor and run reading order
    let config = ReadingOrderConfig::default();
    let predictor = ReadingOrderPredictor::new(config);
    let page_dimensions = create_page_dimensions(&elements);
    let ordered_cids = predictor.predict(&elements, &page_dimensions);

    println!("Reading order produced {} ordered cids", ordered_cids.len());

    // 3. Basic validation
    assert_eq!(
        ordered_cids.len(),
        elements.len(),
        "Reading order should return same number of elements"
    );

    // Check all cids are unique
    let mut seen_cids = std::collections::HashSet::new();
    for &cid in &ordered_cids {
        assert!(
            seen_cids.insert(cid),
            "Duplicate cid {cid} in reading order"
        );
    }

    // All elements should be in the output
    for elem in &elements {
        let cid = elem.cluster().id;
        assert!(
            seen_cids.contains(&cid),
            "Element cid {cid} missing from reading order"
        );
    }

    println!(
        "✓ Reading order validation passed: {} elements correctly ordered",
        ordered_cids.len()
    );
}

#[test]
fn test_caption_assignments_arxiv() {
    // Load elements
    let elements =
        load_all_assembled_elements("arxiv_2206.01062").expect("Failed to load arxiv elements");

    // Run reading order first
    let config = ReadingOrderConfig::default();
    let predictor = ReadingOrderPredictor::new(config);
    let page_dimensions = create_page_dimensions(&elements);
    let ordered_cids = predictor.predict(&elements, &page_dimensions);

    // Reorder elements
    let mut ordered_elements = Vec::new();
    for &cid in &ordered_cids {
        let elem = elements
            .iter()
            .find(|e| e.cluster().id == cid)
            .unwrap_or_else(|| panic!("Element cid {cid} not found"));
        ordered_elements.push(elem.clone());
    }

    // Test caption assignments
    let to_captions = predictor.predict_to_captions(&ordered_elements);

    println!(
        "Caption assignments: {} elements have captions",
        to_captions.len()
    );

    // Validate: captions should be assigned to tables/pictures/code
    for (parent_cid, caption_cids) in &to_captions {
        println!(
            "  Element {} has {} caption(s)",
            parent_cid,
            caption_cids.len()
        );

        // Find parent element
        let parent = elements
            .iter()
            .find(|e| e.cluster().id == *parent_cid)
            .unwrap_or_else(|| panic!("Parent cid {parent_cid} not found"));

        // Check parent is valid type
        let label_str = format!("{:?}", parent.cluster().label).to_lowercase();
        assert!(
            label_str.contains("table")
                || label_str.contains("picture")
                || label_str.contains("code")
                || label_str.contains("formula"),
            "Caption parent {} has invalid label: {:?}",
            parent_cid,
            parent.cluster().label
        );
    }

    println!("✓ Caption assignments valid");
}

#[test]
fn test_footnote_assignments_arxiv() {
    // Load elements
    let elements =
        load_all_assembled_elements("arxiv_2206.01062").expect("Failed to load arxiv elements");

    // Run reading order first
    let config = ReadingOrderConfig::default();
    let predictor = ReadingOrderPredictor::new(config);
    let page_dimensions = create_page_dimensions(&elements);
    let ordered_cids = predictor.predict(&elements, &page_dimensions);

    // Reorder elements
    let mut ordered_elements = Vec::new();
    for &cid in &ordered_cids {
        let elem = elements
            .iter()
            .find(|e| e.cluster().id == cid)
            .unwrap_or_else(|| panic!("Element cid {cid} not found"));
        ordered_elements.push(elem.clone());
    }

    // Test footnote assignments
    let to_footnotes = predictor.predict_to_footnotes(&ordered_elements);

    println!(
        "Footnote assignments: {} elements have footnotes",
        to_footnotes.len()
    );

    // Validate: footnotes should be assigned to tables/pictures
    for (parent_cid, footnote_cids) in &to_footnotes {
        println!(
            "  Element {} has {} footnote(s)",
            parent_cid,
            footnote_cids.len()
        );

        // Find parent element
        let parent = elements
            .iter()
            .find(|e| e.cluster().id == *parent_cid)
            .unwrap_or_else(|| panic!("Parent cid {parent_cid} not found"));

        // Check parent is valid type
        let label_str = format!("{:?}", parent.cluster().label).to_lowercase();
        assert!(
            label_str.contains("table") || label_str.contains("picture"),
            "Footnote parent {} has invalid label: {:?}",
            parent_cid,
            parent.cluster().label
        );
    }

    println!("✓ Footnote assignments valid");
}

#[test]
fn test_text_merges_arxiv() {
    // Load elements
    let elements =
        load_all_assembled_elements("arxiv_2206.01062").expect("Failed to load arxiv elements");

    // Run reading order first
    let config = ReadingOrderConfig::default();
    let predictor = ReadingOrderPredictor::new(config);
    let page_dimensions = create_page_dimensions(&elements);
    let ordered_cids = predictor.predict(&elements, &page_dimensions);

    // Reorder elements
    let mut ordered_elements = Vec::new();
    for &cid in &ordered_cids {
        let elem = elements
            .iter()
            .find(|e| e.cluster().id == cid)
            .unwrap_or_else(|| panic!("Element cid {cid} not found"));
        ordered_elements.push(elem.clone());
    }

    // Test text merges
    let merges = predictor.predict_merges(&ordered_elements);

    println!("Text merges: {} merge operations", merges.len());

    // Validate: merges should connect text elements
    for (source_cid, target_cids) in &merges {
        println!(
            "  Element {} merges with {} element(s)",
            source_cid,
            target_cids.len()
        );

        // Find source element
        let source = elements
            .iter()
            .find(|e| e.cluster().id == *source_cid)
            .unwrap_or_else(|| panic!("Source cid {source_cid} not found"));

        // Check source is text
        let source_label_str = format!("{:?}", source.cluster().label).to_lowercase();
        assert!(
            source_label_str.contains("text"),
            "Merge source {} has invalid label: {:?}",
            source_cid,
            source.cluster().label
        );

        // Check all targets are text
        for &target_cid in target_cids {
            let target = elements
                .iter()
                .find(|e| e.cluster().id == target_cid)
                .unwrap_or_else(|| panic!("Target cid {target_cid} not found"));
            let target_label_str = format!("{:?}", target.cluster().label).to_lowercase();
            assert!(
                target_label_str.contains("text"),
                "Merge target {} has invalid label: {:?}",
                target_cid,
                target.cluster().label
            );
        }
    }

    println!("✓ Text merge assignments valid");
}
