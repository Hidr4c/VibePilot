//! Multi-step vision pipeline and dynamic region-of-interest cropping/zooming.
//!
//! Provides coordinate mapping and cropping utility methods in memory (RAM)
//! to allow the LLM to zoom in on specific regions of interest.

use serde::{Serialize, Deserialize};
use image::DynamicImage;
use crate::llm_client::LlmProvider;

/// A region of interest detected by the LLM on the full screen.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RegionOfInterest {
    /// Relative center X coordinate (0.0 to 1.0)
    pub x: f64,
    /// Relative center Y coordinate (0.0 to 1.0)
    pub y: f64,
    /// Relative width (0.0 to 1.0)
    pub width: f64,
    /// Relative height (0.0 to 1.0)
    pub height: f64,
    /// Human-readable label for this region (e.g. "Send button", "Search input")
    pub label: String,
}

/// Dynamic zoom context needed to convert coordinate spaces.
#[derive(Debug, Clone)]
pub struct ZoomContext {
    /// Relative X offset of the cropped region on the original image (0.0 to 1.0)
    pub crop_x: f64,
    /// Relative Y offset of the cropped region on the original image (0.0 to 1.0)
    pub crop_y: f64,
    /// Relative width of the cropped region on the original image (0.0 to 1.0)
    pub crop_width: f64,
    /// Relative height of the cropped region on the original image (0.0 to 1.0)
    pub crop_height: f64,
}

pub struct VisionPipeline;

impl VisionPipeline {
    /// Crops a region of interest from the screenshot and returns the zoomed image
    /// along with the zoom context for coordinate remapping.
    ///
    /// The crop is computed dynamically in RAM and upscaled by 2x for visual clarity.
    pub fn crop_and_zoom(
        screenshot: &DynamicImage,
        roi: &RegionOfInterest,
    ) -> (DynamicImage, ZoomContext) {
        let img_w = screenshot.width() as f64;
        let img_h = screenshot.height() as f64;

        // Compute crop boundaries (relative coordinates)
        let half_w = roi.width / 2.0;
        let half_h = roi.height / 2.0;

        let crop_x = (roi.x - half_w).clamp(0.0, 1.0);
        let crop_y = (roi.y - half_h).clamp(0.0, 1.0);
        let crop_width = roi.width.min(1.0 - crop_x);
        let crop_height = roi.height.min(1.0 - crop_y);

        // Convert to absolute pixel boundaries
        let x_px = (crop_x * img_w).round() as u32;
        let y_px = (crop_y * img_h).round() as u32;
        let w_px = (crop_width * img_w).round() as u32;
        let h_px = (crop_height * img_h).round() as u32;

        // Avoid cropping empty images or out of bounds
        let x_px = x_px.min(screenshot.width().saturating_sub(1));
        let y_px = y_px.min(screenshot.height().saturating_sub(1));
        let w_px = w_px.max(16).min(screenshot.width() - x_px);
        let h_px = h_px.max(16).min(screenshot.height() - y_px);

        // Perform crop in RAM
        let cropped = screenshot.crop_imm(x_px, y_px, w_px, h_px);

        // Upscale x2 for premium quality and fine text readability
        let zoomed = cropped.resize(
            cropped.width() * 2,
            cropped.height() * 2,
            image::imageops::FilterType::Lanczos3,
        );

        let context = ZoomContext {
            crop_x,
            crop_y,
            crop_width,
            crop_height,
        };

        (zoomed, context)
    }

    /// Converts relative coordinates on a zoomed image back to relative coordinates
    /// on the original image.
    ///
    /// # Arguments
    /// * `ctx` - The ZoomContext of the cropped region
    /// * `zoomed_x` - Relative X coordinate on the zoomed image (0.0 to 1.0)
    /// * `zoomed_y` - Relative Y coordinate on the zoomed image (0.0 to 1.0)
    pub fn remap_coordinates(ctx: &ZoomContext, zoomed_x: f64, zoomed_y: f64) -> (f64, f64) {
        let orig_x = ctx.crop_x + (zoomed_x * ctx.crop_width);
        let orig_y = ctx.crop_y + (zoomed_y * ctx.crop_height);
        (orig_x.clamp(0.0, 1.0), orig_y.clamp(0.0, 1.0))
    }

    /// Calls the LLM to identify the region of interest for a given objective.
    #[allow(clippy::too_many_arguments)]
    pub async fn identify_regions(
        llm: &dyn LlmProvider,
        screenshot: &DynamicImage,
        contexte: &str,
        objectif: &str,
        task: &str,
        url: &str,
        model: &str,
        auth_mode: &str,
        auth_api_key: &str,
        auth_login: &str,
        auth_password: &str,
        timeout_secs: u64,
    ) -> Result<RegionOfInterest, String> {
        let json_text = llm.identify_roi(
            screenshot,
            contexte,
            objectif,
            task,
            url,
            model,
            auth_mode,
            auth_api_key,
            auth_login,
            auth_password,
            timeout_secs,
        ).await?;

        // Parse JSON response
        let roi: RegionOfInterest = serde_json::from_str(&json_text)
            .map_err(|e| format!("Failed to parse ROI JSON: {} (raw response: {})", e, json_text))?;

        // Validate values
        if roi.x < 0.0 || roi.x > 1.0 || roi.y < 0.0 || roi.y > 1.0 {
            return Err(format!("LLM returned out-of-bounds ROI center coordinates: x={}, y={}", roi.x, roi.y));
        }
        if roi.width <= 0.0 || roi.height <= 0.0 {
            return Err(format!("LLM returned invalid ROI dimensions: w={}, h={}", roi.width, roi.height));
        }

        Ok(roi)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_remap_coordinates() {
        let ctx = ZoomContext {
            crop_x: 0.1,
            crop_y: 0.2,
            crop_width: 0.5,
            crop_height: 0.6,
        };

        let (rx, ry) = VisionPipeline::remap_coordinates(&ctx, 0.5, 0.5);
        // orig_x = crop_x + (0.5 * crop_width) = 0.1 + 0.25 = 0.35
        // orig_y = crop_y + (0.5 * crop_height) = 0.2 + 0.30 = 0.50
        assert!((rx - 0.35).abs() < 1e-5);
        assert!((ry - 0.50).abs() < 1e-5);
    }

    #[test]
    fn test_crop_and_zoom() {
        use image::{DynamicImage, RgbaImage};
        let img = DynamicImage::ImageRgba8(RgbaImage::new(100, 100));
        let roi = RegionOfInterest {
            x: 0.5,
            y: 0.5,
            width: 0.2,
            height: 0.2,
            label: "test".to_string(),
        };

        let (zoomed, ctx) = VisionPipeline::crop_and_zoom(&img, &roi);
        assert!((ctx.crop_x - 0.4).abs() < 1e-5);
        assert!((ctx.crop_y - 0.4).abs() < 1e-5);
        assert!((ctx.crop_width - 0.2).abs() < 1e-5);
        assert!((ctx.crop_height - 0.2).abs() < 1e-5);

        // zoomed should be (100 * 0.2) * 2 = 40 pixels wide and high
        assert_eq!(zoomed.width(), 40);
        assert_eq!(zoomed.height(), 40);
    }
}
