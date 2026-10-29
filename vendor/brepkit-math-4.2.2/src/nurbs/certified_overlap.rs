// SPDX-License-Identifier: AGPL-3.0-only
// M5.0 experimental correction, Copyright (c) 2026 Martin Harambat.
//! Exact predicates for bounded integer-coordinate planar affine spans.
//! This deliberately does not certify arbitrary rational curves.

/// Exact rational curve parameter. Denominator is strictly positive.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Parameter {
    /// Exact signed numerator.
    pub numerator: i128,
    /// Exact positive denominator.
    pub denominator: i128,
}
impl Parameter {
    fn new(n: i128, d: i128) -> Self {
        if d < 0 {
            Self {
                numerator: -n,
                denominator: -d,
            }
        } else {
            Self {
                numerator: n,
                denominator: d,
            }
        }
    }
    fn compare(self, other: Self) -> std::cmp::Ordering {
        (self.numerator * other.denominator).cmp(&(other.numerator * self.denominator))
    }
    fn in_unit(self) -> bool {
        self.numerator >= 0 && self.numerator <= self.denominator
    }
    /// Encloses the exact rational parameter, including conversion rounding.
    #[must_use]
    pub fn enclosure(self) -> [f64; 2] {
        let n = self.numerator as f64;
        let d = self.denominator as f64;
        // One outward rounding for each integer conversion and division.
        let lo = n.next_down();
        let hi = n.next_up();
        let dlo = d.next_down();
        let dhi = d.next_up();
        if lo >= 0. {
            [(lo / dhi).next_down(), (hi / dlo).next_up()]
        } else if hi <= 0. {
            [(lo / dlo).next_down(), (hi / dhi).next_up()]
        } else {
            [(lo / dlo).next_down(), (hi / dlo).next_up()]
        }
    }
}
/// Original parameter on A and B at one point.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Correspondence {
    /// Parameter on the original first span.
    pub a: Parameter,
    /// Parameter on the original second span.
    pub b: Parameter,
}
/// Complete classification only for the explicitly admitted affine domain.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// Exact spans do not meet.
    Disjoint,
    /// One exact isolated contact, including endpoints.
    Point(Correspondence),
    /// Exact shared interval, ordered in first-span direction.
    Overlap([Correspondence; 2]),
    /// Input lies outside the proved arithmetic/geometry domain.
    Unresolved,
    /// Cancelled before the bounded operation.
    Cancelled,
    /// Insufficient admitted work units.
    BudgetExceeded,
}
fn integer(p: [f64; 2]) -> Option<[i128; 2]> {
    if p.iter()
        .any(|v| !v.is_finite() || v.fract() != 0. || v.abs() > (1_u64 << 40) as f64)
    {
        return None;
    }
    Some([p[0] as i128, p[1] as i128])
}
fn sub(a: [i128; 2], b: [i128; 2]) -> [i128; 2] {
    [a[0] - b[0], a[1] - b[1]]
}
fn cross(a: [i128; 2], b: [i128; 2]) -> i128 {
    a[0] * b[1] - a[1] * b[0]
}
/// Classifies finite segments using exact i128 determinants and rational maps.
/// Integer inputs are restricted to +/-2^40: every intermediate fits i128.
/// Fractions compared below have <=42-bit affine denominators. General point
/// determinants have <=84 bits and are only compared with zero/their denominator.
/// `work_units` admits one constant-size operation; there is no recursive call.
#[must_use]
pub fn classify(a: [[f64; 2]; 2], b: [[f64; 2]; 2], work_units: usize, cancelled: bool) -> Outcome {
    if cancelled {
        return Outcome::Cancelled;
    }
    if work_units == 0 {
        return Outcome::BudgetExceeded;
    }
    let (Some(a0), Some(a1), Some(b0), Some(b1)) =
        (integer(a[0]), integer(a[1]), integer(b[0]), integer(b[1]))
    else {
        return Outcome::Unresolved;
    };
    let da = sub(a1, a0);
    let db = sub(b1, b0);
    let w = sub(b0, a0);
    if da == [0, 0] || db == [0, 0] {
        return Outcome::Unresolved;
    }
    let det = cross(da, db);
    if det != 0 {
        let point = Correspondence {
            a: Parameter::new(cross(w, db), det),
            b: Parameter::new(cross(w, da), det),
        };
        return if point.a.in_unit() && point.b.in_unit() {
            Outcome::Point(point)
        } else {
            Outcome::Disjoint
        };
    }
    if cross(w, da) != 0 {
        return Outcome::Disjoint;
    }
    let axis = usize::from(da[0] == 0);
    let t0 = Parameter::new(w[axis], da[axis]);
    let t1 = Parameter::new(sub(b1, a0)[axis], da[axis]);
    let (mut lo, mut hi) = if t0.compare(t1).is_le() {
        (t0, t1)
    } else {
        (t1, t0)
    };
    let zero = Parameter::new(0, 1);
    let one = Parameter::new(1, 1);
    if lo.compare(zero).is_lt() {
        lo = zero;
    }
    if hi.compare(one).is_gt() {
        hi = one;
    }
    if lo.compare(hi).is_gt() {
        return Outcome::Disjoint;
    }
    let map = |t: Parameter| Correspondence {
        a: t,
        b: Parameter::new(
            (a0[axis] - b0[axis]) * t.denominator + da[axis] * t.numerator,
            db[axis] * t.denominator,
        ),
    };
    if lo.compare(hi).is_eq() {
        Outcome::Point(map(lo))
    } else {
        Outcome::Overlap([map(lo), map(hi)])
    }
}
