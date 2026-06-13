//! Selective compression for large text blobs in persistence storage.
//!
//! Provides compression and decompression functions that can be used as
//! serde attributes to selectively compress large data structures.
//!
//! # Architecture
//!
//! Uses zstd for fast compression and rmp-serde for binary serialization.
//! The compression level is set to 3 (balanced speed/size).
//!
//! # Examples
//!
//! ```ignore
//! use vibe_pilot_rust::persistence::compression::{compress_blob, decompress_blob};
//! use serde::{Serialize, Deserialize};
//!
//! #[derive(Serialize, Deserialize)]
//! struct MyData {
//!     #[serde(serialize_with = "compress_blob", deserialize_with = "decompress_blob")]
//!     large_field: Vec<u8>,
//! }
//! ```

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::io::Cursor;

/// Compresses a serializable data structure using zstd.
///
/// Serializes the data with rmp-serde, then compresses with zstd level 3.
/// Returns the compressed bytes via the serializer.
///
/// # Errors
/// Returns a serde serialization error if compression or serialization fails.
pub fn compress_blob<S, T>(data: &T, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
    T: Serialize,
{
    let serialized = rmp_serde::to_vec(data).map_err(serde::ser::Error::custom)?;
    let compressed = zstd::encode_all(&serialized[..], 3).map_err(serde::ser::Error::custom)?;
    serializer.serialize_bytes(&compressed)
}

/// Decompresses data that was compressed with `compress_blob`.
///
/// Decompresses zstd data and deserializes with rmp-serde.
///
/// # Errors
/// Returns a serde deserialization error if decompression or deserialization fails.
pub fn decompress_blob<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    let compressed = Vec::<u8>::deserialize(deserializer)?;
    let decompressed = zstd::decode_all(&compressed[..]).map_err(serde::de::Error::custom)?;
    rmp_serde::from_slice(&decompressed).map_err(serde::de::Error::custom)
}

/// Checks if compression would be beneficial for the given data.
///
/// Returns `true` if the estimated compressed size would be smaller than
/// the original serialized size.
pub fn should_compress<T>(data: &T) -> bool
where
    T: Serialize,
{
    let serialized = rmp_serde::to_vec(data).unwrap_or_default();
    if serialized.len() < 256 {
        return false;
    }
    if let Ok(compressed) = zstd::encode_all(&serialized[..], 3) {
        compressed.len() < serialized.len()
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    struct TestData {
        name: String,
        values: Vec<f64>,
    }

    #[test]
    fn test_compress_decompress_roundtrip() {
        let data = TestData {
            name: "test".to_string(),
            values: vec![1.0, 2.0, 3.0, 4.0, 5.0],
        };

        let serialized = rmp_serde::to_vec(&data).unwrap();
        let compressed = zstd::encode_all(&serialized, 3).unwrap();
        let decompressed = zstd::decode_all(&compressed).unwrap();
        let result: TestData = rmp_serde::from_slice(&decompressed).unwrap();

        assert_eq!(data, result);
    }

    #[test]
    fn test_compress_blob_serde() {
        let data = TestData {
            name: "serde_test".to_string(),
            values: vec![0.5, 1.5, 2.5],
        };

        let mut buf = Vec::new();
        {
            let serializer = serde_serial::Serializer::new(&mut buf);
            compress_blob(&data, serializer).unwrap();
        }
        assert!(!buf.is_empty());
    }

    #[test]
    fn test_should_compress_small_data() {
        let small = vec![1u8, 2, 3, 4, 5];
        assert!(!should_compress(&small));
    }

    #[test]
    fn test_should_compress_large_data() {
        let large = vec![0xABu8; 1024];
        assert!(should_compress(&large));
    }

    #[test]
    fn test_compress_decompress_large_vec() {
        let data = TestData {
            name: "large".to_string(),
            values: (0..10000).map(|i| i as f64 * 0.1).collect(),
        };

        let serialized = rmp_serde::to_vec(&data).unwrap();
        let compressed = zstd::encode_all(&serialized, 3).unwrap();
        let ratio = compressed.len() as f64 / serialized.len() as f64;
        assert!(ratio < 0.5, "Compression ratio {} should be < 0.5", ratio);
    }

    #[test]
    fn test_compress_decompress_with_special_chars() {
        let data = TestData {
            name: "caf\u00e9 na\u00efve r\u00e9sum\u00e9".to_string(),
            values: vec![1.0, -0.5, 0.0],
        };

        let serialized = rmp_serde::to_vec(&data).unwrap();
        let compressed = zstd::encode_all(&serialized, 3).unwrap();
        let decompressed = zstd::decode_all(&compressed).unwrap();
        let result: TestData = rmp_serde::from_slice(&decompressed).unwrap();

        assert_eq!(data.name, result.name);
        assert_eq!(data.values, result.values);
    }

    #[test]
    fn test_compress_empty_vec() {
        let data = TestData {
            name: "empty".to_string(),
            values: vec![],
        };

        let serialized = rmp_serde::to_vec(&data).unwrap();
        let compressed = zstd::encode_all(&serialized, 3).unwrap();
        let decompressed = zstd::decode_all(&compressed).unwrap();
        let result: TestData = rmp_serde::from_slice(&decompressed).unwrap();

        assert_eq!(data, result);
    }
}

mod serde_serial {
    use serde::ser::SerializeStruct;
    use serde::{Serialize, Serializer};

    pub struct Serializer<S> {
        inner: S,
    }

    impl<S> Serializer<S> {
        pub fn new(inner: S) -> Self { Self { inner } }
    }

    impl<S: serde::Serializer> serde::Serializer for Serializer<S> {
        type Ok = S::Ok;
        type Error = S::Error;
        type SerializeSeq = S::SerializeSeq;
        type SerializeTuple = S::SerializeTuple;
        type SerializeTupleStruct = S::SerializeTupleStruct;
        type SerializeTupleVariant = S::SerializeTupleVariant;
        type SerializeMap = S::SerializeMap;
        type SerializeStruct = S::SerializeStruct;
        type SerializeStructVariant = S::SerializeStructVariant;

        fn serialize_bool(self, v: bool) -> Result<Self::Ok, Self::Error> {
            self.inner.serialize_bool(v)
        }
        fn serialize_i8(self, v: i8) -> Result<Self::Ok, Self::Error> {
            self.inner.serialize_i8(v)
        }
        fn serialize_i16(self, v: i16) -> Result<Self::Ok, Self::Error> {
            self.inner.serialize_i16(v)
        }
        fn serialize_i32(self, v: i32) -> Result<Self::Ok, Self::Error> {
            self.inner.serialize_i32(v)
        }
        fn serialize_i64(self, v: i64) -> Result<Self::Ok, Self::Error> {
            self.inner.serialize_i64(v)
        }
        fn serialize_u8(self, v: u8) -> Result<Self::Ok, Self::Error> {
            self.inner.serialize_u8(v)
        }
        fn serialize_u16(self, v: u16) -> Result<Self::Ok, Self::Error> {
            self.inner.serialize_u16(v)
        }
        fn serialize_u32(self, v: u32) -> Result<Self::Ok, Self::Error> {
            self.inner.serialize_u32(v)
        }
        fn serialize_u64(self, v: u64) -> Result<Self::Ok, Self::Error> {
            self.inner.serialize_u64(v)
        }
        fn serialize_f32(self, v: f32) -> Result<Self::Ok, Self::Error> {
            self.inner.serialize_f32(v)
        }
        fn serialize_f64(self, v: f64) -> Result<Self::Ok, Self::Error> {
            self.inner.serialize_f64(v)
        }
        fn serialize_char(self, v: char) -> Result<Self::Ok, Self::Error> {
            self.inner.serialize_char(v)
        }
        fn serialize_str(self, v: &str) -> Result<Self::Ok, Self::Error> {
            self.inner.serialize_str(v)
        }
        fn serialize_bytes(self, v: &[u8]) -> Result<Self::Ok, Self::Error> {
            self.inner.serialize_bytes(v)
        }
        fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
            self.inner.serialize_none()
        }
        fn serialize_some<T: ?Sized>(self, value: &T) -> Result<Self::Ok, Self::Error>
        where
            T: Serialize,
        {
            self.inner.serialize_some(value)
        }
        fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
            self.inner.serialize_unit()
        }
        fn serialize_unit_struct(self, name: &'static str) -> Result<Self::Ok, Self::Error> {
            self.inner.serialize_unit_struct(name)
        }
        fn serialize_unit_variant(
            self,
            name: &'static str,
            variant_index: u32,
            variant: &'static str,
        ) -> Result<Self::Ok, Self::Error> {
            self.inner.serialize_unit_variant(name, variant_index, variant)
        }
        fn serialize_newtype_struct<T: ?Sized>(
            self,
            name: &'static str,
            value: &T,
        ) -> Result<Self::Ok, Self::Error>
        where
            T: Serialize,
        {
            self.inner.serialize_newtype_struct(name, value)
        }
        fn serialize_newtype_variant<T: ?Sized>(
            self,
            name: &'static str,
            variant_index: u32,
            variant: &'static str,
            value: &T,
        ) -> Result<Self::Ok, Self::Error>
        where
            T: Serialize,
        {
            self.inner.serialize_newtype_variant(name, variant_index, variant, value)
        }
        fn serialize_seq(self, len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
            self.inner.serialize_seq(len)
        }
        fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple, Self::Error> {
            self.inner.serialize_tuple(len)
        }
        fn serialize_tuple_struct(
            self,
            name: &'static str,
            len: usize,
        ) -> Result<Self::SerializeTupleStruct, Self::Error> {
            self.inner.serialize_tuple_struct(name, len)
        }
        fn serialize_tuple_variant(
            self,
            name: &'static str,
            variant_index: u32,
            variant: &'static str,
            len: usize,
        ) -> Result<Self::SerializeTupleVariant, Self::Error> {
            self.inner.serialize_tuple_variant(name, variant_index, variant, len)
        }
        fn serialize_map(self, len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
            self.inner.serialize_map(len)
        }
        fn serialize_struct(
            self,
            name: &'static str,
            len: usize,
        ) -> Result<Self::SerializeStruct, Self::Error> {
            self.inner.serialize_struct(name, len)
        }
        fn serialize_struct_variant(
            self,
            name: &'static str,
            variant_index: u32,
            variant: &'static str,
            len: usize,
        ) -> Result<Self::SerializeStructVariant, Self::Error> {
            self.inner.serialize_struct_variant(name, variant_index, variant, len)
        }
        fn collect_str<T: ?Sized>(self, value: &T) -> Result<Self::Ok, Self::Error>
        where
            T: std::fmt::Display,
        {
            self.inner.collect_str(value)
        }
        fn is_human_readable(&self) -> bool {
            self.inner.is_human_readable()
        }
    }
}
