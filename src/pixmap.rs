use std::fs::File;
use std::io;
use std::io::{BufRead, BufReader, Error, ErrorKind, Read, Write};
use std::path::Path;
use std::vec::Vec;
use byteorder::{LittleEndian, ReadBytesExt};

use flate2::write::ZlibEncoder;
use flate2::Compression;

#[derive(Debug, Clone)]
pub struct RGB {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

#[derive(Debug)]
pub struct Pixmap {
    pub format: String,
    pub w: u32,
    pub h: u32,
    pub max_color_val: u32,
    pub data: Vec<RGB>,
}

impl Pixmap {
    pub fn at(&self, x: u32, y: u32) -> u8 {
        let index = (y * self.w + x) as usize;

        return ((self.data[index].r as u32 + self.data[index].g as u32 + self.data[index].b as u32) / 3) as u8;
    }
}

pub fn load_ppm(filename: &str) -> io::Result<Pixmap> {
    let file = File::open(filename)?;
    let mut reader = BufReader::new(file);

    let mut header = String::new();
    reader.read_line(&mut header)?;

    if !header.starts_with("P6") {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Invalid PPM format",
        ));
    }

    let mut dimensions = String::new();
    reader.read_line(&mut dimensions)?;

    let mut dimensions_iter = dimensions.split_whitespace();
    let w: u32 = dimensions_iter.next().unwrap().parse().unwrap();
    let h: u32 = dimensions_iter.next().unwrap().parse().unwrap();

    let mut max_color_val_str = String::new();
    reader.read_line(&mut max_color_val_str)?;
    let max_color_val: u32 = max_color_val_str.trim().parse().unwrap();

    let expected_bytes = (w * h * 3) as usize;

    let mut data = Vec::with_capacity(expected_bytes);

    reader.read_to_end(&mut data)?;

    if data.len() != expected_bytes {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "Invalid PPM data"));
    }

    let data = data.chunks(3).map(|chunk| RGB {
        r: chunk[0],
        g: chunk[1],
        b: chunk[2],
    }).collect();

    Ok(Pixmap {
        format: "P6".to_string(),
        w,
        h,
        max_color_val,
        data,
    })
}


fn compress_data(data: &[RGB]) -> Vec<u8> {
    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());

    let mut bytes = Vec::with_capacity(data.len() * 3);
    for pixel in data {
        bytes.push(pixel.r);
        bytes.push(pixel.g);
        bytes.push(pixel.b);
    }

    encoder.write_all(&bytes).unwrap();

    encoder.finish().unwrap()
}

pub fn dump(images: &[Pixmap], filename: &str) {
    let mut f = File::create(filename).unwrap();

    let num_images = images.len() as u32;
    f.write_all(&num_images.to_le_bytes()).unwrap();

    for img in images {
        let format_length = img.format.len() as u32;
        f.write_all(&format_length.to_le_bytes()).unwrap();
        f.write_all(img.format.as_bytes()).unwrap();

        f.write_all(&img.w.to_le_bytes()).unwrap();
        f.write_all(&img.h.to_le_bytes()).unwrap();
        f.write_all(&img.max_color_val.to_le_bytes()).unwrap();

        let compressed_data = compress_data(&img.data);
        let compressed_size = compressed_data.len() as u32;
        f.write_all(&compressed_size.to_le_bytes()).unwrap();
        f.write_all(&compressed_data).unwrap();
    }
}

fn decompress_data(compressed_data: &[u8], original_size: usize) -> Vec<RGB> {
    let mut decompressed_bytes = Vec::new();
    {
        let mut decoder = flate2::read::ZlibDecoder::new(compressed_data);
        decoder.read_to_end(&mut decompressed_bytes).unwrap();
    }

    let mut decompressed_data = Vec::new();
    for chunk in decompressed_bytes.chunks(3) {
        let r = chunk[0];
        let g = chunk[1];
        let b = chunk[2];
        decompressed_data.push(RGB { r, g, b });
    }

    decompressed_data.resize(original_size / 3, RGB { r: 0, g: 0, b: 0 });

    decompressed_data
}

pub fn read_bin(filename: &str) -> Vec<Pixmap> {
    let file = File::open(filename).unwrap();
    let mut reader = BufReader::new(file);

    let num_images = reader.read_u32::<LittleEndian>().unwrap();

    let images: Vec<Pixmap> = (0..num_images).into_iter().map(|_| {
        let format_length = reader.read_u32::<LittleEndian>().unwrap();
        let mut format_buf = vec![0u8; format_length as usize];
        reader.read_exact(&mut format_buf).unwrap();

        let w = reader.read_u32::<LittleEndian>().unwrap();
        let h = reader.read_u32::<LittleEndian>().unwrap();
        let max_color_val = reader.read_u32::<LittleEndian>().unwrap();

        let compressed_size = reader.read_u32::<LittleEndian>().unwrap();
        let mut compressed_data = vec![0u8; compressed_size as usize];
        reader.read_exact(&mut compressed_data).unwrap();

        let data = decompress_data(&compressed_data, (w * h * 3) as usize);

        Pixmap {
            format: String::from_utf8_lossy(&format_buf).to_string(),
            w,
            h,
            max_color_val,
            data,
        }
    }).collect();

    images
}