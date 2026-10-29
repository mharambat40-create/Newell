// Newell - Parametric CAD Software
// Copyright (c) 2026 Martin Harambat
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//! Analytic independent fixtures for the upstream correction dossier.
#[cfg(test)]
mod tests {
    use brepkit_math::vec::{Point3, Vec3};
    use brepkit_topology::{
        Topology,
        builder::make_circle_edge,
        face::{Face, FaceSurface},
        wire::{OrientedEdge, Wire},
    };
    type Result = std::result::Result<(), Box<dyn std::error::Error>>;
    // Smooth full-circle offsets have no join corner: all three policies must
    // yield this same circle. A collapsed radius is an explicit invalid result.
    #[test]
    fn analytic_circle_offset_remains_exact_through_native_extrusion() -> Result {
        for shift in [0., 1e9] {
            for distance in [-0.25, 0.25] {
                let radius = 2. + distance;
                let mut topology = Topology::new();
                let edge = make_circle_edge(
                    &mut topology,
                    Point3::new(shift, shift, 0.),
                    Vec3::new(0., 0., 1.),
                    radius,
                    1e-7,
                )?;
                let wire = topology.add_wire(Wire::new(vec![OrientedEdge::new(edge, true)], true)?);
                let face = topology.add_face(Face::new(
                    wire,
                    vec![],
                    FaceSurface::Plane {
                        normal: Vec3::new(0., 0., 1.),
                        d: 0.,
                    },
                ));
                let begin = std::time::Instant::now();
                let solid = brepkit_operations::extrude::extrude(
                    &mut topology,
                    face,
                    Vec3::new(0., 0., 1.),
                    2.,
                )?;
                assert!(brepkit_operations::validate::validate_solid(&topology, solid)?.is_valid());
                let volume = brepkit_operations::measure::solid_volume(&topology, solid, 1e-5)?;
                let expected = std::f64::consts::PI * radius * radius * 2.;
                assert!((volume - expected).abs() < 1e-5);
                println!(
                    "R3_ANALYTIC_OFFSET,shift={shift:e},distance={distance},radius={radius},expected_volume={expected},volume={volume},elapsed_us={:.3},scope=smooth_circle_only",
                    begin.elapsed().as_secs_f64() * 1e6
                );
            }
        }
        Ok(())
    }
    #[test]
    fn affine_partial_and_reversed_overlap_have_independent_parameter_maps() {
        // A(u)=-2+4u. B(v)=3v or 1-2v. Solve exact affine maps,
        // not a sampled proximity test. Both endpoint pairs must coincide.
        for shift in [0., 1e9] {
            for (origin, direction, lo, hi) in [(0., 3., 0.5, 1.), (1., -2., 0.25, 0.75)] {
                let map = |u: f64| (-2. + 4. * u - origin) / direction;
                let parameters = [(lo, map(lo)), (hi, map(hi))];
                for (u, v) in parameters {
                    assert!((0. ..=1.).contains(&u) && (0. ..=1.).contains(&v));
                    assert_eq!(shift - 2. + 4. * u, shift + origin + direction * v);
                }
                println!(
                    "R3_OVERLAP_ORACLE,shift={shift:e},direction={direction},endpoints={parameters:?}"
                );
            }
        }
    }
    #[test]
    fn printable_ascii_raw_outline_precision_and_rigid_transforms() -> Result {
        use ab_glyph::{Font, FontRef, OutlineCurve};
        let font =
            FontRef::try_from_slice(include_bytes!("../../../../assets/FiraCode-Regular.ttf"))?;
        let mut glyphs = 0;
        let mut coordinates = 0;
        let mut max_translation_error = 0_f64;
        for c in ' '..='~' {
            if c == ' ' {
                continue;
            }
            let outline = font
                .outline(font.glyph_id(c))
                .ok_or("missing printable outline")?;
            glyphs += 1;
            for curve in outline.curves {
                let points = match curve {
                    OutlineCurve::Line(a, b) => vec![a, b],
                    OutlineCurve::Quad(a, b, c) => vec![a, b, c],
                    OutlineCurve::Cubic(a, b, c, d) => vec![a, b, c, d],
                };
                for point in points {
                    for coordinate in [point.x, point.y] {
                        assert_eq!((coordinate * 2.).fract(), 0.);
                        coordinates += 1;
                    }
                    let x = f64::from(point.x) * 10. / 1950.;
                    let y = f64::from(point.y) * 10. / 1950.;
                    let rotated = [
                        (x - y) * std::f64::consts::FRAC_1_SQRT_2,
                        (x + y) * std::f64::consts::FRAC_1_SQRT_2,
                    ];
                    for v in rotated {
                        max_translation_error =
                            max_translation_error.max(((v + 1e9) - 1e9 - v).abs());
                    }
                }
            }
        }
        assert_eq!(glyphs, 94);
        assert!(max_translation_error <= f64::from_bits((1e9_f64).to_bits() + 1) - 1e9);
        println!(
            "R3_ASCII,ink_glyphs={glyphs},space=valid_no_ink,coordinates={coordinates},max_rotated_translation_error_mm={max_translation_error:e},full_font_arrangement=Unproven"
        );
        Ok(())
    }
}
