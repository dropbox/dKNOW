#![cfg(feature = "opencv-preprocessing")]
/// Test OpenCV resize vs image crate resize
use ndarray::Array3;
use npyz::NpyFile;
use opencv::prelude::{MatTrait, MatTraitConst};
use std::fs::File;
use std::io::{BufReader, Write as IoWrite};
use std::path::PathBuf;

fn load_npy_u8(path: &PathBuf) -> Result<Array3<u8>, Box<dyn std::error::Error>> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let npy = NpyFile::new(reader)?;

    let shape = npy.shape().to_vec();
    if shape.len() != 3 {
        return Err(format!("Expected 3D array, got {:?}", shape).into());
    }

    let data: Vec<u8> = npy.into_vec()?;
    Ok(ndarray::Array3::from_shape_vec(
        (shape[0] as usize, shape[1] as usize, shape[2] as usize),
        data,
    )?)
}

#[test]
fn test_opencv_vs_image_crate_resize() {
    println!("\n=== OpenCV vs image crate Resize ===");

    // Load raw image
    let base_path = PathBuf::from("baseline_data/arxiv_2206.01062/page_0/layout");
    let image_path = base_path.join("input_page_image.npy");
    let raw_image = load_npy_u8(&image_path).expect("Failed to load raw image");

    let (height, width, channels) = raw_image.dim();
    println!("Raw image: {}x{}x{}", height, width, channels);

    // Test 1: image crate (current implementation)
    use image::{DynamicImage, ImageBuffer, Rgb};
    let mut img_buffer = ImageBuffer::<Rgb<u8>, Vec<u8>>::new(width as u32, height as u32);
    for y in 0..height {
        for x in 0..width {
            let pixel = [
                raw_image[[y, x, 0]],
                raw_image[[y, x, 1]],
                raw_image[[y, x, 2]],
            ];
            img_buffer.put_pixel(x as u32, y as u32, Rgb(pixel));
        }
    }
    let dynamic_img = DynamicImage::ImageRgb8(img_buffer);
    let resized_image_crate =
        dynamic_img.resize_exact(640, 640, image::imageops::FilterType::Triangle);
    let resized_image_crate_rgb = resized_image_crate.to_rgb8();

    // Test 2: OpenCV resize
    use opencv::core::{Mat, Size, CV_8UC3};
    use opencv::imgproc;

    // Convert ndarray to OpenCV Mat
    let mut mat = Mat::new_rows_cols_with_default(
        height as i32,
        width as i32,
        CV_8UC3,
        opencv::core::Scalar::all(0.0),
    )
    .expect("Failed to create Mat");

    for y in 0..height {
        for x in 0..width {
            let pixel = [
                raw_image[[y, x, 0]],
                raw_image[[y, x, 1]],
                raw_image[[y, x, 2]],
            ];
            unsafe {
                let ptr = mat
                    .ptr_2d_mut(y as i32, x as i32)
                    .expect("Failed to get pixel pointer");
                *ptr.add(0) = pixel[0];
                *ptr.add(1) = pixel[1];
                *ptr.add(2) = pixel[2];
            }
        }
    }

    // Resize with OpenCV using LINEAR interpolation (same as PIL BILINEAR)
    let mut resized_opencv = Mat::default();
    imgproc::resize(
        &mat,
        &mut resized_opencv,
        Size::new(640, 640),
        0.0,
        0.0,
        imgproc::INTER_LINEAR,
    )
    .expect("Failed to resize with OpenCV");

    println!("✓ Resized with image crate and OpenCV");

    // Compare at a specific pixel
    let test_y = 73;
    let test_x = 295;

    let image_crate_pixel = resized_image_crate_rgb.get_pixel(test_x, test_y);
    let opencv_pixel = unsafe {
        let ptr = resized_opencv
            .ptr_2d(test_y as i32, test_x as i32)
            .expect("Failed to get OpenCV pixel");
        [*ptr.add(0), *ptr.add(1), *ptr.add(2)]
    };

    println!("\nPixel at ({}, {}):", test_y, test_x);
    println!(
        "  image crate: [{}, {}, {}]",
        image_crate_pixel[0], image_crate_pixel[1], image_crate_pixel[2]
    );
    println!(
        "  OpenCV:      [{}, {}, {}]",
        opencv_pixel[0], opencv_pixel[1], opencv_pixel[2]
    );

    // Save OpenCV result
    let mut opencv_array = Array3::<u8>::zeros((640, 640, 3));
    for y in 0..640 {
        for x in 0..640 {
            unsafe {
                let ptr = resized_opencv.ptr_2d(y, x).expect("Failed to get pixel");
                opencv_array[[y as usize, x as usize, 0]] = *ptr.add(0);
                opencv_array[[y as usize, x as usize, 1]] = *ptr.add(1);
                opencv_array[[y as usize, x as usize, 2]] = *ptr.add(2);
            }
        }
    }

    // Save to npy
    let output_path = PathBuf::from("/tmp/rust_opencv_resized_uint8.npy");
    let mut file = File::create(&output_path).expect("Failed to create file");

    let dtype_str = "'|u1'";
    let shape_str = format!("({}, {}, {}), ", 640, 640, 3);
    let header = format!(
        "{{'descr': {}, 'fortran_order': False, 'shape': {}}}",
        dtype_str, shape_str
    );
    let header_len = header.len();
    let padding_len = 64 - ((10 + header_len) % 64);
    let padded_header = format!("{}{}", header, " ".repeat(padding_len));

    file.write_all(b"\x93NUMPY").unwrap();
    file.write_all(&[1, 0]).unwrap();
    file.write_all(&[
        (padded_header.len() as u16) as u8,
        ((padded_header.len() as u16) >> 8) as u8,
    ])
    .unwrap();
    file.write_all(padded_header.as_bytes()).unwrap();
    file.write_all(opencv_array.as_slice().unwrap()).unwrap();

    println!("\n✓ Saved OpenCV resized image to {:?}", output_path);
    println!("\nNow run: python3 compare_opencv_with_pil.py");
}
