use std::sync::Arc;
use image::DynamicImage;

/// Trait defining mouse action coordinate auto-calibration policies.
pub trait AutoCalibrator: Send + Sync {
    /// Compares screen captures before and after an action to locate the pixel region that changed
    /// the most and computes its offset drift from target coordinates.
    /// Returns Some((dx, dy)) drift if confident, or None if inconclusive.
    fn calculate_drift(
        &self,
        before: &DynamicImage,
        after: &DynamicImage,
        target_click: (u32, u32),
    ) -> Option<(i32, i32)>;
}

/// Dynamic mouse coordinate auto-calibration based on visual differences.
pub struct VisualDiffAutoCalibrator;

impl VisualDiffAutoCalibrator {
    /// Creates a new VisualDiffAutoCalibrator instance.
    pub fn new() -> Self {
        Self
    }
}

impl Default for VisualDiffAutoCalibrator {
    fn default() -> Self {
        Self::new()
    }
}

impl AutoCalibrator for VisualDiffAutoCalibrator {
    fn calculate_drift(
        &self,
        before: &DynamicImage,
        after: &DynamicImage,
        target_click: (u32, u32),
    ) -> Option<(i32, i32)> {
        if before.width() != after.width() || before.height() != after.height() {
            return None;
        }

        let before_gray = before.to_luma8();
        let after_gray = after.to_luma8();

        let width = before_gray.width();
        let height = before_gray.height();

        let target_x = target_click.0;
        let target_y = target_click.1;

        // Search in a local 100-pixel bounding box around target coordinate
        let search_radius = 100u32;
        let min_x = target_x.saturating_sub(search_radius);
        let max_x = (target_x + search_radius).min(width);
        let min_y = target_y.saturating_sub(search_radius);
        let max_y = (target_y + search_radius).min(height);

        let mut sum_x = 0u64;
        let mut sum_y = 0u64;
        let mut count = 0u64;

        for y in min_y..max_y {
            for x in min_x..max_x {
                let b_pixel = before_gray.get_pixel(x, y).0[0];
                let a_pixel = after_gray.get_pixel(x, y).0[0];
                let diff = (b_pixel as i16 - a_pixel as i16).unsigned_abs();

                if diff > 40 {
                    sum_x += x as u64;
                    sum_y += y as u64;
                    count += 1;
                }
            }
        }

        // Confident match if changed pixel count is a local cluster (10 to 2000 pixels)
        if count >= 10 && count <= 2000 {
            let actual_x = (sum_x / count) as u32;
            let actual_y = (sum_y / count) as u32;

            let dx = target_x as i32 - actual_x as i32;
            let dy = target_y as i32 - actual_y as i32;

            // Only apply calibration if drift is small (<= 30px)
            if dx.abs() <= 30 && dy.abs() <= 30 {
                return Some((dx, dy));
            }
        }

        None
    }
}

/// Factory to construct AutoCalibrator implementations following the IoC pattern.
pub struct AutoCalibratorFactory;

impl AutoCalibratorFactory {
    /// Creates an AutoCalibrator instance.
    pub fn create() -> Arc<dyn AutoCalibrator> {
        Arc::new(VisualDiffAutoCalibrator::new())
    }
}
