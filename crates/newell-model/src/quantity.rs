// Newell - Parametric CAD Software
// Copyright (c) 2026 Martin Harambat
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//! Finite engineering quantities used at the model boundary.

#[cfg(test)]
use std::f64::consts::PI;

use crate::ModelError;

/// Finite signed length stored in canonical millimetres.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Length(f64);
impl Length {
    /// Canonical positive zero, also used by newly opened command drafts.
    pub const ZERO: Self = Self(0.0);
    /// Constructs a length without rounding; negative zero becomes positive zero.
    ///
    /// # Errors
    /// Returns [`ModelError::NonFiniteQuantity`] for NaN or infinity.
    pub fn millimetres(value: f64) -> Result<Self, ModelError> {
        finite(value)?;
        Ok(Self(if value == 0.0 { 0.0 } else { value }))
    }
    /// Returns the original canonical value, independent of display precision.
    #[must_use]
    pub const fn as_millimetres(self) -> f64 {
        self.0
    }
    /// Formats one fractional digit for presentation; never use this to store a literal.
    #[must_use]
    pub fn display(self) -> String {
        format!("{:.1} mm", self.0)
    }
}

/// Finite signed angle stored in canonical radians.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Angle(f64);
impl Angle {
    /// Constructs a canonical angle without rounding, normalizing negative zero.
    ///
    /// # Errors
    /// Returns [`ModelError::NonFiniteQuantity`] for NaN or infinity.
    pub fn radians(value: f64) -> Result<Self, ModelError> {
        finite(value)?;
        Ok(Self(if value == 0.0 { 0.0 } else { value }))
    }
    /// Converts a finite degree value to canonical radians.
    ///
    /// # Errors
    /// Rejects nonfinite input, conversion overflow, or nonzero input converted to zero.
    pub fn degrees(value: f64) -> Result<Self, ModelError> {
        finite(value)?;
        let converted = value.to_radians();
        if !converted.is_finite() {
            return Err(ModelError::QuantityOverflow);
        }
        if value != 0.0 && converted == 0.0 {
            return Err(ModelError::QuantityUnderflow);
        }
        Self::radians(converted)
    }
    /// Returns the stored canonical radians without display rounding.
    #[must_use]
    pub const fn as_radians(self) -> f64 {
        self.0
    }
    /// Converts the stored radians to degrees without display rounding.
    ///
    /// # Errors
    /// Returns [`ModelError::QuantityOverflow`] if the result is not finite.
    pub fn as_degrees(self) -> Result<f64, ModelError> {
        let value = self.0.to_degrees();
        if !value.is_finite() {
            return Err(ModelError::QuantityOverflow);
        }
        Ok(value)
    }
}

/// Finite signed dimensionless value.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Scalar(f64);
impl Scalar {
    /// Constructs a scalar without rounding, normalizing negative zero.
    ///
    /// # Errors
    /// Returns [`ModelError::NonFiniteQuantity`] for NaN or infinity.
    pub fn new(value: f64) -> Result<Self, ModelError> {
        finite(value)?;
        Ok(Self(if value == 0.0 { 0.0 } else { value }))
    }
    /// Returns the stored dimensionless value.
    #[must_use]
    pub const fn value(self) -> f64 {
        self.0
    }
}

/// Finite literal carrying its dimension and full canonical precision.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Quantity {
    /// Canonical millimetres.
    Length(Length),
    /// Canonical radians.
    Angle(Angle),
    /// Dimensionless value.
    Scalar(Scalar),
}
/// Dimension required by a parameter destination.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuantityKind {
    /// Length dimension.
    Length,
    /// Angular dimension.
    Angle,
    /// No physical dimension.
    Scalar,
}
/// Supported literal and presentation units; radians remain an internal angular unit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Unit {
    /// Length, with optional exact `mm` suffix at the literal boundary.
    Millimetre,
    /// Angle, with optional exact `deg` suffix at the literal boundary.
    Degree,
    /// Scalar, accepting no suffix.
    Unitless,
}
/// Orientation metadata independent of a quantity's magnitude or formula source.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Direction {
    /// Uses the destination's forward reference orientation.
    #[default]
    Forward,
    /// Reverses the destination's reference orientation without negating the quantity.
    Reverse,
}

impl Quantity {
    /// Returns the dimension carried by this literal.
    #[must_use]
    pub const fn kind(self) -> QuantityKind {
        match self {
            Self::Length(_) => QuantityKind::Length,
            Self::Angle(_) => QuantityKind::Angle,
            Self::Scalar(_) => QuantityKind::Scalar,
        }
    }
    /// Parses a signed decimal or scientific literal using `unit` when its suffix is omitted.
    ///
    /// Only ASCII end whitespace, numeric syntax, and the exact `mm`/`deg` suffixes are
    /// accepted. No expression is evaluated. Field magnitude and range rules belong to the
    /// command boundary, so this API also permits signed values and greater precision.
    ///
    /// # Errors
    /// Distinguishes malformed syntax, unsupported units, dimension mismatch, nonfinite input,
    /// overflow, and nonzero input that underflows to zero.
    pub fn parse_literal(source: &str, unit: Unit) -> Result<Self, ModelError> {
        let trimmed = source.trim_matches(|c: char| c.is_ascii_whitespace());
        if matches!(
            trimmed.split_ascii_whitespace().next(),
            Some(
                "NaN"
                    | "+NaN"
                    | "-NaN"
                    | "inf"
                    | "+inf"
                    | "-inf"
                    | "Infinity"
                    | "+Infinity"
                    | "-Infinity"
            )
        ) {
            return Err(ModelError::NonFiniteQuantity);
        }
        let (number, suffix) = parse_parts(source)?;
        let expected = match unit {
            Unit::Millimetre => "mm",
            Unit::Degree => "deg",
            Unit::Unitless => "",
        };
        if !suffix.is_empty() && !suffix.bytes().all(|c| c.is_ascii_alphabetic()) {
            return Err(ModelError::InvalidLiteral(source.to_owned()));
        }
        if !suffix.is_empty() && !matches!(suffix, "mm" | "deg") {
            return Err(ModelError::UnsupportedUnit(suffix.to_owned()));
        }
        if !suffix.is_empty() && suffix != expected {
            return Err(ModelError::DimensionMismatch);
        }
        let value: f64 = number
            .parse()
            .map_err(|_| ModelError::InvalidLiteral(source.to_owned()))?;
        if !value.is_finite() {
            return Err(ModelError::QuantityOverflow);
        }
        let mantissa = number.split(['e', 'E']).next().unwrap_or(number);
        if value == 0.0 && mantissa.bytes().any(|b| matches!(b, b'1'..=b'9')) {
            return Err(ModelError::QuantityUnderflow);
        }
        match unit {
            Unit::Millimetre => Ok(Self::Length(Length::millimetres(value)?)),
            Unit::Degree => Ok(Self::Angle(Angle::degrees(value)?)),
            Unit::Unitless => Ok(Self::Scalar(Scalar::new(value)?)),
        }
    }
    /// Returns an unrounded value in a compatible unit.
    ///
    /// # Errors
    /// Returns [`ModelError::DimensionMismatch`] even for mismatched zero quantities, or
    /// [`ModelError::QuantityOverflow`] when conversion to degrees overflows.
    pub fn value_in(self, unit: Unit) -> Result<f64, ModelError> {
        match (self, unit) {
            (Self::Length(v), Unit::Millimetre) => Ok(v.0),
            (Self::Angle(v), Unit::Degree) => v.as_degrees(),
            (Self::Scalar(v), Unit::Unitless) => Ok(v.0),
            _ => Err(ModelError::DimensionMismatch),
        }
    }
    /// Formats one fractional digit and its suffix for presentation, without changing storage.
    ///
    /// # Errors
    /// Propagates the compatibility and conversion errors from [`Self::value_in`].
    pub fn display(self, unit: Unit) -> Result<String, ModelError> {
        let value = self.value_in(unit)?;
        Ok(match unit {
            Unit::Millimetre => format!("{value:.1} mm"),
            Unit::Degree => format!("{value:.1} deg"),
            Unit::Unitless => format!("{value:.1}"),
        })
    }
}

fn finite(value: f64) -> Result<(), ModelError> {
    if value.is_finite() {
        Ok(())
    } else {
        Err(ModelError::NonFiniteQuantity)
    }
}

fn parse_parts(source: &str) -> Result<(&str, &str), ModelError> {
    let s = source.trim_matches(|c: char| c.is_ascii_whitespace());
    let mut end = 0;
    let b = s.as_bytes();
    if b.first().is_some_and(|c| *c == b'+' || *c == b'-') {
        end += 1;
    }
    let mut digits = 0;
    while b.get(end).is_some_and(u8::is_ascii_digit) {
        end += 1;
        digits += 1;
    }
    if b.get(end) == Some(&b'.') {
        end += 1;
        while b.get(end).is_some_and(u8::is_ascii_digit) {
            end += 1;
            digits += 1;
        }
    }
    if digits == 0 {
        return Err(ModelError::InvalidLiteral(source.to_owned()));
    }
    if b.get(end).is_some_and(|c| *c == b'e' || *c == b'E') {
        end += 1;
        if b.get(end).is_some_and(|c| *c == b'+' || *c == b'-') {
            end += 1;
        }
        let exp_start = end;
        while b.get(end).is_some_and(u8::is_ascii_digit) {
            end += 1;
        }
        if end == exp_start {
            return Err(ModelError::InvalidLiteral(source.to_owned()));
        }
    }
    let number = &s[..end];
    let rest = s[end..].trim_matches(|c: char| c.is_ascii_whitespace());
    if rest.chars().any(|c: char| c.is_ascii_whitespace()) {
        return Err(ModelError::InvalidLiteral(source.to_owned()));
    }
    Ok((number, rest))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finite_values_units_and_rounding_are_separate() {
        assert_eq!(
            Length::millimetres(1.234_567_89).unwrap().as_millimetres(),
            1.234_567_89
        );
        assert_eq!(
            Length::millimetres(-0.0)
                .unwrap()
                .as_millimetres()
                .to_bits(),
            0.0f64.to_bits()
        );
        assert_eq!(
            Quantity::parse_literal(" 007.5 mm ", Unit::Millimetre)
                .unwrap()
                .value_in(Unit::Millimetre)
                .unwrap(),
            7.5
        );
        assert_eq!(
            Quantity::parse_literal("180 deg", Unit::Degree)
                .unwrap()
                .value_in(Unit::Degree)
                .unwrap(),
            180.0
        );
        assert!((Angle::degrees(180.0).unwrap().as_radians() - PI).abs() < 1e-14);
        assert!(Angle::radians(f64::MAX).unwrap().as_degrees().is_err());
        assert!(Quantity::parse_literal("0 mm", Unit::Degree).is_err());
        assert!(Quantity::parse_literal("2 in", Unit::Millimetre).is_err());
        assert!(Quantity::parse_literal("1e-999", Unit::Millimetre).is_err());
        for bad in ["", ".", "1e", "1 mm mm", "NaN", "inf", "1,2", "1 x"] {
            assert!(
                Quantity::parse_literal(bad, Unit::Millimetre).is_err(),
                "{bad}"
            );
        }
    }
}

#[cfg(test)]
mod literal_contract_tests {
    use super::*;
    #[test]
    fn boundary_grammar_errors_and_unit_compatibility() {
        for (text, value) in [
            ("5", 5.0),
            (".5", 0.5),
            ("5.", 5.0),
            ("007.5", 7.5),
            ("+1e2 mm", 100.0),
            ("-2E-1 mm", -0.2),
            ("0e10", 0.0),
            ("-0e-999", 0.0),
        ] {
            assert_eq!(
                Quantity::parse_literal(text, Unit::Millimetre)
                    .unwrap()
                    .value_in(Unit::Millimetre)
                    .unwrap(),
                value
            );
        }
        for source in ["", ".", "1e", "1e+", "1 2", "1 mm mm", "1+2", "1,2", "--1"] {
            assert!(
                matches!(
                    Quantity::parse_literal(source, Unit::Millimetre),
                    Err(ModelError::InvalidLiteral(_))
                ),
                "{source}"
            );
        }
        for source in ["1 cm", "1 rad", "1 inch"] {
            assert!(matches!(
                Quantity::parse_literal(source, Unit::Millimetre),
                Err(ModelError::UnsupportedUnit(_))
            ));
        }
        for source in ["NaN", "inf", "-inf"] {
            assert_eq!(
                Quantity::parse_literal(source, Unit::Millimetre),
                Err(ModelError::NonFiniteQuantity)
            );
        }
        assert_eq!(
            Quantity::parse_literal("1e999", Unit::Millimetre),
            Err(ModelError::QuantityOverflow)
        );
        assert_eq!(
            Quantity::parse_literal("1e-999", Unit::Millimetre),
            Err(ModelError::QuantityUnderflow)
        );
        assert_eq!(
            Quantity::parse_literal("0 deg", Unit::Millimetre),
            Err(ModelError::DimensionMismatch)
        );
        assert_eq!(
            Quantity::parse_literal("0 mm", Unit::Unitless),
            Err(ModelError::DimensionMismatch)
        );
        assert!(Length::millimetres(f64::INFINITY).is_err());
        assert!(Scalar::new(f64::NAN).is_err());
        assert!(Angle::radians(f64::INFINITY).is_err());
        assert_eq!(
            Angle::degrees(f64::from_bits(1)),
            Err(ModelError::QuantityUnderflow)
        );
        assert_eq!(
            Angle::radians(f64::MAX).unwrap().as_degrees(),
            Err(ModelError::QuantityOverflow)
        );
        assert_eq!(
            Scalar::new(-0.0).unwrap().value().to_bits(),
            0.0f64.to_bits()
        );
        assert_eq!(
            Quantity::parse_literal("1e-320", Unit::Millimetre)
                .unwrap()
                .value_in(Unit::Millimetre)
                .unwrap(),
            1e-320
        );
    }
}
