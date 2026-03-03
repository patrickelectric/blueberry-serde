//! Ordinal-aware optional field for interleaved backward compatibility.
//!
//! [`OptionalField<ORD, T>`] wraps an `Option<T>` and carries a const ordinal
//! that the deserializer checks against `payload_field_count` to decide whether
//! the field is present (`Some`) or absent (`None`).
//!
//! # Example
//!
//! ```rust
//! use blueberry_serde::OptionalField;
//! use serde::{Serialize, Deserialize};
//!
//! #[derive(Serialize, Deserialize)]
//! struct Sensor {
//!     a: u8,
//!     c: OptionalField<3, u8>,
//!     b: u32,
//! }
//! ```

use serde::{de, ser};
use std::fmt;
use std::marker::PhantomData;
use std::ops::Deref;

/// Sentinel name passed to `deserialize_tuple_struct` so the deserializer can
/// distinguish ordinal-checked optional fields from regular tuple structs.
pub const OPTIONAL_FIELD_MARKER: &str = "__blueberry_optional";

/// An optional field that carries a const ordinal for presence checks during
/// deserialization.
///
/// Use this for fields that may not exist in older message versions, especially
/// when they are interleaved between other required fields (not just trailing).
///
/// The ordinal should equal `non_optional_field_count + 1` for the first
/// optional field, incrementing by one for each subsequent version.
pub struct OptionalField<const ORD: usize, T>(pub Option<T>);

impl<const ORD: usize, T> OptionalField<ORD, T> {
    pub fn some(value: T) -> Self {
        Self(Some(value))
    }

    pub fn none() -> Self {
        Self(None)
    }
}

// --- Trait implementations ---

impl<const ORD: usize, T: fmt::Debug> fmt::Debug for OptionalField<ORD, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl<const ORD: usize, T: PartialEq> PartialEq for OptionalField<ORD, T> {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl<const ORD: usize, T: Clone> Clone for OptionalField<ORD, T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<const ORD: usize, T> Default for OptionalField<ORD, T> {
    fn default() -> Self {
        Self(None)
    }
}

impl<const ORD: usize, T> Deref for OptionalField<ORD, T> {
    type Target = Option<T>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<const ORD: usize, T> From<Option<T>> for OptionalField<ORD, T> {
    fn from(opt: Option<T>) -> Self {
        Self(opt)
    }
}

impl<const ORD: usize, T> From<T> for OptionalField<ORD, T> {
    fn from(value: T) -> Self {
        Self(Some(value))
    }
}

impl<const ORD: usize, T> From<OptionalField<ORD, T>> for Option<T> {
    fn from(field: OptionalField<ORD, T>) -> Self {
        field.0
    }
}

// --- Serialize ---

impl<const ORD: usize, T: ser::Serialize> ser::Serialize for OptionalField<ORD, T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ser::Serializer,
    {
        match &self.0 {
            Some(v) => serializer.serialize_some(v),
            None => serializer.serialize_none(),
        }
    }
}

// --- Deserialize ---

impl<'de, const ORD: usize, T> de::Deserialize<'de> for OptionalField<ORD, T>
where
    T: de::Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        deserializer.deserialize_tuple_struct(
            OPTIONAL_FIELD_MARKER,
            ORD,
            OptionalFieldVisitor::<ORD, T>(PhantomData),
        )
    }
}

struct OptionalFieldVisitor<const ORD: usize, T>(PhantomData<T>);

impl<'de, const ORD: usize, T> de::Visitor<'de> for OptionalFieldVisitor<ORD, T>
where
    T: de::Deserialize<'de>,
{
    type Value = OptionalField<ORD, T>;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "optional field with ordinal {}", ORD)
    }

    fn visit_none<E: de::Error>(self) -> Result<Self::Value, E> {
        Ok(OptionalField(None))
    }

    fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        let value = T::deserialize(deserializer)?;
        Ok(OptionalField(Some(value)))
    }
}
