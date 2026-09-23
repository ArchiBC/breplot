//! Display mesh facets and legacy vector-flat colors.
//! The raster mode also uses their smooth normals for per-pixel lighting.

use brepkit_math::vec::{Point3, Vec3};

use crate::hlr::Point;

pub struct Facet {
    pub points: [Point; 3],
    pub world: [Point3; 3],
    pub normals: [Vec3; 3],
    pub depth: f64,
    pub color: [u8; 3],
}

impl Facet {
    pub fn new(
        points: [Point; 3],
        world: [Point3; 3],
        normals: [Vec3; 3],
        light: Vec3,
        fill: Vec3,
        viewer: Vec3,
        specular: f64,
    ) -> Option<Self> {
        let geometric = (world[1] - world[0])
            .cross(world[2] - world[0])
            .normalize()
            .ok()?;
        let average = normals[0] + normals[1] + normals[2];
        let mut normal = average.normalize().unwrap_or(geometric);
        if normal.dot(geometric) < 0.0 {
            normal = -normal;
        }
        // Closed solids are shaded on the side facing the viewer. This also
        // avoids dark interior facets leaking through small mesh gaps.
        if normal.dot(viewer) < 0.0 {
            normal = -normal;
        }
        let diffuse = normal.dot(light).max(0.0);
        let secondary = normal.dot(fill).max(0.0);
        let half = (light + viewer).normalize().ok()?;
        let highlight = normal.dot(half).max(0.0).powf(18.0) * specular;
        let intensity = 0.58 + 0.34 * diffuse + 0.09 * secondary;
        let base = [194.0, 210.0, 224.0];
        let color = base.map(|v| {
            (v * intensity + 255.0 * highlight)
                .round()
                .clamp(0.0, 255.0) as u8
        });
        let depth = points.iter().map(|p| p.depth).sum::<f64>() / 3.0;
        let normals = normals.map(|n| {
            let mut n = n.normalize().unwrap_or(geometric);
            if n.dot(geometric) < 0.0 {
                n = -n;
            }
            if n.dot(viewer) < 0.0 {
                n = -n;
            }
            n
        });
        Some(Self {
            points,
            world,
            normals,
            depth,
            color,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn specular_light_brightens_aligned_facet() {
        let points = [
            Point {
                x: 0.0,
                y: 0.0,
                depth: 0.0,
            },
            Point {
                x: 1.0,
                y: 0.0,
                depth: 0.0,
            },
            Point {
                x: 0.0,
                y: 1.0,
                depth: 0.0,
            },
        ];
        let world = [
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(0.0, 1.0, 0.0),
        ];
        let normals = [Vec3::new(0.0, 0.0, 1.0); 3];
        let viewer = Vec3::new(0.0, 0.0, 1.0);
        let matte = Facet::new(points, world, normals, viewer, viewer, viewer, 0.0).unwrap();
        let glossy = Facet::new(points, world, normals, viewer, viewer, viewer, 0.3).unwrap();
        assert!(glossy.color.iter().zip(matte.color).all(|(a, b)| *a >= b));
        assert!(glossy.color.iter().zip(matte.color).any(|(a, b)| *a > b));
    }
}
