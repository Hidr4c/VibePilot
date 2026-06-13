//! Post-action reflection and visual validation module.
//!
//! Provides lightweight visual diff (SSIM-inspired) to detect whether an action
//! had a visible effect on the screen, without requiring an extra LLM call.
//! Only escalates to LLM-based reflection when the quick diff is inconclusive.
//!
//! # Architecture
//!
//! ```text
//! before_screenshot ──┐
//!                     ├── quick_visual_diff() ──→ VisualDiffResult
//! after_screenshot  ──┘                               │
//!                                          ┌──────────┤
//!                                          │          │
//!                                    Changed    Unchanged/Ambiguous
//!                                          │          │
//!                                    Success     evaluate() → LLM call
//! ```
//!
//! # Examples
//!
//! ```no_run
//! use vibe_pilot_rust::reflection::ReflectionModule;
//! use image::DynamicImage;
//!
//! let before = DynamicImage::new_rgba8(100, 100);
//! let after = DynamicImage::new_rgba8(100, 100);
//! let diff = ReflectionModule::quick_visual_diff(&before, &after);
//! ```

use image::DynamicImage;

/// Verdict from the reflection module after evaluating an action's effect.
#[derive(Debug, Clone, PartialEq)]
pub enum ReflectionVerdict {
    /// The action had a visible effect. Confidence is 0.0 to 1.0.
    Success { confidence: f32 },
    /// The action had no visible effect on the screen.
    NoEffect { suggestion: String },
    /// The screen changed in an unexpected way (e.g., error dialog appeared).
    UnexpectedChange { description: String },
    /// The screen state regressed (e.g., navigated away from the target).
    Regression { rollback_suggestion: String },
}

/// Result of a quick pixel-based visual comparison.
#[derive(Debug, Clone, PartialEq)]
pub enum VisualDiffResult {
    /// Significant visual change detected (> threshold).
    Changed { diff_ratio: f32 },
    /// No significant change detected.
    Unchanged { diff_ratio: f32 },
    /// Could not compare (different sizes, etc.).
    Incomparable,
}

/// Threshold for the percentage of changed pixels to consider a "significant" change.
const CHANGE_THRESHOLD: f32 = 0.003; // 0.3% of pixels changed

/// Per-pixel intensity difference threshold to count as "changed".
const PIXEL_DIFF_THRESHOLD: u8 = 30;

/// Module for post-action reflection and visual validation.
///
/// Uses a fast pixel-based diff to avoid unnecessary LLM calls.
/// Only escalates to full LLM evaluation when the diff is ambiguous
/// or the action was expected to have a specific outcome.
pub struct ReflectionModule;

impl ReflectionModule {
    /// Performs a fast pixel-based visual comparison between two screenshots.
    ///
    /// Downsamples both images to a common resolution (max 256×256) for speed,
    /// then compares per-pixel intensity differences.
    ///
    /// # Returns
    /// - `Changed { diff_ratio }` if more than [`CHANGE_THRESHOLD`] of pixels differ
    /// - `Unchanged { diff_ratio }` if fewer pixels differ
    /// - `Incomparable` if the images cannot be compared
    pub fn quick_visual_diff(before: &DynamicImage, after: &DynamicImage) -> VisualDiffResult {
        // Downscale both images for fast comparison
        let target_size = 256u32;
        let before_small = before.resize_exact(
            target_size, target_size,
            image::imageops::FilterType::Nearest,
        );
        let after_small = after.resize_exact(
            target_size, target_size,
            image::imageops::FilterType::Nearest,
        );

        let before_gray = before_small.to_luma8();
        let after_gray = after_small.to_luma8();

        let total_pixels = (target_size * target_size) as usize;
        if total_pixels == 0 {
            return VisualDiffResult::Incomparable;
        }

        let mut changed_pixels = 0usize;

        for (b_pixel, a_pixel) in before_gray.pixels().zip(after_gray.pixels()) {
            let diff = (b_pixel.0[0] as i16 - a_pixel.0[0] as i16).unsigned_abs() as u8;
            if diff > PIXEL_DIFF_THRESHOLD {
                changed_pixels += 1;
            }
        }

        let diff_ratio = changed_pixels as f32 / total_pixels as f32;

        if diff_ratio >= CHANGE_THRESHOLD {
            VisualDiffResult::Changed { diff_ratio }
        } else {
            VisualDiffResult::Unchanged { diff_ratio }
        }
    }

    /// Evaluates the effect of an action using the visual diff result
    /// and contextual information.
    ///
    /// This method does NOT call the LLM — it makes a local decision based on:
    /// - The visual diff result
    /// - The type of action performed
    /// - Whether the action was a retry
    ///
    /// For full LLM-based reflection, use `evaluate_with_llm()` (Phase 2).
    pub fn evaluate_local(
        diff: &VisualDiffResult,
        action_type: &str,
        was_retry: bool,
    ) -> ReflectionVerdict {
        match diff {
            VisualDiffResult::Changed { diff_ratio } => {
                if *diff_ratio > 0.5 {
                    // Massive change — might be unexpected (new page, error dialog)
                    ReflectionVerdict::Success {
                        confidence: if *diff_ratio > 0.8 { 0.6 } else { 0.8 },
                    }
                } else {
                    ReflectionVerdict::Success {
                        confidence: 0.9,
                    }
                }
            }
            VisualDiffResult::Unchanged { .. } => {
                match action_type {
                    "WAIT" => {
                        // WAIT actions are expected to have no visual change
                        ReflectionVerdict::Success { confidence: 1.0 }
                    }
                    "SCROLL" => {
                        // Scroll with no change might mean we're at the end
                        ReflectionVerdict::NoEffect {
                            suggestion: if was_retry {
                                "Scroll had no effect twice. Try scrolling in the other direction or clicking elsewhere.".into()
                            } else {
                                "Scroll had no visible effect. May be at the edge of the page.".into()
                            },
                        }
                    }
                    "CLICK_AND_TYPE" => {
                        ReflectionVerdict::NoEffect {
                            suggestion: if was_retry {
                                "Click had no effect after retry. The coordinates are off-target. You MUST adjust your target coordinates using the Axis-by-Axis calibration protocol (adjust X first while keeping Y same, then adjust Y if needed). Verify the red crosshair marker from your last attempt to see where you clicked relative to the target!".into()
                            } else {
                                "Click had no visible effect. The coordinates may be off-target. Use the Axis-by-Axis calibration protocol to adjust them (change only one axis first, e.g., X, then Y, then alternate). Check the red crosshair marker from your last attempt to verify alignment!".into()
                            },
                        }
                    }
                    _ => {
                        ReflectionVerdict::NoEffect {
                            suggestion: "Action had no visible effect.".into(),
                        }
                    }
                }
            }
            VisualDiffResult::Incomparable => {
                ReflectionVerdict::UnexpectedChange {
                    description: "Screenshots have different dimensions — screen configuration may have changed.".into(),
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reflection_verdict_equality() {
        let v1 = ReflectionVerdict::Success { confidence: 0.9 };
        let v2 = ReflectionVerdict::Success { confidence: 0.9 };
        assert_eq!(v1, v2);

        let v3 = ReflectionVerdict::NoEffect { suggestion: "test".into() };
        assert_ne!(v1, v3);
    }

    #[test]
    fn test_visual_diff_identical_images() {
        let img1 = DynamicImage::new_rgba8(100, 100);
        let img2 = DynamicImage::new_rgba8(100, 100);
        let result = ReflectionModule::quick_visual_diff(&img1, &img2);
        match result {
            VisualDiffResult::Unchanged { diff_ratio } => {
                assert!(diff_ratio < CHANGE_THRESHOLD);
            }
            _ => panic!("Expected Unchanged for identical images"),
        }
    }

    #[test]
    fn test_visual_diff_completely_different() {
        // Black image
        let img1 = DynamicImage::new_rgba8(100, 100);
        // White image
        let mut img2 = DynamicImage::new_rgba8(100, 100);
        if let Some(rgba) = img2.as_mut_rgba8() {
            for pixel in rgba.pixels_mut() {
                *pixel = image::Rgba([255, 255, 255, 255]);
            }
        }
        let result = ReflectionModule::quick_visual_diff(&img1, &img2);
        match result {
            VisualDiffResult::Changed { diff_ratio } => {
                assert!(diff_ratio > 0.5, "Expected high diff ratio, got {}", diff_ratio);
            }
            _ => panic!("Expected Changed for completely different images"),
        }
    }

    #[test]
    fn test_visual_diff_slight_change() {
        let img1 = DynamicImage::new_rgba8(100, 100);
        let mut img2 = DynamicImage::new_rgba8(100, 100);
        // Change just a few pixels (< 2%)
        if let Some(rgba) = img2.as_mut_rgba8() {
            rgba.put_pixel(50, 50, image::Rgba([255, 0, 0, 255]));
        }
        let result = ReflectionModule::quick_visual_diff(&img1, &img2);
        // A single pixel change on a 100x100 image should be well below threshold
        assert!(matches!(result, VisualDiffResult::Unchanged { .. }));
    }

    #[test]
    fn test_evaluate_local_click_changed() {
        let diff = VisualDiffResult::Changed { diff_ratio: 0.15 };
        let verdict = ReflectionModule::evaluate_local(&diff, "CLICK_AND_TYPE", false);
        assert!(matches!(verdict, ReflectionVerdict::Success { confidence } if confidence > 0.5));
    }

    #[test]
    fn test_evaluate_local_click_unchanged() {
        let diff = VisualDiffResult::Unchanged { diff_ratio: 0.001 };
        let verdict = ReflectionModule::evaluate_local(&diff, "CLICK_AND_TYPE", false);
        assert!(matches!(verdict, ReflectionVerdict::NoEffect { .. }));
    }

    #[test]
    fn test_evaluate_local_click_unchanged_retry() {
        let diff = VisualDiffResult::Unchanged { diff_ratio: 0.001 };
        let verdict = ReflectionModule::evaluate_local(&diff, "CLICK_AND_TYPE", true);
        match verdict {
            ReflectionVerdict::NoEffect { suggestion } => {
                assert!(suggestion.contains("retry"), "Expected retry-aware suggestion");
            }
            _ => panic!("Expected NoEffect for unchanged click retry"),
        }
    }

    #[test]
    fn test_evaluate_local_wait_unchanged() {
        let diff = VisualDiffResult::Unchanged { diff_ratio: 0.0 };
        let verdict = ReflectionModule::evaluate_local(&diff, "WAIT", false);
        assert!(matches!(verdict, ReflectionVerdict::Success { confidence: 1.0 }));
    }

    #[test]
    fn test_evaluate_local_scroll_unchanged() {
        let diff = VisualDiffResult::Unchanged { diff_ratio: 0.005 };
        let verdict = ReflectionModule::evaluate_local(&diff, "SCROLL", false);
        assert!(matches!(verdict, ReflectionVerdict::NoEffect { .. }));
    }

    #[test]
    fn test_evaluate_local_scroll_unchanged_retry() {
        let diff = VisualDiffResult::Unchanged { diff_ratio: 0.005 };
        let verdict = ReflectionModule::evaluate_local(&diff, "SCROLL", true);
        match verdict {
            ReflectionVerdict::NoEffect { suggestion } => {
                assert!(suggestion.contains("other direction"));
            }
            _ => panic!("Expected NoEffect for scroll retry"),
        }
    }

    #[test]
    fn test_evaluate_local_massive_change() {
        let diff = VisualDiffResult::Changed { diff_ratio: 0.85 };
        let verdict = ReflectionModule::evaluate_local(&diff, "CLICK_AND_TYPE", false);
        // Massive change should have lower confidence (might be unexpected)
        match verdict {
            ReflectionVerdict::Success { confidence } => {
                assert!(confidence < 0.8, "Expected lower confidence for massive change, got {}", confidence);
            }
            _ => panic!("Expected Success for massive change"),
        }
    }

    #[test]
    fn test_evaluate_local_incomparable() {
        let diff = VisualDiffResult::Incomparable;
        let verdict = ReflectionModule::evaluate_local(&diff, "CLICK_AND_TYPE", false);
        assert!(matches!(verdict, ReflectionVerdict::UnexpectedChange { .. }));
    }

    #[test]
    fn test_visual_diff_result_debug() {
        let r = VisualDiffResult::Changed { diff_ratio: 0.15 };
        assert!(format!("{:?}", r).contains("0.15"));
    }
}
