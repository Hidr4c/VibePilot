//! Vision pipeline with Strategy pattern for zoom-based coordinate precision.
//!
//! Provides a flexible zoom strategy system that allows:
//! - LLM-based region-of-interest detection (high precision)
//! - Null/no-op fallback (zero overhead)
//! - Chaining multiple strategies together
//!
//! # Architecture
//!
//! - `ZoomStrategy` trait defines the interface for zoom operations.
//! - `LlmZoomStrategy` uses the LLM to detect target regions.
//! - `NullZoomStrategy` is the default no-op implementation.
//! - `CompositeZoomStrategy` chains multiple strategies.
//!
//! # Examples
//!
//! ```
//! use vibe_pilot_rust::vision::{
//!     ZoomStrategy, LlmZoomStrategy, CompositeZoomStrategy,
//!     ZoomedRegion, ZoomContext,
//! };
//!
//! // Create a composite with LLM strategy
//! let strategy = CompositeZoomStrategy::new(vec![
//!     Box::new(LlmZoomStrategy::new(2.0)),
//! ]);
//! ```

use image::DynamicImage;
use std::pin::Pin;
use std::sync::Arc;
use std::future::Future;
use crate::llm_client::LlmProvider;

/// A zoomed region of interest with its original coordinates.
#[derive(Debug, Clone)]
pub struct ZoomedRegion {
    /// The cropped and upscaled image.
    pub zoomed_image: DynamicImage,
    /// Original image region: (x, y, width, height) in absolute pixels.
    pub original_rect: (u32, u32, u32, u32),
    /// The crop context used to compute this region.
    pub crop_context: ZoomContext,
}

/// Context for coordinate remapping between zoomed and original spaces.
#[derive(Debug, Clone)]
pub struct ZoomContext {
    /// Relative X offset of the cropped region on the original image (0.0 to 1.0).
    pub crop_x: f64,
    /// Relative Y offset of the cropped region on the original image (0.0 to 1.0).
    pub crop_y: f64,
    /// Relative width of the cropped region on the original image (0.0 to 1.0).
    pub crop_width: f64,
    /// Relative height of the cropped region on the original image (0.0 to 1.0).
    pub crop_height: f64,
}

/// Future type alias for zoom strategy async operations.
pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// Strategy trait for zoom-based region detection.
///
/// Implementors detect a region of interest on a screenshot and return
/// a zoomed crop with coordinate mapping context.
///
/// This trait is designed to be dyn-compatible using `BoxFuture`.
pub trait ZoomStrategy: Send + Sync {
    /// Detects a region of interest and returns a zoomed crop.
    ///
    /// Returns `None` if the strategy cannot find a region (e.g., LLM failed,
    /// no OCR backend available).
    fn get_zoomed_region<'a>(
        &'a self,
        llm: &'a dyn LlmProvider,
        full_screenshot: &'a DynamicImage,
        config: &'a crate::config::SavedConfig,
    ) -> BoxFuture<'a, Option<ZoomedRegion>>;

    /// Converts relative coordinates on a zoomed image back to the original image.
    fn remap_coordinates(&self, zoomed_coords: (f64, f64), region: &ZoomedRegion) -> (f64, f64);
}

pub use null_strategy::NullZoomStrategy;
pub use llm_strategy::LlmZoomStrategy;
pub use composite_strategy::CompositeZoomStrategy;
pub use zoom_cache::{ZoomCache, compute_entropy};

pub mod null_strategy;
pub mod llm_strategy;
pub mod composite_strategy;
pub mod zoom_cache;
