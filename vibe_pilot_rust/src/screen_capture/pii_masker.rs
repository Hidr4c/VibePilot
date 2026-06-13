use std::sync::Arc;
use image::DynamicImage;

/// Trait defining a PII masking service for cleaning screenshots.
pub trait PiiMasker: Send + Sync {
    /// Masks any detected PII directly in the pixel buffer of the image.
    fn mask_pii(&self, img: &mut DynamicImage);
}

/// No-op implementation of PiiMasker.
#[derive(Debug, Clone, Default)]
pub struct NullPiiMasker;

impl NullPiiMasker {
    /// Creates a new NullPiiMasker.
    pub fn new() -> Self {
        Self
    }
}

impl PiiMasker for NullPiiMasker {
    fn mask_pii(&self, _img: &mut DynamicImage) {}
}

/// OCR-based PII Masker that blacks out credit cards, password fields, and API keys.
pub struct OcrPiiMasker {
    lang: String,
}

impl OcrPiiMasker {
    /// Creates a new OcrPiiMasker.
    pub fn new(lang: String) -> Self {
        Self { lang }
    }

    pub(crate) fn is_credit_card(&self, s: &str) -> bool {
        let digits: String = s.chars().filter(|c| c.is_ascii_digit()).collect();
        digits.len() >= 13 && digits.len() <= 19
    }

    pub(crate) fn is_password_bullets(&self, s: &str) -> bool {
        if s.is_empty() {
            return false;
        }
        s.chars().all(|c| c == '•' || c == '●' || c == '*' || c == 'x' || c == 'X')
    }

    pub(crate) fn is_sensitive_key_pattern(&self, s: &str) -> bool {
        let clean = s.to_lowercase();
        if clean.starts_with("sk-") || clean.starts_with("api-") || clean.starts_with("key-") {
            return true;
        }
        // General hex token or mixed-case API key lookalikes
        if s.len() >= 24 {
            let has_digit = s.chars().any(|c| c.is_ascii_digit());
            let has_upper = s.chars().any(|c| c.is_ascii_uppercase());
            let has_lower = s.chars().any(|c| c.is_ascii_lowercase());
            if has_digit && has_upper && has_lower {
                return true;
            }
        }
        false
    }

    pub(crate) fn mask_rect(&self, img: &mut DynamicImage, left: i32, top: i32, width: i32, height: i32) {
        let img_w = img.width();
        let img_h = img.height();

        let start_x = (left.max(0) as u32).min(img_w);
        let end_x = ((left + width).max(0) as u32).min(img_w);
        let start_y = (top.max(0) as u32).min(img_h);
        let end_y = ((top + height).max(0) as u32).min(img_h);

        if let Some(rgba_img) = img.as_mut_rgba8() {
            for y in start_y..end_y {
                for x in start_x..end_x {
                    rgba_img.put_pixel(x, y, image::Rgba([0, 0, 0, 255]));
                }
            }
        }
    }
}

impl PiiMasker for OcrPiiMasker {
    fn mask_pii(&self, img: &mut DynamicImage) {
        let tess_image = match rusty_tesseract::Image::from_dynamic_image(img) {
            Ok(img) => img,
            Err(_) => return,
        };

        let args = rusty_tesseract::Args {
            lang: self.lang.clone(),
            ..Default::default()
        };

        if let Ok(data_output) = rusty_tesseract::image_to_data(&tess_image, &args) {
            for entry in data_output.data {
                if entry.conf < 0.0 {
                    continue;
                }
                let clean_text = entry.text.trim();
                if clean_text.is_empty() {
                    continue;
                }

                if self.is_credit_card(clean_text)
                    || self.is_password_bullets(clean_text)
                    || self.is_sensitive_key_pattern(clean_text)
                {
                    self.mask_rect(img, entry.left, entry.top, entry.width, entry.height);
                }
            }
        }
    }
}

/// Factory to construct PiiMasker implementations following the IoC pattern.
pub struct PiiMaskerFactory;

impl PiiMaskerFactory {
    /// Creates the default PiiMasker implementation.
    pub fn create(lang: String) -> Arc<dyn PiiMasker> {
        Arc::new(OcrPiiMasker::new(lang))
    }
}
