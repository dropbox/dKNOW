/// Baseline data loading utilities for tests.
///
/// This module provides functions to load baseline data from disk for validation tests.
/// These functions are NOT part of the production library and are only used in tests.
use docling_pdf_ml::pipeline::{
    BoundingBox, Cluster, CoordOrigin, DocItemLabel, PageElement, TableCell, TableElement,
};
use ndarray::ArrayD;
use npyz::{NpyFile, WriterBuilder};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::path::Path;

/// Load JSON data from a file
pub fn load_json<T: for<'de> Deserialize<'de>>(
    path: &Path,
) -> Result<T, Box<dyn std::error::Error>> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let data = serde_json::from_reader(reader)?;
    Ok(data)
}

/// Load numpy array from .npy file (f32)
///
/// CRITICAL: Handles both C-order (row-major) and Fortran-order (column-major) numpy files.
/// `npyz::NpyFile.into_vec()` returns data in the file's native order, so we must check
/// the order flag and transpose if needed.
pub fn load_numpy(path: &Path) -> Result<ArrayD<f32>, Box<dyn std::error::Error>> {
    let file = File::open(path)?;
    let npy = NpyFile::new(file)?;
    let shape: Vec<usize> = npy.shape().iter().map(|&x| x as usize).collect();
    let fortran_order = matches!(npy.order(), npyz::Order::Fortran);
    let data: Vec<f32> = npy.into_vec()?;

    if fortran_order {
        // Fortran order: data is column-major, need to transpose
        // Load with reversed shape, then transpose back to original shape
        let reversed_shape: Vec<usize> = shape.iter().copied().rev().collect();
        let temp_arr = ArrayD::from_shape_vec(ndarray::IxDyn(&reversed_shape), data)?;

        // Transpose: reverse all axes to convert from Fortran to C layout
        let ndims = shape.len();
        let axes: Vec<usize> = (0..ndims).rev().collect();
        Ok(temp_arr.permuted_axes(axes))
    } else {
        // C order: data is row-major, standard loading
        Ok(ArrayD::from_shape_vec(ndarray::IxDyn(&shape), data)?)
    }
}

/// Load numpy array from .npy file (uint8) and convert to f32
pub fn load_numpy_u8_as_f32(path: &Path) -> Result<ArrayD<f32>, Box<dyn std::error::Error>> {
    let file = File::open(path)?;
    let npy = NpyFile::new(file)?;
    let shape: Vec<usize> = npy.shape().iter().map(|&x| x as usize).collect();
    let data: Vec<u8> = npy.into_vec()?;
    let data_f32: Vec<f32> = data.iter().map(|&x| x as f32).collect();
    Ok(ArrayD::from_shape_vec(ndarray::IxDyn(&shape), data_f32)?)
}

/// Load numpy array from .npy file as u8 (for images)
pub fn load_numpy_u8(path: &Path) -> Result<ArrayD<u8>, Box<dyn std::error::Error>> {
    let file = File::open(path)?;
    let npy = NpyFile::new(file)?;
    let shape: Vec<usize> = npy.shape().iter().map(|&x| x as usize).collect();
    let data: Vec<u8> = npy.into_vec()?;
    Ok(ArrayD::from_shape_vec(ndarray::IxDyn(&shape), data)?)
}

/// Save ndarray to .npy file in C-order (row-major)
pub fn save_numpy(arr: &ArrayD<f32>, path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let file = File::create(path)?;
    let mut buf_writer = BufWriter::new(file);

    // Convert shape to u64
    let shape: Vec<u64> = arr.shape().iter().map(|&x| x as u64).collect();

    // Collect data into a Vec in C-order
    let data: Vec<f32> = arr.iter().copied().collect();

    // Write using npyz WriteOptions
    {
        let mut npy = npyz::WriteOptions::<f32>::new()
            .default_dtype()
            .shape(&shape)
            .writer(&mut buf_writer)
            .begin_nd()?;
        npy.extend(data)?;
        npy.finish()?;
    }

    Ok(())
}

/// Baseline table JSON structure (matches Python output)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaselineTableStructure {
    pub num_rows: usize,
    pub num_cols: usize,
    #[serde(alias = "cells")] // Support old format
    pub table_cells: Vec<BaselineTableCell>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaselineTableCell {
    pub row: Option<i32>, // For old format (TableFormer raw): -1 = header cell
    pub col: Option<i32>, // For old format (TableFormer raw): -1 = header cell
    pub row_span: usize,
    pub col_span: usize,
    pub bbox: BaselineBBox,
    pub text: String,
    // New format (Python final) - computed positions
    pub start_row_offset_idx: Option<usize>,
    pub end_row_offset_idx: Option<usize>,
    pub start_col_offset_idx: Option<usize>,
    pub end_col_offset_idx: Option<usize>,
    pub column_header: Option<bool>,
    pub row_header: Option<bool>,
    pub row_section: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaselineBBox {
    pub l: f64,
    pub t: f64,
    pub r: f64,
    pub b: f64,
}

/// Load table structures for a specific page and map to cluster IDs
pub fn load_table_structures(
    pdf_name: &str,
    page_no: usize,
    table_clusters: &[Cluster],
) -> Result<HashMap<usize, TableElement>, Box<dyn std::error::Error>> {
    let mut table_map = HashMap::new();

    // Sort table clusters by ID to match with table_0, table_1, etc.
    let mut sorted_clusters: Vec<&Cluster> = table_clusters.iter().collect();
    sorted_clusters.sort_by_key(|c| c.id);

    for (i, cluster) in sorted_clusters.iter().enumerate() {
        // Try Python final format first (with correct positions)
        let python_final_path = Path::new("baseline_data_python_final")
            .join(pdf_name)
            .join(format!("page_{page_no}"))
            .join(format!("table_{i}_python_final.json"));

        // Fall back to old TableFormer format
        let old_path = Path::new("baseline_data")
            .join(pdf_name)
            .join(format!("page_{page_no}"))
            .join("table")
            .join(format!("table_{i}"))
            .join("output_table_structure.json");

        let table_path = if python_final_path.exists() {
            python_final_path
        } else if old_path.exists() {
            old_path
        } else {
            // If neither exists, skip (page may have table labels but no structure)
            continue;
        };

        let file = File::open(&table_path)?;
        let baseline: BaselineTableStructure = serde_json::from_reader(BufReader::new(file))?;

        // Convert to TableElement
        let table_element = convert_to_table_element(&baseline, cluster, page_no);
        table_map.insert(cluster.id, table_element);
    }

    Ok(table_map)
}

fn convert_to_table_element(
    baseline: &BaselineTableStructure,
    cluster: &Cluster,
    page_no: usize,
) -> TableElement {
    let table_cells: Vec<TableCell> = baseline
        .table_cells
        .iter()
        .map(|cell| {
            // Use Python-computed positions if available (Python final format)
            // Otherwise fall back to old format (TableFormer raw)
            let (start_row, end_row, start_col, end_col) =
                if let (Some(sr), Some(er), Some(sc), Some(ec)) = (
                    cell.start_row_offset_idx,
                    cell.end_row_offset_idx,
                    cell.start_col_offset_idx,
                    cell.end_col_offset_idx,
                ) {
                    // Python final format - use computed positions
                    (sr, er, sc, ec)
                } else {
                    // Old format (TableFormer raw) - convert row/col
                    // Handle -1 values (header cells in Python) - convert to 0
                    let row = cell.row.unwrap_or(-1);
                    let col = cell.col.unwrap_or(-1);
                    let start_row = if row < 0 { 0 } else { row as usize };
                    let start_col = if col < 0 { 0 } else { col as usize };
                    let end_row = start_row + cell.row_span.saturating_sub(1);
                    let end_col = start_col + cell.col_span.saturating_sub(1);
                    (start_row, end_row, start_col, end_col)
                };

            TableCell {
                text: cell.text.clone(),
                bbox: BoundingBox {
                    l: cell.bbox.l as f32,
                    t: cell.bbox.t as f32,
                    r: cell.bbox.r as f32,
                    b: cell.bbox.b as f32,
                    coord_origin: CoordOrigin::TopLeft,
                },
                row_span: cell.row_span,
                col_span: cell.col_span,
                start_row_offset_idx: start_row,
                end_row_offset_idx: end_row,
                start_col_offset_idx: start_col,
                end_col_offset_idx: end_col,
                column_header: cell.column_header.unwrap_or(false),
                row_header: cell.row_header.unwrap_or(false),
                from_ocr: false,
                confidence: None,
            }
        })
        .collect();

    TableElement {
        label: DocItemLabel::Table,
        id: cluster.id,
        page_no,
        text: None,
        cluster: cluster.clone(),
        otsl_seq: Vec::new(),
        num_rows: baseline.num_rows,
        num_cols: baseline.num_cols,
        table_cells,
        captions: Vec::new(),
        footnotes: Vec::new(),
    }
}

/// Page size structure from preprocessing baseline
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageSize {
    pub width: f64,
    pub height: f64,
}

/// Load page size from preprocessing baseline
pub fn load_page_size(
    pdf_name: &str,
    page_no: usize,
) -> Result<PageSize, Box<dyn std::error::Error>> {
    let path = Path::new("baseline_data")
        .join(pdf_name)
        .join(format!("page_{page_no}"))
        .join("preprocessing")
        .join("page_size.json");

    let file = File::open(&path)?;
    let page_size: PageSize = serde_json::from_reader(BufReader::new(file))?;
    Ok(page_size)
}

/// Baseline assembly data structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssemblyBaseline {
    pub elements: Vec<PageElement>,
    pub body: Vec<PageElement>,
    pub headers: Vec<PageElement>,
}

/// Load assembly baseline for a specific page
pub fn load_assembly_baseline(
    pdf_name: &str,
    page_no: usize,
) -> Result<AssemblyBaseline, Box<dyn std::error::Error>> {
    let base_path = Path::new("baseline_data")
        .join(pdf_name)
        .join(format!("page_{page_no}"))
        .join("assembly");

    // Load elements
    let elements_path = base_path.join("elements.json");
    let elements_file = File::open(&elements_path)?;
    let elements: Vec<PageElement> = serde_json::from_reader(BufReader::new(elements_file))?;

    // Load body
    let body_path = base_path.join("body.json");
    let body_file = File::open(&body_path)?;
    let body: Vec<PageElement> = serde_json::from_reader(BufReader::new(body_file))?;

    // Load headers
    let headers_path = base_path.join("headers.json");
    let headers_file = File::open(&headers_path)?;
    let headers: Vec<PageElement> = serde_json::from_reader(BufReader::new(headers_file))?;

    Ok(AssemblyBaseline {
        elements,
        body,
        headers,
    })
}

/// Extract unique clusters from assembly baseline elements
/// This is useful for reconstructing the input layout prediction
pub fn extract_clusters_from_baseline(baseline: &AssemblyBaseline) -> Vec<Cluster> {
    let mut clusters = Vec::new();
    let mut seen_ids = std::collections::HashSet::new();

    for element in &baseline.elements {
        let cluster = element.cluster();
        if seen_ids.insert(cluster.id) {
            clusters.push(cluster.clone());
        }
    }

    clusters.sort_by_key(|c| c.id);
    clusters
}

/// Load layout clusters WITH cells for assembly testing
/// This loads the full postprocessed clusters from layout stage
pub fn load_layout_stage34_clusters_after_assembly(
    pdf_name: &str,
    page_no: usize,
) -> Result<Vec<Cluster>, Box<dyn std::error::Error>> {
    let path = Path::new("baseline_data")
        .join(pdf_name)
        .join(format!("page_{page_no}"))
        .join("layout")
        .join("stage34_clusters_after_assembly.json");

    let file = File::open(&path)?;
    let clusters: Vec<Cluster> = serde_json::from_reader(BufReader::new(file))?;
    Ok(clusters)
}

/// Load layout clusters with cell information for decision accuracy testing
///
/// Alias for `load_layout_stage34_clusters_after_assembly` - loads the postprocessed
/// clusters from the layout stage which includes cell bounding boxes and labels.
///
/// # Arguments
/// * `pdf_name` - Name of the PDF in baseline_data directory (e.g., "arxiv_2206.01062")
/// * `page_no` - Page number (0-indexed)
///
/// # Returns
/// Vector of Cluster structs with full cell information
pub fn load_layout_clusters_with_cells(
    pdf_name: &str,
    page_no: usize,
) -> Result<Vec<Cluster>, Box<dyn std::error::Error>> {
    load_layout_stage34_clusters_after_assembly(pdf_name, page_no)
}

/// Load all assembled elements across all pages for a PDF
/// Returns a flat list of all elements in document order (by page)
/// NOTE: Renumbers cluster IDs to be globally unique (not page-local)
pub fn load_all_assembled_elements(
    pdf_name: &str,
) -> Result<Vec<PageElement>, Box<dyn std::error::Error>> {
    let base_path = Path::new("baseline_data").join(pdf_name);
    let mut all_elements = Vec::new();
    let mut id_offset = 0;

    // Try to load pages 0..N until we hit a missing page
    for page_no in 0..1000 {
        let page_path = base_path
            .join(format!("page_{page_no}"))
            .join("assembly")
            .join("elements.json");

        if !page_path.exists() {
            break;
        }

        let file = File::open(&page_path)?;
        let mut elements: Vec<PageElement> = serde_json::from_reader(BufReader::new(file))?;

        // Renumber cluster IDs to be globally unique
        for elem in &mut elements {
            elem.renumber_cluster_id(id_offset);
        }

        // Update offset for next page
        if let Some(max_id) = elements.iter().map(|e| e.cluster().id).max() {
            id_offset = max_id + 1;
        }

        all_elements.extend(elements);
    }

    if all_elements.is_empty() {
        return Err(format!("No assembled pages found for PDF: {pdf_name}").into());
    }

    Ok(all_elements)
}
