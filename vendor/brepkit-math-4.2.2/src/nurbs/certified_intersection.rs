// SPDX-License-Identifier: AGPL-3.0-only
// M5.0 experimental correction, Copyright (c) 2026 Martin Harambat.
//! Feature-isolated, conservative entry point. Unsupported curves are unresolved.
use super::{
    certified_overlap::{Outcome, classify},
    curve::NurbsCurve,
};
/// Classifies affine XY spans with unit domains, equal positive weights and
/// integer-coordinate endpoints. All other curve families remain unresolved.
#[must_use]
pub fn intersect(a: &NurbsCurve, b: &NurbsCurve, work_units: usize, cancelled: bool) -> Outcome {
    if cancelled {
        return Outcome::Cancelled;
    }
    if work_units == 0 {
        return Outcome::BudgetExceeded;
    }
    let affine = |c: &NurbsCurve| {
        if c.degree() != 1
            || c.control_points().len() != 2
            || c.domain() != (0., 1.)
            || c.weights().len() != 2
            || c.weights()[0] != c.weights()[1]
            || !c.weights()[0].is_finite()
            || c.weights()[0] <= 0.
            || c.control_points().iter().any(|p| p.z() != 0.)
        {
            return None;
        }
        Some([
            [c.control_points()[0].x(), c.control_points()[0].y()],
            [c.control_points()[1].x(), c.control_points()[1].y()],
        ])
    };
    match (affine(a), affine(b)) {
        (Some(a), Some(b)) => classify(a, b, work_units, false),
        _ => Outcome::Unresolved,
    }
}

/// Enclosing parameters on the original quadratic and horizontal segment.
#[derive(Debug, Clone, Copy)]
pub struct QuadraticHit {
    /// Original clamped quadratic parameter enclosure.
    pub quadratic: [f64; 2],
    /// Original segment parameter enclosure.
    pub segment: [f64; 2],
    /// Algebraic root multiplicity (two only with an exact integer witness).
    pub multiplicity: u8,
}
/// Complete classification for the admitted quadratic/horizontal-line domain.
#[derive(Debug)]
pub enum QuadraticOutcome {
    /// All admitted roots, each with original parameter enclosures.
    Complete(Vec<QuadraticHit>),
    /// The operation cannot certify this input within its error/domain contract.
    Unresolved,
    /// Cancellation requested before this constant-size operation.
    Cancelled,
    /// Caller did not admit one bounded operation.
    BudgetExceeded,
}
#[derive(Clone, Copy)]
struct Interval(f64, f64);
impl Interval {
    fn exact(x: f64) -> Self {
        Self(x, x)
    }
    fn add(self, b: Self) -> Self {
        Self((self.0 + b.0).next_down(), (self.1 + b.1).next_up())
    }
    fn neg(self) -> Self {
        Self(-self.1, -self.0)
    }
    fn sub(self, b: Self) -> Self {
        self.add(b.neg())
    }
    fn mul(self, b: Self) -> Self {
        let p = [self.0 * b.0, self.0 * b.1, self.1 * b.0, self.1 * b.1];
        Self(
            p.iter().copied().fold(f64::INFINITY, f64::min).next_down(),
            p.iter()
                .copied()
                .fold(f64::NEG_INFINITY, f64::max)
                .next_up(),
        )
    }
    fn div(self, b: Self) -> Option<Self> {
        if b.0 <= 0. && b.1 >= 0. {
            return None;
        }
        Some(self.mul(Self((1. / b.1).next_down(), (1. / b.0).next_up())))
    }
    fn finite(self) -> bool {
        self.0.is_finite() && self.1.is_finite() && self.0 <= self.1
    }
}
// Sterbenz's lemma certifies the subtraction when same-sign operands are within
// a factor of two; elsewhere retain a conservative outward-rounded difference.
fn difference(a: f64, b: f64) -> Interval {
    if a == b
        || a == 0.
        || b == 0.
        || (a.is_sign_positive() == b.is_sign_positive()
            && a.abs() * 0.5 <= b.abs()
            && b.abs() <= a.abs() * 2.)
    {
        Interval::exact(a - b)
    } else {
        Interval::exact(a).sub(Interval::exact(b))
    }
}
/// Analytically isolates roots of a positive-weight clamped rational quadratic
/// against a horizontal XY segment. Outward intervals bound every arithmetic
/// operation; ambiguous discriminants, boundaries and overlaps are unresolved.
/// Exact tangencies are admitted only with an integer/unit-weight discriminant
/// witness. At most two roots/one bounded operation, no recursion or sampling.
#[must_use]
pub fn horizontal_quadratic(
    q: &NurbsCurve,
    line: &NurbsCurve,
    tolerance: f64,
    work_units: usize,
    cancelled: bool,
) -> QuadraticOutcome {
    use QuadraticOutcome::{Complete, Unresolved};
    if cancelled {
        return QuadraticOutcome::Cancelled;
    }
    if work_units == 0 {
        return QuadraticOutcome::BudgetExceeded;
    }
    if !tolerance.is_finite()
        || tolerance <= 0.
        || q.degree() != 2
        || q.control_points().len() != 3
        || q.domain() != (0., 1.)
        || q.knots() != [0., 0., 0., 1., 1., 1.]
        || line.degree() != 1
        || line.control_points().len() != 2
        || line.domain() != (0., 1.)
        || line.knots() != [0., 0., 1., 1.]
        || line.weights().len() != 2
        || line.weights()[0] != line.weights()[1]
        || line.weights()[0] <= 0.
        || !line.weights()[0].is_finite()
    {
        return Unresolved;
    }
    let lp = line.control_points();
    let points = q.control_points();
    let weights = q.weights();
    if weights.len() != 3
        || weights.iter().any(|w| !w.is_finite() || *w <= 0.)
        || points
            .iter()
            .chain(lp)
            .any(|p| p.z() != 0. || !p.x().is_finite() || !p.y().is_finite())
        || lp[0].y() != lp[1].y()
        || lp[0].x() == lp[1].x()
    {
        return Unresolved;
    }
    let y = lp[0].y();
    let zero = Interval::exact(0.);
    let one = Interval::exact(1.);
    let two = Interval::exact(2.);
    let four = Interval::exact(4.);
    let c: Vec<_> = points
        .iter()
        .zip(weights)
        .map(|(p, w)| difference(p.y(), y).mul(Interval::exact(*w)))
        .collect();
    let a = c[0].sub(two.mul(c[1])).add(c[2]);
    let b = two.mul(c[1].sub(c[0]));
    if !a.finite() || !b.finite() || !c[0].finite() || a.0 <= 0. && a.1 >= 0. {
        return Unresolved;
    }
    let disc = b.mul(b).sub(four.mul(a).mul(c[0]));
    if !disc.finite() {
        return Unresolved;
    }
    let exact_int = |v: f64| {
        if v.fract() == 0. && v.abs() <= ((1_u64 << 20) as f64) {
            Some(v as i128)
        } else {
            None
        }
    };
    let local_y: Vec<_> = points.iter().map(|p| difference(p.y(), y)).collect();
    let tangent = if weights.iter().all(|w| *w == 1.) && local_y.iter().all(|v| v.0 == v.1) {
        match (
            exact_int(local_y[0].0),
            exact_int(local_y[1].0),
            exact_int(local_y[2].0),
        ) {
            (Some(p0), Some(p1), Some(p2)) => {
                let aa = p0 - 2 * p1 + p2;
                let bb = 2 * (p1 - p0);
                aa != 0 && bb * bb - 4 * aa * p0 == 0
            }
            _ => false,
        }
    } else {
        false
    };
    let mut roots = Vec::with_capacity(2);
    if tangent {
        let Some(t) = b.neg().div(two.mul(a)) else {
            return Unresolved;
        };
        roots.push((t, 2));
    } else if disc.1 < 0. {
        return Complete(Vec::new());
    } else if disc.0 > 0. {
        let sqrt = Interval(disc.0.sqrt().next_down(), disc.1.sqrt().next_up());
        for sign in [-1., 1.] {
            let Some(t) = b.neg().add(Interval::exact(sign).mul(sqrt)).div(two.mul(a)) else {
                return Unresolved;
            };
            roots.push((t, 1));
        }
        roots.sort_by(|a, b| a.0.0.total_cmp(&b.0.0));
        if roots[0].0.1 >= roots[1].0.0 {
            return Unresolved;
        }
    } else {
        return Unresolved;
    }
    let length = difference(lp[1].x(), lp[0].x());
    let mut hits = Vec::with_capacity(2);
    for (t, multiplicity) in roots {
        if !t.finite() {
            return Unresolved;
        }
        if t.1 < 0. || t.0 > 1. {
            continue;
        }
        if t.0 < 0. || t.1 > 1. {
            return Unresolved;
        }
        let u = one.sub(t);
        let basis = [u.mul(u), two.mul(t).mul(u), t.mul(t)];
        let mut x = zero;
        let mut denominator = zero;
        for i in 0..3 {
            let w = basis[i].mul(Interval::exact(weights[i]));
            x = x.add(w.mul(difference(points[i].x(), lp[0].x())));
            denominator = denominator.add(w);
        }
        if denominator.0 <= 0. {
            return Unresolved;
        }
        let Some(x) = x.div(denominator) else {
            return Unresolved;
        };
        if !x.finite() || x.1 - x.0 > tolerance {
            return Unresolved;
        }
        let Some(s) = x.div(length) else {
            return Unresolved;
        };
        if !s.finite() {
            return Unresolved;
        }
        if s.1 < 0. || s.0 > 1. {
            continue;
        }
        if s.0 < 0. || s.1 > 1. {
            return Unresolved;
        }
        hits.push(QuadraticHit {
            quadratic: [t.0, t.1],
            segment: [s.0, s.1],
            multiplicity,
        });
    }
    Complete(hits)
}

/// A parameter box on each original curve; endpoints are outward enclosures.
#[derive(Debug, Clone, Copy)]
pub struct MappedPair {
    /// Original parameter on the first curve.
    pub a: [f64; 2],
    /// Original parameter on the second curve.
    pub b: [f64; 2],
}
/// Complete classification only for the admitted, explicitly checked families.
#[derive(Debug)]
pub enum MappedOutcome {
    /// No common point in the admitted domain.
    Disjoint,
    /// One isolated common point.
    Point(MappedPair),
    /// Exact common geometry between these two mapped endpoints.
    Overlap([MappedPair; 2]),
    /// Domain or arithmetic cannot be certified by this candidate.
    Unresolved,
    /// Caller cancelled before this bounded operation.
    Cancelled,
    /// Caller did not admit the required control-point work.
    BudgetExceeded,
}

fn mapped_parameter(domain: (f64, f64), unit: [f64; 2]) -> Option<[f64; 2]> {
    let width = difference(domain.1, domain.0);
    let mapped = Interval::exact(domain.0).add(width.mul(Interval(unit[0], unit[1])));
    mapped.finite().then_some([mapped.0, mapped.1])
}

/// Exact affine classification with original knot domains, plus exact
/// control-net identity/reversal of single-span positive-weight rational curves.
///
/// Curve identity compares all controls and weights, never proximity samples.
/// Clamped spans may use different finite non-unit knot domains. The rational
/// Bernstein basis is invariant under this affine reparameterization. No claim
/// is made for differently refined or geometrically equivalent control nets.
/// Affine determinants retain `certified_overlap::classify`'s integer bounds.
#[must_use]
pub fn intersect_mapped(
    a: &NurbsCurve,
    b: &NurbsCurve,
    work_units: usize,
    cancelled: bool,
) -> MappedOutcome {
    use MappedOutcome::{BudgetExceeded, Overlap, Unresolved};
    if cancelled {
        return MappedOutcome::Cancelled;
    }
    let cost = a
        .control_points()
        .len()
        .saturating_add(b.control_points().len());
    if work_units < cost {
        return BudgetExceeded;
    }
    let single_span = |c: &NurbsCurve| {
        let d = c.domain();
        let p = c.degree();
        p > 0
            && p <= 32
            && c.control_points().len() == p + 1
            && c.weights().len() == p + 1
            && c.knots().len() == 2 * (p + 1)
            && d.0.is_finite()
            && d.1.is_finite()
            && d.0 < d.1
            && c.knots()[..=p].iter().all(|&k| k == d.0)
            && c.knots()[p + 1..].iter().all(|&k| k == d.1)
            && c.weights().iter().all(|&w| w.is_finite() && w > 0.)
            && c.control_points()
                .iter()
                .all(|p| p.x().is_finite() && p.y().is_finite() && p.z() == 0.)
    };
    if !single_span(a) || !single_span(b) {
        return Unresolved;
    }
    let da = a.domain();
    let db = b.domain();
    if a.degree() == 1
        && b.degree() == 1
        && a.weights()[0] == a.weights()[1]
        && b.weights()[0] == b.weights()[1]
    {
        let endpoints = |c: &NurbsCurve| {
            [
                [c.control_points()[0].x(), c.control_points()[0].y()],
                [c.control_points()[1].x(), c.control_points()[1].y()],
            ]
        };
        let mapped = |c: super::certified_overlap::Correspondence| -> Option<MappedPair> {
            Some(MappedPair {
                a: mapped_parameter(da, c.a.enclosure())?,
                b: mapped_parameter(db, c.b.enclosure())?,
            })
        };
        return match classify(endpoints(a), endpoints(b), 1, false) {
            Outcome::Disjoint => MappedOutcome::Disjoint,
            Outcome::Point(c) => mapped(c).map_or(Unresolved, MappedOutcome::Point),
            Outcome::Overlap(c) => match (mapped(c[0]), mapped(c[1])) {
                (Some(first), Some(last)) => Overlap([first, last]),
                _ => Unresolved,
            },
            _ => Unresolved,
        };
    }
    if a.degree() != b.degree() {
        return Unresolved;
    }
    let identical = a
        .control_points()
        .iter()
        .zip(b.control_points())
        .all(|(p, q)| p == q)
        && a.weights() == b.weights();
    let reverse = a
        .control_points()
        .iter()
        .zip(b.control_points().iter().rev())
        .all(|(p, q)| p == q)
        && a.weights()
            .iter()
            .zip(b.weights().iter().rev())
            .all(|(w, v)| w == v);
    if !identical && !reverse {
        return Unresolved;
    }
    // Constant curves have no one-to-one parameter correspondence.
    if a.control_points()
        .iter()
        .all(|p| p == &a.control_points()[0])
    {
        return Unresolved;
    }
    let (b0, b1) = if identical {
        (db.0, db.1)
    } else {
        (db.1, db.0)
    };
    Overlap([
        MappedPair {
            a: [da.0, da.0],
            b: [b0, b0],
        },
        MappedPair {
            a: [da.1, da.1],
            b: [b1, b1],
        },
    ])
}
