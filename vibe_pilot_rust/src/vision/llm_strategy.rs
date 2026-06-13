//! LLM-based zoom strategy for region-of-interest detection.
//!
//! Calls the LLM to identify a target region on the screenshot,
//! crops and upscales that region, and provides coordinate remapping.

use super::{ZoomStrategy, ZoomContext, ZoomedRegion, BoxFuture};
use crate::llm_client::LlmProvider;
use crate::vision_pipeline::RegionOfInterest;
use image::DynamicImage;

/// Zoom strategy that uses the LLM to detect regions of interest.
///
/// Configurable zoom factor for upscaled crops.
#[derive(Debug, Clone)]
pub struct LlmZoomStrategy {
    zoom_factor: f64,
}

impl LlmZoomStrategy {
    /// Creates a new LLM zoom strategy with the given zoom factor.
    ///
    /// The zoom factor (e.g., 2.0) determines the upscale ratio of the cropped region.
    pub fn new(zoom_factor: f64) -> Self {
        Self { zoom_factor }
    }

    /// Returns the zoom factor of this strategy.
    pub fn zoom_factor(&self) -> f64 {
        self.zoom_factor
    }
}

impl ZoomStrategy for LlmZoomStrategy {
    fn get_zoomed_region<'a>(
        &'a self,
        llm: &'a dyn LlmProvider,
        full_screenshot: &'a DynamicImage,
        config: &'a crate::config::SavedConfig,
    ) -> BoxFuture<'a, Option<ZoomedRegion>> {
        let zoom_factor = self.zoom_factor;
        Box::pin(async move {
            let (target_url, target_model, auth_mode, auth_key, auth_login, auth_pass, timeout) = if config.utiliser_moteur_vision_dedie {
                (
                    &config.url_api_vision,
                    &config.nom_modele_vision,
                    &config.auth_mode_vision,
                    &config.auth_api_key_vision,
                    &config.auth_login_vision,
                    &config.auth_password_vision,
                    config.request_timeout_secs_vision,
                )
            } else {
                (
                    &config.url_api,
                    &config.nom_modele,
                    &config.auth_mode,
                    &config.auth_api_key,
                    &config.auth_login,
                    &config.auth_password,
                    config.request_timeout_secs,
                )
            };

            let roi = match crate::vision_pipeline::VisionPipeline::identify_regions(
                llm,
                full_screenshot,
                &config.contexte,
                &config.objectif,
                &config.task,
                target_url,
                target_model,
                auth_mode,
                auth_key,
                auth_login,
                auth_pass,
                timeout,
            ).await {
                Ok(roi) => roi,
                Err(_) => return None,
            };

            let (zoomed, ctx) = crop_and_zoom_with_factor(full_screenshot, &roi, zoom_factor);

            let (ox, oy, ow, oh) = (
                (ctx.crop_x * full_screenshot.width() as f64).round() as u32,
                (ctx.crop_y * full_screenshot.height() as f64).round() as u32,
                (ctx.crop_width * full_screenshot.width() as f64).round() as u32,
                (ctx.crop_height * full_screenshot.height() as f64).round() as u32,
            );

            Some(ZoomedRegion {
                zoomed_image: zoomed,
                original_rect: (ox, oy, ow, oh),
                crop_context: ctx,
            })
        })
    }

    fn remap_coordinates(&self, (zx, zy): (f64, f64), region: &ZoomedRegion) -> (f64, f64) {
        let ctx = &region.crop_context;
        let orig_x = ctx.crop_x + (zx * ctx.crop_width);
        let orig_y = ctx.crop_y + (zy * ctx.crop_height);
        (orig_x.clamp(0.0, 1.0), orig_y.clamp(0.0, 1.0))
    }
}

/// Crop and zoom with a configurable factor.
fn crop_and_zoom_with_factor(
    screenshot: &DynamicImage,
    roi: &RegionOfInterest,
    zoom_factor: f64,
) -> (DynamicImage, ZoomContext) {
    let img_w = screenshot.width() as f64;
    let img_h = screenshot.height() as f64;

    let half_w = roi.width / 2.0;
    let half_h = roi.height / 2.0;

    let crop_x = (roi.x - half_w).clamp(0.0, 1.0);
    let crop_y = (roi.y - half_h).clamp(0.0, 1.0);
    let crop_width = roi.width.min(1.0 - crop_x);
    let crop_height = roi.height.min(1.0 - crop_y);

    let x_px = (crop_x * img_w).round() as u32;
    let y_px = (crop_y * img_h).round() as u32;
    let w_px = (crop_width * img_w).round() as u32;
    let h_px = (crop_height * img_h).round() as u32;

    let x_px = x_px.min(screenshot.width().saturating_sub(1));
    let y_px = y_px.min(screenshot.height().saturating_sub(1));
    let w_px = w_px.max(16).min(screenshot.width() - x_px);
    let h_px = h_px.max(16).min(screenshot.height() - y_px);

    let cropped = screenshot.crop_imm(x_px, y_px, w_px, h_px);

    let zoom_scale = zoom_factor.max(1.0);
    let zoomed = cropped.resize(
        (cropped.width() as f64 * zoom_scale).round() as u32,
        (cropped.height() as f64 * zoom_scale).round() as u32,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_llm_zoom_strategy_new() {
        let strategy = LlmZoomStrategy::new(2.0);
        assert!((strategy.zoom_factor() - 2.0).abs() < 1e-5);
    }

    #[test]
    fn test_llm_zoom_strategy_default_zoom() {
        let strategy = LlmZoomStrategy::new(1.0);
        assert!((strategy.zoom_factor() - 1.0).abs() < 1e-5);
    }

    #[test]
    fn test_llm_zoom_strategy_remap_coordinates() {
        let strategy = LlmZoomStrategy::new(2.0);
        let ctx = ZoomContext {
            crop_x: 0.1,
            crop_y: 0.2,
            crop_width: 0.5,
            crop_height: 0.6,
        };
        let region = ZoomedRegion {
            zoomed_image: image::DynamicImage::new_rgb8(10, 10),
            original_rect: (10, 20, 50, 60),
            crop_context: ctx,
        };
        let (rx, ry) = strategy.remap_coordinates((0.5, 0.5), &region);
        assert!((rx - 0.35).abs() < 1e-5);
        assert!((ry - 0.5).abs() < 1e-5);
    }

    #[test]
    fn test_llm_zoom_strategy_remap_at_origin() {
        let strategy = LlmZoomStrategy::new(3.0);
        let ctx = ZoomContext {
            crop_x: 0.0,
            crop_y: 0.0,
            crop_width: 1.0,
            crop_height: 1.0,
        };
        let region = ZoomedRegion {
            zoomed_image: image::DynamicImage::new_rgb8(10, 10),
            original_rect: (0, 0, 100, 100),
            crop_context: ctx,
        };
        let (rx, ry) = strategy.remap_coordinates((0.0, 0.0), &region);
        assert!((rx - 0.0).abs() < 1e-5);
        assert!((ry - 0.0).abs() < 1e-5);
    }

    #[test]
    fn test_llm_zoom_strategy_remap_at_boundary() {
        let strategy = LlmZoomStrategy::new(2.0);
        let ctx = ZoomContext {
            crop_x: 0.0,
            crop_y: 0.0,
            crop_width: 1.0,
            crop_height: 1.0,
        };
        let region = ZoomedRegion {
            zoomed_image: image::DynamicImage::new_rgb8(10, 10),
            original_rect: (0, 0, 100, 100),
            crop_context: ctx,
        };
        let (rx, ry) = strategy.remap_coordinates((1.0, 1.0), &region);
        assert!((rx - 1.0).abs() < 1e-5);
        assert!((ry - 1.0).abs() < 1e-5);
    }

    #[test]
    fn test_crop_and_zoom_with_factor_basic() {
        let img = DynamicImage::ImageRgba8(image::RgbaImage::new(200, 100));
        let roi = RegionOfInterest {
            x: 0.5,
            y: 0.5,
            width: 0.25,
            height: 0.25,
            label: "test".to_string(),
        };
        let (zoomed, ctx) = crop_and_zoom_with_factor(&img, &roi, 2.0);
        assert!((ctx.crop_x - 0.375).abs() < 1e-5);
        assert!((ctx.crop_y - 0.375).abs() < 1e-5);
        assert!((ctx.crop_width - 0.25).abs() < 1e-5);
        assert!((ctx.crop_height - 0.25).abs() < 1e-5);
        assert_eq!(zoomed.width(), 100);
        assert_eq!(zoomed.height(), 50);
    }

    #[test]
    fn test_crop_and_zoom_with_factor_high_zoom() {
        let img = DynamicImage::ImageRgba8(image::RgbaImage::new(200, 200));
        let roi = RegionOfInterest {
            x: 0.5,
            y: 0.5,
            width: 0.1,
            height: 0.1,
            label: "small".to_string(),
        };
        let (zoomed, _) = crop_and_zoom_with_factor(&img, &roi, 5.0);
        // 200 * 0.1 = 20px crop, * 5.0 = 100px zoomed
        assert_eq!(zoomed.width(), 100);
        assert_eq!(zoomed.height(), 100);
    }

    #[test]
    fn test_crop_and_zoom_with_factor_clamp_right() {
        let img = DynamicImage::ImageRgba8(image::RgbaImage::new(100, 100));
        let roi = RegionOfInterest {
            x: 0.95,
            y: 0.5,
            width: 0.1,
            height: 0.2,
            label: "edge".to_string(),
        };
        let (_, ctx) = crop_and_zoom_with_factor(&img, &roi, 2.0);
        assert!(ctx.crop_x >= 0.0 && ctx.crop_x <= 1.0);
        assert!(ctx.crop_width > 0.0);
    }

    #[test]
    fn test_crop_and_zoom_with_factor_clamp_bottom() {
        let img = DynamicImage::ImageRgba8(image::RgbaImage::new(100, 100));
        let roi = RegionOfInterest {
            x: 0.5,
            y: 0.95,
            width: 0.2,
            height: 0.1,
            label: "edge".to_string(),
        };
        let (_, ctx) = crop_and_zoom_with_factor(&img, &roi, 2.0);
        assert!(ctx.crop_y >= 0.0 && ctx.crop_y <= 1.0);
        assert!(ctx.crop_height > 0.0);
    }

    #[test]
    fn test_crop_and_zoom_with_factor_minimum_size() {
        let img = DynamicImage::ImageRgba8(image::RgbaImage::new(100, 100));
        let roi = RegionOfInterest {
            x: 0.5,
            y: 0.5,
            width: 0.001,
            height: 0.001,
            label: "tiny".to_string(),
        };
        let (zoomed, _) = crop_and_zoom_with_factor(&img, &roi, 2.0);
        assert!(zoomed.width() >= 16);
        assert!(zoomed.height() >= 16);
    }

    #[test]
    fn test_crop_and_zoom_with_factor_zero_width() {
        let img = DynamicImage::ImageRgba8(image::RgbaImage::new(100, 100));
        let roi = RegionOfInterest {
            x: 0.5,
            y: 0.5,
            width: 0.0,
            height: 0.5,
            label: "zero_w".to_string(),
        };
        let (_, ctx) = crop_and_zoom_with_factor(&img, &roi, 2.0);
        assert!(ctx.crop_width >= 0.0);
    }

    #[tokio::test]
    async fn test_llm_zoom_strategy_get_zoomed_region() {
        use crate::orchestrator::tests::MockLlmClient;
        use crate::llm_client::LlmResponse;

        let llm = MockLlmClient {
            next_response: std::sync::Mutex::new(LlmResponse::default()),
            decompose_response: std::sync::Mutex::new(String::new()),
            call_count: std::sync::Mutex::new(0),
        };

        let strategy = LlmZoomStrategy::new(2.0);
        let img = DynamicImage::ImageRgba8(image::RgbaImage::new(100, 100));
        let config = crate::config::SavedConfig::default();

        let region = strategy.get_zoomed_region(&llm, &img, &config).await;
        assert!(region.is_some());
        let reg = region.unwrap();
        assert_eq!(reg.original_rect.2, 20); // roi.width is 0.2 * 100
        assert_eq!(reg.original_rect.3, 20); // roi.height is 0.2 * 100
    }
}
