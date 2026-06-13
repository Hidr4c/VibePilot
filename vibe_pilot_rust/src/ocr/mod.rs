//! Text detection for modal dialogs and UI elements.
//!
//! Provides a trait-based text detection system that can be extended with
//! different backends (OCR, template matching, etc.).
//!
//! # Architecture
//!
//! - `TextDetector` trait defines the interface for finding text in images.
//! - `NullTextDetector` is the default no-op implementation.
//! - `CompositeDetector` chains multiple detectors, returning the first match.
//!
//! # Examples
//!
//! ```
//! use vibe_pilot_rust::ocr::{TextDetector, CompositeDetector, NullTextDetector};
//! use image::DynamicImage;
//!
//! let detector = CompositeDetector::new(vec![
//!     Box::new(NullTextDetector::new()),
//! ]);
//! // find_text_center returns None for NullTextDetector
//! ```

pub use null_detector::NullTextDetector;
pub use composite_detector::CompositeDetector;
pub use tesseract_detector::TesseractTextDetector;
pub use template_detector::TemplateTextDetector;

pub mod tesseract_detector;
pub mod template_detector;

use image::DynamicImage;

/// Trait for detecting text in images and returning center coordinates.
///
/// Implementors search for target words/phrases in the image and return
/// the center pixel coordinates of the first match, or `None` if not found.
pub trait TextDetector: Send + Sync {
    /// Searches for target words in the image and returns the center coordinates.
    ///
    /// # Arguments
    /// * `image` - The image to search in.
    /// * `target_words` - Words/phrases to look for (case-insensitive).
    ///
    /// # Returns
    /// `Some((x, y))` pixel coordinates of the text center, or `None` if not found.
    fn find_text_center(&self, image: &DynamicImage, target_words: &[&str]) -> Option<(u32, u32)>;
}

/// Default no-op text detector.
///
/// Always returns `None`. Used as a fallback when no OCR backend is available.
pub mod null_detector {
    use super::TextDetector;
    use image::DynamicImage;

    /// A text detector that always returns `None`.
    ///
    /// This is the simplest possible implementation, used when no OCR
    /// backend (Tesseract, etc.) is installed or configured.
    #[derive(Debug, Clone, Default)]
    pub struct NullTextDetector;

    impl NullTextDetector {
        /// Creates a new null text detector.
        pub fn new() -> Self {
            Self
        }
    }

    impl TextDetector for NullTextDetector {
        fn find_text_center(&self, _image: &DynamicImage, _target_words: &[&str]) -> Option<(u32, u32)> {
            None
        }
    }
}

/// Composite detector that chains multiple text detectors.
///
/// Iterates through detectors in order, returning the first non-null result.
/// If all detectors return `None`, the composite returns `None`.
pub mod composite_detector {
    use super::TextDetector;
    use image::DynamicImage;
    use std::sync::Arc;

    /// Chains multiple `TextDetector` implementations.
    ///
    /// # Examples
    ///
    /// ```
    /// use vibe_pilot_rust::ocr::{CompositeDetector, NullTextDetector};
    ///
    /// let detector = CompositeDetector::new(vec![
    ///     Box::new(NullTextDetector::new()),
    /// ]);
    /// ```
    #[derive(Clone)]
    #[allow(dead_code)]
    pub struct CompositeDetector {
        detectors: Vec<Arc<dyn TextDetector>>,
        debug_name: String,
    }

    impl CompositeDetector {
        /// Creates a new composite detector from a list of detectors.
        pub fn new(detectors: Vec<Box<dyn TextDetector>>) -> Self {
            Self {
                detectors: detectors.into_iter().map(|d| Arc::from(d) as Arc<dyn TextDetector>).collect(),
                debug_name: "composite".to_string(),
            }
        }

        /// Returns the number of detectors in this composite.
        pub fn len(&self) -> usize {
            self.detectors.len()
        }

        /// Returns `true` if the composite contains no detectors.
        pub fn is_empty(&self) -> bool {
            self.detectors.is_empty()
        }
    }

    impl Default for CompositeDetector {
        fn default() -> Self {
            Self {
                detectors: Vec::new(),
                debug_name: "default".to_string(),
            }
        }
    }

    impl TextDetector for CompositeDetector {
        fn find_text_center(&self, image: &DynamicImage, target_words: &[&str]) -> Option<(u32, u32)> {
            for detector in &self.detectors {
                if let Some(pos) = detector.find_text_center(image, target_words) {
                    return Some(pos);
                }
            }
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::DynamicImage;

    #[test]
    fn test_null_detector_always_returns_none() {
        let detector = NullTextDetector::new();
        let img = DynamicImage::new_rgb8(100, 100);
        assert!(detector.find_text_center(&img, &["ok", "yes", "cancel"]).is_none());
    }

    #[test]
    fn test_composite_empty_returns_none() {
        let detector = CompositeDetector::default();
        let img = DynamicImage::new_rgb8(100, 100);
        assert!(detector.find_text_center(&img, &["ok"]).is_none());
    }

    #[test]
    fn test_composite_with_null_detector() {
        let detector = CompositeDetector::new(vec![Box::new(NullTextDetector::new())]);
        let img = DynamicImage::new_rgb8(100, 100);
        assert!(detector.find_text_center(&img, &["ok"]).is_none());
        assert_eq!(detector.len(), 1);
        assert!(!detector.is_empty());
    }

    #[test]
    fn test_composite_len_and_empty() {
        let detector = CompositeDetector::new(vec![Box::new(NullTextDetector::new())]);
        assert_eq!(detector.len(), 1);
        assert!(!detector.is_empty());

        let empty = CompositeDetector::default();
        assert_eq!(empty.len(), 0);
        assert!(empty.is_empty());
    }

    #[test]
    fn test_tesseract_detector_new() {
        let detector = TesseractTextDetector::new("fra".to_string());
        assert_eq!(detector.lang, "fra");
    }

    #[test]
    fn test_tesseract_detector_runs_gracefully() {
        let detector = TesseractTextDetector::default();
        let img = DynamicImage::new_rgb8(100, 100);
        let _ = detector.find_text_center(&img, &["ok"]);
    }

    #[test]
    fn test_template_detector_matching() {
        let temp_dir = tempfile::tempdir().unwrap();
        let templates_path = temp_dir.path();

        let mut template = DynamicImage::new_rgb8(5, 5);
        if let Some(rgb) = template.as_mut_rgb8() {
            for y in 0..5 {
                for x in 0..5 {
                    rgb.put_pixel(x, y, image::Rgb([255, 0, 0]));
                }
            }
        }
        
        let template_file_path = templates_path.join("ok.png");
        template.save(&template_file_path).unwrap();

        let mut input = DynamicImage::new_rgb8(50, 50);
        if let Some(rgb) = input.as_mut_rgb8() {
            for y in 20..25 {
                for x in 20..25 {
                    rgb.put_pixel(x, y, image::Rgb([255, 0, 0]));
                }
            }
        }

        let detector = TemplateTextDetector::new(templates_path, 225.0);
        let coords = detector.find_text_center(&input, &["ok"]);
        
        assert!(coords.is_some());
        let (cx, cy) = coords.unwrap();
        assert_eq!(cx, 22);
        assert_eq!(cy, 22);
    }

    #[test]
    fn test_template_detector_no_match() {
        let temp_dir = tempfile::tempdir().unwrap();
        let templates_path = temp_dir.path();

        let mut template = DynamicImage::new_rgb8(5, 5);
        if let Some(rgb) = template.as_mut_rgb8() {
            for y in 0..5 {
                for x in 0..5 {
                    rgb.put_pixel(x, y, image::Rgb([255, 0, 0]));
                }
            }
        }
        template.save(templates_path.join("ok.png")).unwrap();

        let mut input = DynamicImage::new_rgb8(50, 50);
        if let Some(rgb) = input.as_mut_rgb8() {
            for y in 0..50 {
                for x in 0..50 {
                    rgb.put_pixel(x, y, image::Rgb([255, 255, 255]));
                }
            }
        }

        let detector = TemplateTextDetector::new(templates_path, 225.0);
        let coords = detector.find_text_center(&input, &["ok"]);
        assert!(coords.is_none());
    }

    #[test]
    fn test_template_detector_missing_file_returns_none() {
        let temp_dir = tempfile::tempdir().unwrap();
        let detector = TemplateTextDetector::new(temp_dir.path(), 225.0);
        let img = DynamicImage::new_rgb8(50, 50);
        assert!(detector.find_text_center(&img, &["ok"]).is_none());
    }

    #[test]
    fn test_template_detector_default() {
        let detector = TemplateTextDetector::default();
        let img = DynamicImage::new_rgb8(50, 50);
        let _ = detector.find_text_center(&img, &["ok"]);
    }

    #[test]
    fn test_template_detector_empty_target() {
        let temp_dir = tempfile::tempdir().unwrap();
        let detector = TemplateTextDetector::new(temp_dir.path(), 225.0);
        let img = DynamicImage::new_rgb8(50, 50);
        assert!(detector.find_text_center(&img, &["", "   "]).is_none());
    }

    #[test]
    fn test_template_detector_corrupted_file() {
        let temp_dir = tempfile::tempdir().unwrap();
        let file_path = temp_dir.path().join("corrupt.png");
        std::fs::write(&file_path, b"invalid png data").unwrap();

        let detector = TemplateTextDetector::new(temp_dir.path(), 225.0);
        let img = DynamicImage::new_rgb8(50, 50);
        assert!(detector.find_text_center(&img, &["corrupt"]).is_none());
    }

    #[test]
    fn test_template_detector_input_smaller_than_template() {
        let temp_dir = tempfile::tempdir().unwrap();
        let templates_path = temp_dir.path();

        let mut template = DynamicImage::new_rgb8(10, 10);
        if let Some(rgb) = template.as_mut_rgb8() {
            for y in 0..10 {
                for x in 0..10 {
                    rgb.put_pixel(x, y, image::Rgb([255, 0, 0]));
                }
            }
        }
        template.save(templates_path.join("large.png")).unwrap();

        let detector = TemplateTextDetector::new(templates_path, 225.0);
        let input = DynamicImage::new_rgb8(5, 5);
        assert!(detector.find_text_center(&input, &["large"]).is_none());
    }
}
