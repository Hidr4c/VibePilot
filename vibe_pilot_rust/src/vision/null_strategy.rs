//! Null (no-op) zoom strategy.
//!
//! Always returns `None` for zoomed regions. Used as a fallback when
//! no LLM or OCR backend is available.

use super::{ZoomStrategy, ZoomContext, ZoomedRegion, BoxFuture};
use crate::llm_client::LlmProvider;
use image::DynamicImage;
use std::future::ready;
use std::sync::Arc;

/// A zoom strategy that always returns `None`.
///
/// This is the simplest possible implementation, used when no LLM
/// or OCR backend is installed or configured.
#[derive(Debug, Clone, Default)]
pub struct NullZoomStrategy;

impl NullZoomStrategy {
    /// Creates a new null zoom strategy.
    pub fn new() -> Self { Self }
}

impl ZoomStrategy for NullZoomStrategy {
    fn get_zoomed_region<'a>(
        &'a self,
        _llm: &'a dyn LlmProvider,
        _full_screenshot: &'a DynamicImage,
        _config: &'a crate::config::SavedConfig,
    ) -> BoxFuture<'a, Option<ZoomedRegion>> {
        Box::pin(async { None })
    }

    fn remap_coordinates(&self, (zx, zy): (f64, f64), _region: &ZoomedRegion) -> (f64, f64) {
        (zx, zy)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_null_strategy_new() {
        let strategy = NullZoomStrategy::new();
        let result = strategy.get_zoomed_region(
            &crate::llm_client::LlmClient::new(),
            &image::DynamicImage::new_rgb8(100, 100),
            &crate::config::SavedConfig::default(),
        ).await;
        assert!(result.is_none());
    }

    #[test]
    fn test_null_strategy_remap_identity() {
        let strategy = NullZoomStrategy::new();
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
        assert!((rx - 0.5).abs() < 1e-5);
        assert!((ry - 0.5).abs() < 1e-5);
    }

    #[tokio::test]
    async fn test_null_strategy_default() {
        let strategy = NullZoomStrategy::new();
        let result = strategy.get_zoomed_region(
            &crate::llm_client::LlmClient::new(),
            &image::DynamicImage::new_rgb8(100, 100),
            &crate::config::SavedConfig::default(),
        ).await;
        assert!(result.is_none());
    }
}
