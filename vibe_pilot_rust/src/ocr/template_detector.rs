use super::TextDetector;
use image::DynamicImage;
use std::path::{Path, PathBuf};

/// Template-matching-based text detector.
///
/// Searches for matching template images saved on disk representing modal buttons
/// like "OK", "Yes", "Oui".
#[derive(Debug, Clone)]
pub struct TemplateTextDetector {
    templates_dir: PathBuf,
    mse_threshold: f64,
}

impl TemplateTextDetector {
    /// Creates a new `TemplateTextDetector` looking in the specified directory.
    pub fn new<P: AsRef<Path>>(templates_dir: P, mse_threshold: f64) -> Self {
        Self {
            templates_dir: templates_dir.as_ref().to_path_buf(),
            mse_threshold,
        }
    }
}

impl Default for TemplateTextDetector {
    fn default() -> Self {
        Self {
            templates_dir: PathBuf::from("resources/templates"),
            mse_threshold: 225.0, // Mean Squared Error threshold (avg difference of 15.0 per pixel)
        }
    }
}

impl TextDetector for TemplateTextDetector {
    fn find_text_center(&self, image: &DynamicImage, target_words: &[&str]) -> Option<(u32, u32)> {
        let input_gray = image.to_luma8();

        for &target in target_words {
            // Clean target to a safe filename
            let clean_target = target.trim().to_lowercase();
            if clean_target.is_empty() {
                continue;
            }

            // Construct template path
            let template_path = self.templates_dir.join(format!("{}.png", clean_target));
            if !template_path.exists() {
                log::debug!("Template matching: template file not found at {:?}", template_path);
                continue;
            }

            // Load template image
            let template_img = match image::open(&template_path) {
                Ok(img) => img,
                Err(e) => {
                    log::warn!("Template matching: failed to load template at {:?}: {}", template_path, e);
                    continue;
                }
            };

            let template_gray = template_img.to_luma8();
            let tw = template_gray.width();
            let th = template_gray.height();

            if input_gray.width() < tw || input_gray.height() < th {
                log::debug!(
                    "Template matching: input image ({:?}) is smaller than template ({:?})",
                    (input_gray.width(), input_gray.height()),
                    (tw, th)
                );
                continue;
            }

            // Perform template matching using custom implementation
            if let Some((loc, sse)) = find_best_template_match(&input_gray, &template_gray) {
                let mse = sse as f64 / (tw as f64 * th as f64);

                log::debug!(
                    "Template matching: target '{}' matched with SSE: {}, MSE: {:.2} (threshold: {:.2})",
                    clean_target,
                    sse,
                    mse,
                    self.mse_threshold
                );

                if mse <= self.mse_threshold {
                    let cx = loc.0 + tw / 2;
                    let cy = loc.1 + th / 2;
                    return Some((cx, cy));
                }
            }
        }

        None
    }
}

/// Custom lightweight Sum-of-Squared-Errors template matching function to avoid imageproc crate.
fn find_best_template_match(
    input: &image::GrayImage,
    template: &image::GrayImage,
) -> Option<((u32, u32), f32)> {
    let (input_w, input_h) = input.dimensions();
    let (tpl_w, tpl_h) = template.dimensions();

    if input_w < tpl_w || input_h < tpl_h {
        return None;
    }

    let mut min_sse = f32::MAX;
    let mut best_loc = (0, 0);

    for y in 0..=(input_h - tpl_h) {
        for x in 0..=(input_w - tpl_w) {
            let mut sse = 0.0;
            let mut exceeded = false;
            for dy in 0..tpl_h {
                for dx in 0..tpl_w {
                    let pixel_in = input.get_pixel(x + dx, y + dy).0[0] as f32;
                    let pixel_tpl = template.get_pixel(dx, dy).0[0] as f32;
                    let diff = pixel_in - pixel_tpl;
                    sse += diff * diff;
                }
                if sse >= min_sse {
                    exceeded = true;
                    break;
                }
            }
            if !exceeded && sse < min_sse {
                min_sse = sse;
                best_loc = (x, y);
            }
        }
    }

    Some((best_loc, min_sse))
}
