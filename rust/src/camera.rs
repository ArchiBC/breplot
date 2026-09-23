//! Shared orthographic camera for solids and standalone 2D/3D curves.

use crate::hlr::Point;
use brepkit_math::vec::{Point3, Vec3};

pub(crate) struct Camera {
    pub x: Vec3,
    pub y: Vec3,
    pub toward_scene: Vec3,
}

impl Camera {
    /// CeTZ draw.ortho angles in degrees (Rx * Ry * Rz).
    pub fn from_ortho(view: [f64; 3]) -> Result<Self, String> {
        if !view.iter().all(|v| v.is_finite()) {
            return Err("view angles must be finite".into());
        }
        let [x, y, z] = view.map(f64::to_radians);
        let (sx, cx) = x.sin_cos();
        let (sy, cy) = y.sin_cos();
        let (sz, cz) = z.sin_cos();
        let up = [cx * sz - sx * sy * cz, cx * cz + sx * sy * sz, -sx * cy];
        let direction = [-sx * sz - cx * sy * cz, -sx * cz + cx * sy * sz, -cx * cy];
        Self::new(direction, up)
    }

    pub fn new(direction: [f64; 3], up: [f64; 3]) -> Result<Self, String> {
        if !direction.iter().chain(up.iter()).all(|v| v.is_finite()) {
            return Err("direction and up must be finite".into());
        }
        let direction = Vec3::new(direction[0], direction[1], direction[2]);
        let up = Vec3::new(up[0], up[1], up[2]);
        let toward_scene = direction
            .normalize()
            .map_err(|_| "direction must be nonzero")?;
        let x = toward_scene
            .cross(up)
            .normalize()
            .map_err(|_| "up must not be parallel to direction")?;
        let y = x.cross(toward_scene);
        Ok(Self { x, y, toward_scene })
    }

    pub fn project(&self, p: Point3) -> Point {
        let v = Vec3::new(p.x(), p.y(), p.z());
        Point {
            x: self.x.dot(v),
            y: -self.y.dot(v),
            depth: -self.toward_scene.dot(v),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ortho_axes_match_cetz_rotation() {
        let camera = Camera::from_ortho([0.0, 90.0, 0.0]).unwrap();
        let x = camera.project(Point3::new(1.0, 0.0, 0.0));
        let z = camera.project(Point3::new(0.0, 0.0, 1.0));
        assert!(x.x.abs() < 1e-12);
        assert!((z.x + 1.0).abs() < 1e-12);
        let camera = Camera::from_ortho([0.0, 0.0, 90.0]).unwrap();
        let x = camera.project(Point3::new(1.0, 0.0, 0.0));
        assert!(x.x.abs() < 1e-12);
        assert!((x.y + 1.0).abs() < 1e-12);
    }
}
