// Newell - Parametric CAD Software
// Copyright (c) 2026 Martin Harambat
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//! Immutable camera data and physical scene rectangles consumed by the View.

/// Column-major matrix for WGPU's right-handed, zero-to-one depth convention.
pub type CameraMatrix = [[f32; 4]; 4];

/// Immutable camera matrices prepared by the Controller. Contains no navigation state.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CameraSnapshot {
    view: CameraMatrix,
    projection: CameraMatrix,
    view_projection: CameraMatrix,
    revision: u64,
}

impl CameraSnapshot {
    /// Validates the finite matrix payload at the View boundary.
    #[must_use]
    pub fn new(
        view: CameraMatrix,
        projection: CameraMatrix,
        view_projection: CameraMatrix,
        revision: u64,
    ) -> Option<Self> {
        view.iter()
            .chain(projection.iter())
            .chain(view_projection.iter())
            .flatten()
            .all(|value| value.is_finite())
            .then_some(Self {
                view,
                projection,
                view_projection,
                revision,
            })
    }

    /// Camera matrix in world-to-view convention.
    #[must_use]
    pub const fn view(self) -> CameraMatrix {
        self.view
    }
    /// Clip projection prepared by the Controller.
    #[must_use]
    pub const fn projection(self) -> CameraMatrix {
        self.projection
    }
    /// Matrix written to the existing scene uniform layout.
    #[must_use]
    pub const fn view_projection(self) -> CameraMatrix {
        self.view_projection
    }
    /// Controller revision, used to identify immutable payloads.
    #[must_use]
    pub const fn revision(self) -> u64 {
        self.revision
    }
}

impl Default for CameraSnapshot {
    fn default() -> Self {
        let identity = [
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ];
        Self {
            view: identity,
            projection: identity,
            view_projection: identity,
            revision: 0,
        }
    }
}

/// A physical-pixel rectangle occupied by the 3D viewport.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SceneViewport {
    /// Left pixel coordinate.
    pub x: u32,
    /// Top pixel coordinate.
    pub y: u32,
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
}

impl SceneViewport {
    /// Makes a physical viewport rectangle.
    #[must_use]
    pub const fn new(x: u32, y: u32, width: u32, height: u32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    /// Whether a finite physical-pixel point belongs to this rectangle.
    #[must_use]
    pub fn contains(self, x: f64, y: f64) -> bool {
        x >= f64::from(self.x)
            && y >= f64::from(self.y)
            && x < f64::from(self.x.saturating_add(self.width))
            && y < f64::from(self.y.saturating_add(self.height))
            && self.width > 0
            && self.height > 0
    }
}

/// Rectangular scene regions remaining after opaque UI footprints are subtracted.
#[must_use]
pub fn exposed_regions(
    viewport: SceneViewport,
    occlusions: &[SceneViewport],
) -> Vec<SceneViewport> {
    occlusions
        .iter()
        .copied()
        .fold(vec![viewport], |regions, occlusion| {
            regions
                .into_iter()
                .flat_map(|region| subtract(region, occlusion))
                .collect()
        })
}

fn subtract(viewport: SceneViewport, occlusion: SceneViewport) -> Vec<SceneViewport> {
    let Some(occlusion) = intersection(viewport, occlusion) else {
        return vec![viewport];
    };
    let right = viewport.x + viewport.width;
    let bottom = viewport.y + viewport.height;
    let occ_right = occlusion.x + occlusion.width;
    let occ_bottom = occlusion.y + occlusion.height;
    let mut regions = Vec::with_capacity(4);
    let mut push = |x, y, width, height| {
        if width > 0 && height > 0 {
            regions.push(SceneViewport::new(x, y, width, height));
        }
    };
    push(
        viewport.x,
        viewport.y,
        viewport.width,
        occlusion.y - viewport.y,
    );
    push(viewport.x, occ_bottom, viewport.width, bottom - occ_bottom);
    push(
        viewport.x,
        occlusion.y,
        occlusion.x - viewport.x,
        occlusion.height,
    );
    push(occ_right, occlusion.y, right - occ_right, occlusion.height);
    regions
}

fn intersection(a: SceneViewport, b: SceneViewport) -> Option<SceneViewport> {
    let x = a.x.max(b.x);
    let y = a.y.max(b.y);
    let right = a.x.saturating_add(a.width).min(b.x.saturating_add(b.width));
    let bottom =
        a.y.saturating_add(a.height)
            .min(b.y.saturating_add(b.height));
    (right > x && bottom > y).then(|| SceneViewport::new(x, y, right - x, bottom - y))
}

#[cfg(test)]
mod tests {
    use super::CameraSnapshot;
    #[test]
    fn snapshot_rejects_nonfinite_payloads() {
        let identity = CameraSnapshot::default();
        let mut invalid = identity.view();
        invalid[1][2] = f32::NAN;
        assert!(
            CameraSnapshot::new(
                invalid,
                identity.projection(),
                identity.view_projection(),
                1
            )
            .is_none()
        );
        invalid[1][2] = f32::INFINITY;
        assert!(
            CameraSnapshot::new(identity.view(), invalid, identity.view_projection(), 1).is_none()
        );
        assert!(CameraSnapshot::new(identity.view(), identity.projection(), invalid, 1).is_none());
    }
}
