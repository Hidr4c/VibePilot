use super::TextDetector;
use image::DynamicImage;

/// OCR-based text detector using Tesseract via `rusty-tesseract`.
#[derive(Debug, Clone)]
pub struct TesseractTextDetector {
    pub lang: String,
}

impl TesseractTextDetector {
    /// Creates a new `TesseractTextDetector` with the specified language.
    pub fn new(lang: String) -> Self {
        Self { lang }
    }
}

impl Default for TesseractTextDetector {
    fn default() -> Self {
        Self {
            lang: "eng+fra".to_string(),
        }
    }
}

impl TextDetector for TesseractTextDetector {
    fn find_text_center(&self, image: &DynamicImage, target_words: &[&str]) -> Option<(u32, u32)> {
        let tess_image = match rusty_tesseract::Image::from_dynamic_image(image) {
            Ok(img) => img,
            Err(e) => {
                log::warn!("Tesseract: failed to create image: {}", e);
                return None;
            }
        };

        let args = rusty_tesseract::Args {
            lang: self.lang.clone(),
            ..Default::default()
        };

        match rusty_tesseract::image_to_data(&tess_image, &args) {
            Ok(data_output) => {
                for entry in data_output.data {
                    if entry.conf < 0.0 {
                        continue;
                    }
                    let clean_text = entry.text.trim().to_lowercase();
                    if clean_text.is_empty() {
                        continue;
                    }
                    for &target in target_words {
                        let target_clean = target.trim().to_lowercase();
                        if !target_clean.is_empty() && (clean_text == target_clean || clean_text.contains(&target_clean)) {
                            let cx = entry.left + entry.width / 2;
                            let cy = entry.top + entry.height / 2;
                            return Some((cx as u32, cy as u32));
                        }
                    }
                }
                None
            }
            Err(e) => {
                log::warn!(
                    "Tesseract: OCR execution failed (make sure the tesseract executable is in your PATH): {}",
                    e
                );
                None
            }
        }
    }
}
