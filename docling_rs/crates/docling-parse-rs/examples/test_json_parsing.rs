use docling_parse_rs::types::DoclingParseResult;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let json = std::fs::read_to_string("/tmp/docling_parse_page0.json")?;
    let result: DoclingParseResult = serde_json::from_str(&json)?;

    println!("✓ JSON parsed successfully!");
    println!(
        "  Info: {} pages, file: {}",
        result.info.num_pages, result.info.filename
    );
    println!("  Pages: {}", result.pages.len());

    if let Some(page) = result.pages.first() {
        println!("\nFirst page:");
        println!("  Page number: {}", page.page_number);
        println!(
            "  Dimensions: {}x{}",
            page.original.dimension.width, page.original.dimension.height
        );
        println!("  Line cells: {}", page.original.line_cells.data.len());
        println!("  Word cells: {}", page.original.word_cells.data.len());

        if let Some(cell) = page.original.line_cells.data.first() {
            println!("\nFirst line cell:");
            println!("  Text: {:?}", cell.text);
            println!(
                "  BBox: ({}, {}, {}, {})",
                cell.bbox.x0, cell.bbox.y0, cell.bbox.x1, cell.bbox.y1
            );
            println!("  Font size: {}", cell.font_size);
            println!("  Font name: {}", cell.font_name);
        }
    }

    Ok(())
}
