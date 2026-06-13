//! Semantic cache for zoom strategy results with TTL expiration.

use image::DynamicImage;
use std::collections::HashMap;
use std::time::Instant;

use crate::vision::ZoomedRegion;

/// Cached zoom result with expiration timestamp.
struct CachedRegion {
    region: ZoomedRegion,
    expires_at: Instant,
}

/// Semantic cache that avoids redundant LLM calls for similar regions.
///
/// Keys are computed from the target description + approximate screen zone hash.
pub struct ZoomCache {
    cache: HashMap<String, CachedRegion>,
    ttl_secs: u64,
}

impl ZoomCache {
    /// Creates a new zoom cache with the given TTL in seconds.
    pub fn new(ttl_secs: u64) -> Self {
        Self {
            cache: HashMap::new(),
            ttl_secs,
        }
    }

    /// Computes a cache key from a target description and approximate zone.
    pub fn make_key(description: &str, zone_hash: u64) -> String {
        format!("{}:{}", description.to_lowercase(), zone_hash)
    }

    /// Inserts a zoomed region into the cache.
    pub fn insert(&mut self, key: String, region: ZoomedRegion) {
        self.cache.insert(
            key,
            CachedRegion {
                region,
                expires_at: Instant::now() + std::time::Duration::from_secs(self.ttl_secs),
            },
        );
    }

    /// Retrieves a cached region if it exists and hasn't expired.
    pub fn get(&mut self, key: &str) -> Option<ZoomedRegion> {
        if let Some(entry) = self.cache.get(key) {
            if entry.expires_at > Instant::now() {
                return Some(entry.region.clone());
            }
            self.cache.remove(key);
        }
        None
    }

    /// Returns `true` if the cache has no entries.
    pub fn is_empty(&self) -> bool {
        self.cache.is_empty()
    }

    /// Evicts expired entries.
    pub fn evict_expired(&mut self) {
        let now = Instant::now();
        self.cache.retain(|_, entry| entry.expires_at > now);
    }
}

impl Default for ZoomCache {
    fn default() -> Self {
        Self::new(30) // 30-second default TTL
    }
}

/// Computes Shannon entropy approximation from a grayscale image histogram.
///
/// Higher entropy (> 4.0) indicates a complex texture region that may benefit
/// from zoom-based refinement.
pub fn compute_entropy(image: &DynamicImage) -> f64 {
    let gray = image.to_luma8();
    let total_pixels = (gray.width() * gray.height()) as f64;

    if total_pixels == 0.0 {
        return 0.0;
    }

    // Build histogram (256 bins for 8-bit grayscale)
    let mut histogram = [0u32; 256];
    for pixel in gray.pixels() {
        histogram[pixel[0] as usize] += 1;
    }

    // Compute Shannon entropy
    let mut entropy = 0.0f64;
    for &count in &histogram {
        if count > 0 {
            let p = count as f64 / total_pixels;
            entropy -= p * p.log2();
        }
    }

    entropy
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::DynamicImage;

    #[test]
    fn test_zoom_cache_insert_and_get() {
        let mut cache = ZoomCache::new(60);
        let key = ZoomCache::make_key("button", 123);
        let region = ZoomedRegion {
            zoomed_image: DynamicImage::new_rgb8(10, 10),
            original_rect: (0, 0, 10, 10),
            crop_context: crate::vision::ZoomContext {
                crop_x: 0.1,
                crop_y: 0.2,
                crop_width: 0.5,
                crop_height: 0.6,
            },
        };
        cache.insert(key.clone(), region);
        assert!(cache.get(&key).is_some());
        assert!(!cache.is_empty());
    }

    #[test]
    fn test_zoom_cache_expired() {
        let mut cache = ZoomCache::new(1);
        let key = ZoomCache::make_key("button", 123);
        let region = ZoomedRegion {
            zoomed_image: DynamicImage::new_rgb8(10, 10),
            original_rect: (0, 0, 10, 10),
            crop_context: crate::vision::ZoomContext {
                crop_x: 0.0,
                crop_y: 0.0,
                crop_width: 1.0,
                crop_height: 1.0,
            },
        };
        cache.insert(key.clone(), region);

        // Simulate TTL expiration
        std::thread::sleep(std::time::Duration::from_secs(2));
        assert!(cache.get(&key).is_none());
        assert!(cache.is_empty());
    }

    #[test]
    fn test_zoom_cache_evict_expired() {
        let mut cache = ZoomCache::new(60);
        let key1 = ZoomCache::make_key("button", 1);
        let key2 = ZoomCache::make_key("input", 2);
        let region = ZoomedRegion {
            zoomed_image: DynamicImage::new_rgb8(10, 10),
            original_rect: (0, 0, 10, 10),
            crop_context: crate::vision::ZoomContext {
                crop_x: 0.0,
                crop_y: 0.0,
                crop_width: 1.0,
                crop_height: 1.0,
            },
        };
        cache.insert(key1.clone(), region.clone());
        cache.insert(key2.clone(), region);

        // Manually expire key1 by setting a past timestamp
        if let Some(entry) = cache.cache.get_mut(&key1) {
            entry.expires_at = Instant::now() - std::time::Duration::from_secs(1);
        }

        cache.evict_expired();
        // key2 should still be present
        assert!(cache.cache.get(&key2).is_some());
        // key1 should be removed
        assert!(cache.cache.get(&key1).is_none());
    }

    #[test]
    fn test_zoom_cache_make_key() {
        let key1 = ZoomCache::make_key("Button", 123);
        let key2 = ZoomCache::make_key("button", 123);
        assert_eq!(key1, key2); // Case-insensitive
    }

    #[test]
    fn test_compute_entropy_uniform_image() {
        // Uniform red image should have low entropy
        let mut img = DynamicImage::new_rgb8(50, 50);
        if let Some(rgb) = img.as_mut_rgb8() {
            for y in 0..50 {
                for x in 0..50 {
                    rgb.put_pixel(x, y, image::Rgb([128, 128, 128]));
                }
            }
        }
        let entropy = compute_entropy(&img);
        assert_eq!(entropy, 0.0); // Uniform = zero entropy
    }

    #[test]
    fn test_compute_entropy_gradient_image() {
        // Gradient should have higher entropy
        let mut img = DynamicImage::new_rgb8(50, 50);
        if let Some(rgb) = img.as_mut_rgb8() {
            for y in 0..50 {
                for x in 0..50 {
                    let val = ((x + y) * 2) as u8;
                    rgb.put_pixel(x, y, image::Rgb([val, val, val]));
                }
            }
        }
        let entropy = compute_entropy(&img);
        assert!(entropy > 0.0);
    }

    #[test]
    fn test_compute_entropy_empty_image() {
        let img = DynamicImage::new_rgb8(0, 0);
        let entropy = compute_entropy(&img);
        assert_eq!(entropy, 0.0);
    }

    #[test]
    fn test_zoom_cache_default_ttl() {
        let cache = ZoomCache::default();
        assert_eq!(cache.ttl_secs, 30);
    }
}
