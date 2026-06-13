//! Composite zoom strategy that chains multiple strategies.
//!
//! Iterates through strategies in order, returning the first non-null result.
//! If all strategies return `None`, the composite returns `None`.

use super::{ZoomStrategy, ZoomContext, ZoomedRegion, BoxFuture};
use crate::llm_client::LlmProvider;
use image::DynamicImage;
use std::sync::Arc;

/// Chains multiple `ZoomStrategy` implementations.
///
/// Returns the first non-null zoomed region from the chain.
#[derive(Clone, Default)]
pub struct CompositeZoomStrategy {
    strategies: Vec<Arc<dyn ZoomStrategy>>,
}

impl CompositeZoomStrategy {
    /// Creates a new composite zoom strategy from a list of strategies.
    pub fn new(strategies: Vec<Box<dyn ZoomStrategy>>) -> Self {
        Self {
            strategies: strategies.into_iter().map(|s| Arc::from(s) as Arc<dyn ZoomStrategy>).collect(),
        }
    }

    /// Returns the number of strategies in this composite.
    pub fn len(&self) -> usize {
        self.strategies.len()
    }

    /// Returns `true` if the composite contains no strategies.
    pub fn is_empty(&self) -> bool {
        self.strategies.is_empty()
    }
}

impl ZoomStrategy for CompositeZoomStrategy {
    fn get_zoomed_region<'a>(
        &'a self,
        llm: &'a dyn LlmProvider,
        full_screenshot: &'a DynamicImage,
        config: &'a crate::config::SavedConfig,
    ) -> BoxFuture<'a, Option<ZoomedRegion>> {
        let strategies = self.strategies.clone();
        Box::pin(async move {
            for strategy in &strategies {
                if let Some(region) = strategy.get_zoomed_region(llm, full_screenshot, config).await {
                    return Some(region);
                }
            }
            None
        })
    }

    fn remap_coordinates(&self, zoomed_coords: (f64, f64), region: &ZoomedRegion) -> (f64, f64) {
        if let Some(last) = self.strategies.last() {
            last.remap_coordinates(zoomed_coords, region)
        } else {
            zoomed_coords
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vision::{NullZoomStrategy, ZoomContext};

    #[test]
    fn test_composite_empty_returns_none() {
        let composite = CompositeZoomStrategy::default();
        assert!(composite.is_empty());
        assert_eq!(composite.len(), 0);
    }

    #[test]
    fn test_composite_with_strategies() {
        let composite = CompositeZoomStrategy::new(vec![
            Box::new(NullZoomStrategy::new()),
        ]);
        assert!(!composite.is_empty());
        assert_eq!(composite.len(), 1);
    }

    #[test]
    fn test_composite_remap_default() {
        let composite = CompositeZoomStrategy::default();
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
        let (rx, ry) = composite.remap_coordinates((0.5, 0.5), &region);
        assert!((rx - 0.5).abs() < 1e-5);
        assert!((ry - 0.5).abs() < 1e-5);
    }

    #[test]
    fn test_composite_remap_with_null_strategy() {
        let composite = CompositeZoomStrategy::new(vec![
            Box::new(NullZoomStrategy::new()),
        ]);
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
        let (rx, ry) = composite.remap_coordinates((0.5, 0.5), &region);
        assert!((rx - 0.5).abs() < 1e-5);
        assert!((ry - 0.5).abs() < 1e-5);
    }

    struct DummyZoomStrategy;
    impl ZoomStrategy for DummyZoomStrategy {
        fn get_zoomed_region<'a>(
            &'a self,
            _llm: &'a dyn LlmProvider,
            full_screenshot: &'a DynamicImage,
            _config: &'a crate::config::SavedConfig,
        ) -> BoxFuture<'a, Option<ZoomedRegion>> {
            Box::pin(async move {
                let ctx = ZoomContext {
                    crop_x: 0.1,
                    crop_y: 0.2,
                    crop_width: 0.5,
                    crop_height: 0.6,
                };
                Some(ZoomedRegion {
                    zoomed_image: full_screenshot.clone(),
                    original_rect: (10, 20, 50, 60),
                    crop_context: ctx,
                })
            })
        }
        fn remap_coordinates(&self, (zx, zy): (f64, f64), _region: &ZoomedRegion) -> (f64, f64) {
            (zx, zy)
        }
    }

    #[tokio::test]
    async fn test_composite_get_zoomed_region() {
        use crate::orchestrator::tests::MockLlmClient;
        use crate::llm_client::LlmResponse;
        use crate::vision::NullZoomStrategy;

        let llm = MockLlmClient {
            next_response: std::sync::Mutex::new(LlmResponse::default()),
            decompose_response: std::sync::Mutex::new(String::new()),
            call_count: std::sync::Mutex::new(0),
        };

        let composite_null = CompositeZoomStrategy::new(vec![
            Box::new(NullZoomStrategy::new()),
        ]);
        let img = DynamicImage::new_rgb8(100, 100);
        let config = crate::config::SavedConfig::default();

        let res_none = composite_null.get_zoomed_region(&llm, &img, &config).await;
        assert!(res_none.is_none());

        let composite_some = CompositeZoomStrategy::new(vec![
            Box::new(DummyZoomStrategy),
        ]);
        let res_some = composite_some.get_zoomed_region(&llm, &img, &config).await;
        assert!(res_some.is_some());
        let region = res_some.unwrap();
        assert_eq!(region.original_rect.0, 10);
        assert_eq!(region.original_rect.1, 20);
    }
}
