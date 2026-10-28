// Newell - Parametric CAD Software
// Copyright (c) 2026 Martin Harambat
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//! Primary-button navigator capture, independent of scene and shell activation.
use newell_ui::navigation_widgets::{NavGeometry, NavPart, ViewDirection};
#[derive(Debug, Clone, Default)]
pub enum NavInteraction {
    #[default]
    Idle,
    Pressed {
        part: NavPart,
        origin: (f64, f64),
        slop: f64,
        layout: u64,
        camera: u64,
    },
    CancelledUntilPrimaryRelease,
}
impl NavInteraction {
    pub fn active(&self) -> bool {
        !matches!(self, Self::Idle)
    }
    pub fn cancel(&mut self) {
        if self.active() {
            *self = Self::CancelledUntilPrimaryRelease;
        }
    }
    pub fn press(&mut self, g: &NavGeometry, p: (f64, f64), scale: f64, camera: u64) {
        *self = if scale.is_finite() && scale > 0. {
            g.hit(p)
                .map_or(Self::CancelledUntilPrimaryRelease, |part| Self::Pressed {
                    part,
                    origin: p,
                    slop: 4. * scale,
                    layout: g.layout.generation,
                    camera,
                })
        } else {
            Self::CancelledUntilPrimaryRelease
        };
    }
    pub fn advance(&mut self, p: (f64, f64)) {
        if let Self::Pressed { origin, slop, .. } = self
            && (!p.0.is_finite()
                || !p.1.is_finite()
                || (p.0 - origin.0).hypot(p.1 - origin.1) > *slop)
        {
            *self = Self::CancelledUntilPrimaryRelease;
        }
    }
    pub fn validate(&mut self, g: Option<&NavGeometry>, camera: u64) {
        if let Self::Pressed {
            layout,
            camera: old,
            ..
        } = self
            && (*old != camera || g.is_none_or(|g| g.layout.generation != *layout))
        {
            self.cancel();
        }
    }
    pub fn release(
        &mut self,
        g: Option<&NavGeometry>,
        p: (f64, f64),
        camera: u64,
    ) -> Option<ViewDirection> {
        self.advance(p);
        self.validate(g, camera);
        let old = std::mem::take(self);
        if let Self::Pressed { part, .. } = old
            && g.and_then(|g| g.hit(p)) == Some(part)
        {
            Some(match part {
                NavPart::Cube(d) => d,
                NavPart::Axis(cap) => cap.direction(),
            })
        } else {
            None
        }
    }
}
/// Resolves material hover from the Controller's semantic hit, after input admission.
/// The renderer receives the result; it never repeats this picking decision.
pub fn axis_hover(
    geometry: Option<&NavGeometry>,
    pointer: Option<(f64, f64)>,
    eligible: bool,
    captured: bool,
) -> Option<newell_ui::navigation_widgets::AxisCap> {
    if !eligible || captured {
        return None;
    }
    match geometry?.hit(pointer?)? {
        NavPart::Axis(cap) => Some(cap),
        NavPart::Cube(_) => None,
    }
}

/// Resolves the single cube patch whose rest/hover material the renderer may expose.
pub fn cube_hover(
    geometry: Option<&NavGeometry>,
    pointer: Option<(f64, f64)>,
    eligible: bool,
    captured: bool,
) -> Option<ViewDirection> {
    if !eligible || captured {
        return None;
    }
    match geometry?.hit(pointer?)? {
        NavPart::Cube(direction) => Some(direction),
        NavPart::Axis(_) => None,
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use newell_ui::{GridMetrics, navigation_widgets::*};
    fn geometry() -> NavGeometry {
        let m = GridMetrics::from_k(2).unwrap();
        let rect = PixelRect {
            x: 0.,
            y: 0.,
            width: 400.,
            height: 400.,
        };
        NavGeometry::new(
            layout_navigation(rect, rect, m, true, NavigatorKind::Cube, None),
            NavBasis {
                right: [1., 0., 0.],
                up: [0., 1., 0.],
                forward: [0., 0., -1.],
            },
            m,
        )
        .unwrap()
    }
    #[test]
    fn primary_certificate_is_single_use_and_stale_capture_is_consumed() {
        let g = geometry();
        let t = g.layout.tile.unwrap();
        let p = (t.x + 60., t.y + 60.);
        let mut i = NavInteraction::default();
        i.press(&g, p, 1., 2);
        assert!(i.release(Some(&g), p, 2).is_some());
        assert!(!i.active());
        assert!(i.release(Some(&g), p, 2).is_none());
        i.press(&g, p, 1., 2);
        i.validate(Some(&g), 3);
        assert!(i.active());
        assert!(i.release(Some(&g), p, 3).is_none());
    }
    #[test]
    fn exceeding_slop_and_returning_never_reactivates() {
        let g = geometry();
        let t = g.layout.tile.unwrap();
        let p = (t.x + 60., t.y + 60.);
        let mut i = NavInteraction::default();
        i.press(&g, p, 2., 0);
        i.advance((p.0 + 8.01, p.1));
        i.advance(p);
        assert!(i.release(Some(&g), p, 0).is_none());
        i.press(&g, p, 2., 0);
        i.advance((p.0 + 8., p.1));
        assert!(i.release(Some(&g), p, 0).is_some());
    }
}

#[cfg(test)]
mod hover_tests {
    use super::*;
    use newell_ui::{GridMetrics, navigation_widgets::*};
    fn geometry(kind: NavigatorKind, visible: bool) -> NavGeometry {
        let m = GridMetrics::from_k(2).unwrap();
        let r = PixelRect {
            x: 0.,
            y: 0.,
            width: 400.,
            height: 400.,
        };
        NavGeometry::new(
            layout_navigation(r, r, m, visible, kind, None),
            NavBasis {
                right: [1., 0., 0.],
                up: [0., 1., 0.],
                forward: [0., 0., -1.],
            },
            m,
        )
        .unwrap()
    }
    #[test]
    fn semantic_hover_clears_for_exit_hidden_occlusion_and_capture() {
        let g = geometry(NavigatorKind::Gizmo, true);
        let t = g.layout.tile.unwrap();
        let p = (t.x + 79., t.y + 48.);
        let cap = AxisCap {
            axis: SignedAxis::X,
            positive: true,
        };
        assert_eq!(axis_hover(Some(&g), Some(p), true, false), Some(cap));
        assert_eq!(axis_hover(Some(&g), None, true, false), None);
        assert_eq!(axis_hover(None, Some(p), true, false), None);
        assert_eq!(axis_hover(Some(&g), Some(p), false, false), None);
        assert_eq!(axis_hover(Some(&g), Some(p), true, true), None);
        assert_eq!(
            axis_hover(
                Some(&geometry(NavigatorKind::Gizmo, false)),
                Some(p),
                true,
                false
            ),
            None
        );
        assert_eq!(
            axis_hover(
                Some(&geometry(NavigatorKind::Cube, true)),
                Some(p),
                true,
                false
            ),
            None
        );
        assert_eq!(axis_hover(Some(&g), Some((0., 0.)), true, false), None);
    }

    #[test]
    fn cube_hover_is_one_semantic_patch_and_clears_for_ineligible_pointer_or_capture() {
        let g = geometry(NavigatorKind::Cube, true);
        let tile = g.layout.tile.unwrap();
        // This basis views +Z. +X is edge-on and its unit normal lies outside
        // the beveled cube; hover must target a visible supporting polygon.
        let direction = ViewDirection::new([0, 0, 1]).unwrap();
        let point = g.project(direction.normal());
        let point = (tile.x + point[0], tile.y + point[1]);
        assert_eq!(
            cube_hover(Some(&g), Some(point), true, false),
            Some(direction)
        );
        assert_eq!(cube_hover(Some(&g), None, true, false), None);
        assert_eq!(cube_hover(Some(&g), Some(point), false, false), None);
        assert_eq!(cube_hover(Some(&g), Some(point), true, true), None);
        assert_eq!(cube_hover(None, Some(point), true, false), None);
        assert_eq!(
            cube_hover(
                Some(&geometry(NavigatorKind::Cube, false)),
                Some(point),
                true,
                false
            ),
            None
        );
    }
}
