use std::fs::{self, OpenOptions};
use std::path::Path;
use std::time::SystemTime;
use img_parts::jpeg::Jpeg;
use img_parts::png::Png;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FileTimestamps {
    pub modified: Option<SystemTime>,
    pub accessed: Option<SystemTime>,
    pub created: Option<SystemTime>,
}

impl FileTimestamps {
    pub fn from_path(path: &Path) -> Option<Self> {
        let meta = fs::metadata(path).ok()?;
        Some(Self {
            modified: meta.modified().ok(),
            accessed: meta.accessed().ok(),
            created: meta.created().ok(),
        })
    }

    pub fn apply_to(&self, target: &Path) -> std::io::Result<()> {
        let mut times = std::fs::FileTimes::new();
        if let Some(m) = self.modified {
            times = times.set_modified(m);
        }
        if let Some(a) = self.accessed {
            times = times.set_accessed(a);
        }
        #[cfg(windows)]
        if let Some(c) = self.created {
            use std::os::windows::fs::FileTimesExt;
            times = times.set_created(c);
        }
        let file = OpenOptions::new().write(true).open(target)?;
        file.set_times(times)?;
        Ok(())
    }
}

fn preserve_jpeg_metadata(src_bytes: &[u8], encoded_bytes: &[u8]) -> Option<Vec<u8>> {
    let src_jpeg = Jpeg::from_bytes(bytes::Bytes::copy_from_slice(src_bytes)).ok()?;
    let mut dest_jpeg = Jpeg::from_bytes(bytes::Bytes::copy_from_slice(encoded_bytes)).ok()?;

    let metadata_segments: Vec<_> = src_jpeg
        .segments()
        .iter()
        .filter(|seg| {
            let m = seg.marker();
            (0xE1..=0xEF).contains(&m) || m == 0xFE
        })
        .cloned()
        .collect();

    if metadata_segments.is_empty() {
        return None;
    }

    let insert_pos = dest_jpeg
        .segments()
        .iter()
        .position(|seg| seg.marker() != 0xE0)
        .unwrap_or(0);

    for (i, seg) in metadata_segments.into_iter().enumerate() {
        dest_jpeg.segments_mut().insert(insert_pos + i, seg);
    }

    Some(dest_jpeg.encoder().bytes().to_vec())
}

fn preserve_png_metadata(src_bytes: &[u8], encoded_bytes: &[u8]) -> Option<Vec<u8>> {
    let src_png = Png::from_bytes(bytes::Bytes::copy_from_slice(src_bytes)).ok()?;
    let mut dest_png = Png::from_bytes(bytes::Bytes::copy_from_slice(encoded_bytes)).ok()?;

    let is_critical = |kind: [u8; 4]| -> bool {
        matches!(&kind, b"IHDR" | b"PLTE" | b"IDAT" | b"IEND")
    };

    let metadata_chunks: Vec<_> = src_png
        .chunks()
        .iter()
        .filter(|chunk| !is_critical(chunk.kind()))
        .cloned()
        .collect();

    if metadata_chunks.is_empty() {
        return None;
    }

    for chunk in &metadata_chunks {
        let kind = chunk.kind();
        if kind == *b"eXIf" || kind == *b"iCCP" || kind == *b"sRGB" || kind == *b"gAMA" || kind == *b"pHYs" {
            dest_png.remove_chunks_by_type(kind);
        }
    }

    let insert_pos = dest_png
        .chunks()
        .iter()
        .position(|chunk| chunk.kind() == *b"IDAT" || chunk.kind() == *b"IEND")
        .unwrap_or_else(|| dest_png.chunks().len());

    for (i, chunk) in metadata_chunks.into_iter().enumerate() {
        dest_png.chunks_mut().insert(insert_pos + i, chunk);
    }

    Some(dest_png.encoder().bytes().to_vec())
}

pub fn preserve_image_metadata(src_bytes: &[u8], encoded_bytes: &[u8], ext: &str) -> Vec<u8> {
    match ext.to_lowercase().as_str() {
        "jpg" | "jpeg" => {
            preserve_jpeg_metadata(src_bytes, encoded_bytes).unwrap_or_else(|| encoded_bytes.to_vec())
        }
        "png" => {
            preserve_png_metadata(src_bytes, encoded_bytes).unwrap_or_else(|| encoded_bytes.to_vec())
        }
        _ => encoded_bytes.to_vec(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{ExtendedColorType, ImageEncoder, Rgb, RgbImage, RgbaImage};

    #[test]
    fn test_preserve_jpeg_metadata() {
        use img_parts::jpeg::{Jpeg, JpegSegment};

        let mut img = RgbImage::new(100, 100);
        for pixel in img.pixels_mut() {
            *pixel = Rgb([50, 50, 50]);
        }

        let mut raw_jpeg = Vec::new();
        image::codecs::jpeg::JpegEncoder::new(&mut raw_jpeg)
            .write_image(
                img.as_raw(),
                100,
                100,
                ExtendedColorType::Rgb8,
            )
            .unwrap();

        let mut src_jpeg = Jpeg::from_bytes(bytes::Bytes::copy_from_slice(&raw_jpeg)).unwrap();
        let custom_exif = JpegSegment::new_with_contents(
            0xE1,
            bytes::Bytes::from_static(b"Exif\0\0fake_exif_data"),
        );
        src_jpeg.segments_mut().insert(1, custom_exif);
        let src_bytes = src_jpeg.encoder().bytes().to_vec();

        let mut cropped_jpeg = Vec::new();
        let cropped_img = RgbImage::new(50, 50);
        image::codecs::jpeg::JpegEncoder::new(&mut cropped_jpeg)
            .write_image(
                cropped_img.as_raw(),
                50,
                50,
                ExtendedColorType::Rgb8,
            )
            .unwrap();

        let result_bytes = preserve_image_metadata(&src_bytes, &cropped_jpeg, "jpg");
        let result_jpeg = Jpeg::from_bytes(bytes::Bytes::copy_from_slice(&result_bytes)).unwrap();
        let has_exif = result_jpeg.segments().iter().any(|s| {
            s.marker() == 0xE1 && s.contents().starts_with(b"Exif\0\0fake_exif_data")
        });
        assert!(has_exif, "APP1 segment should be preserved in JPEG");
    }

    #[test]
    fn test_preserve_png_metadata() {
        use img_parts::png::{Png, PngChunk};

        let img = RgbaImage::new(100, 100);
        let mut raw_png = Vec::new();
        image::codecs::png::PngEncoder::new(&mut raw_png)
            .write_image(
                img.as_raw(),
                100,
                100,
                ExtendedColorType::Rgba8,
            )
            .unwrap();

        let mut src_png = Png::from_bytes(bytes::Bytes::copy_from_slice(&raw_png)).unwrap();
        let custom_chunk = PngChunk::new(*b"tEXt", bytes::Bytes::from_static(b"Comment\0CropperTest"));
        src_png.chunks_mut().insert(1, custom_chunk);
        let src_bytes = src_png.encoder().bytes().to_vec();

        let cropped_img = RgbaImage::new(50, 50);
        let mut cropped_png = Vec::new();
        image::codecs::png::PngEncoder::new(&mut cropped_png)
            .write_image(
                cropped_img.as_raw(),
                50,
                50,
                ExtendedColorType::Rgba8,
            )
            .unwrap();

        let result_bytes = preserve_image_metadata(&src_bytes, &cropped_png, "png");
        let result_png = Png::from_bytes(bytes::Bytes::copy_from_slice(&result_bytes)).unwrap();
        let has_chunk = result_png.chunks().iter().any(|c| {
            c.kind() == *b"tEXt" && c.contents().as_ref() == b"Comment\0CropperTest"
        });
        assert!(has_chunk, "tEXt chunk should be preserved in PNG");
    }
}
