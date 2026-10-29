// SPDX-License-Identifier: AGPL-3.0-only
// M5.0 experimental correction, Copyright (c) 2026 Martin Harambat.
//! Analytic full-circle offset candidate; no polyline or general offset claim.
use brepkit_math::curves::Circle3D;
/// Bounded result, with the radius interval enclosing exact real r+distance.
#[derive(Debug)]
pub struct CircleOffset {
    /// Analytic output in the unchanged input frame.
    pub circle: Circle3D,
    /// Outward enclosure of exact real radius plus distance.
    pub radius_enclosure: [f64; 2],
}
/// Errors are explicit; no degenerate radius becomes accepted geometry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OffsetFailure {
    /// Cancellation requested before work.
    Cancelled,
    /// No operation admitted by the caller.
    BudgetExceeded,
    /// Nonfinite input or a collapsed/reversed radius.
    Invalid,
    /// Rounding cannot certify a positive radius.
    Unresolved,
}
/// Preserves the circle's frame. A nonzero offset with rounding-ambiguous sign
/// is unresolved; arbitrary curves, joins and self-intersections are not covered.
pub fn circle(
    circle: &Circle3D,
    distance: f64,
    work_units: usize,
    cancelled: bool,
) -> Result<CircleOffset, OffsetFailure> {
    if cancelled {
        return Err(OffsetFailure::Cancelled);
    }
    if work_units == 0 {
        return Err(OffsetFailure::BudgetExceeded);
    }
    if !distance.is_finite() {
        return Err(OffsetFailure::Invalid);
    }
    let axes = [circle.normal(), circle.u_axis(), circle.v_axis()];
    let center = circle.center();
    if [center.x(), center.y(), center.z()]
        .iter()
        .any(|v| !v.is_finite())
        || axes
            .iter()
            .any(|v| !v.length().is_finite() || (v.length() - 1.).abs() > 1e-12)
        || axes[0].dot(axes[1]).abs() > 1e-12
        || axes[0].dot(axes[2]).abs() > 1e-12
        || axes[1].dot(axes[2]).abs() > 1e-12
    {
        return Err(OffsetFailure::Invalid);
    }
    let radius = circle.radius() + distance;
    if !radius.is_finite() || radius <= 0. {
        return Err(OffsetFailure::Invalid);
    }
    let enclosure = [radius.next_down(), radius.next_up()];
    if enclosure[0] <= 0. {
        return Err(OffsetFailure::Unresolved);
    }
    let result = Circle3D::with_axes(
        circle.center(),
        circle.normal(),
        radius,
        circle.u_axis(),
        circle.v_axis(),
    );
    let circle = result.map_err(|_| OffsetFailure::Invalid)?;
    Ok(CircleOffset {
        circle,
        radius_enclosure: enclosure,
    })
}
