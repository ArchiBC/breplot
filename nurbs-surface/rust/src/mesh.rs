use crate::evaluate::{cross, norm, sub};
use crate::{DataError, NurbsSurface};

// doc:mesh:start
#[derive(Clone, Copy, Debug)]
pub struct MeshVertex {
    pub uv: [f64; 2],
    pub point: [f64; 3],
    pub normal: Option<[f64; 3]>,
    pub spans: [usize; 2],
}
#[derive(Clone, Debug)]
pub struct Mesh {
    pub vertices: Vec<MeshVertex>,
    pub triangles: Vec<[usize; 3]>,
    pub subdivisions: usize,
    pub sampled_error: f64,
    pub degenerate_triangles: usize,
}
#[derive(Clone, Copy, Debug)]
pub struct MeshOptions {
    pub tolerance: f64,
    pub initial_subdivisions: usize,
    pub max_subdivisions: usize,
    pub max_vertices: usize,
}
// doc:mesh:end
impl Default for MeshOptions {
    fn default() -> Self {
        Self {
            tolerance: 0.01,
            initial_subdivisions: 2,
            max_subdivisions: 128,
            max_vertices: 1_000_000,
        }
    }
}

fn parameter(a: f64, b: f64, i: usize, n: usize) -> f64 {
    if i == 0 {
        a
    } else if i == n {
        b
    } else {
        a + (b - a) * (i as f64 / n as f64)
    }
}

impl NurbsSurface {
    /// Uniform subdivision of EACH nonzero knot rectangle. Vertices are shared
    /// inside a rectangle, duplicated across knots to preserve one-sided normals
    /// and disconnected geometry. This is a render mesh, not a welded B-Rep mesh.
    pub fn mesh_uniform(
        &self,
        subdivisions: usize,
        max_vertices: usize,
    ) -> Result<Mesh, DataError> {
        if subdivisions == 0 {
            return Err(DataError::new("subdivisions", "must be positive"));
        }
        let us: Vec<_> = self.u().spans().collect();
        let vs: Vec<_> = self.v().spans().collect();
        let side = subdivisions.checked_add(1);
        let count = side
            .and_then(|s| s.checked_mul(s))
            .and_then(|n| n.checked_mul(us.len()))
            .and_then(|n| n.checked_mul(vs.len()));
        let count = count
            .filter(|&n| n <= max_vertices)
            .ok_or_else(|| DataError::new("max_vertices", "mesh vertex budget exceeded"))?;
        let side = side.unwrap();
        let mut mesh = Mesh {
            vertices: Vec::with_capacity(count),
            triangles: Vec::new(),
            subdivisions,
            sampled_error: 0.0,
            degenerate_triangles: 0,
        };
        for &(su, [ua, ub]) in &us {
            for &(sv, [va, vb]) in &vs {
                let base = mesh.vertices.len();
                for i in 0..=subdivisions {
                    for j in 0..=subdivisions {
                        let uv = [
                            parameter(ua, ub, i, subdivisions),
                            parameter(va, vb, j, subdivisions),
                        ];
                        let sample = self.evaluate_in_spans(uv, [su, sv])?;
                        mesh.vertices.push(MeshVertex {
                            uv,
                            point: sample.point,
                            normal: sample.normal,
                            spans: [su, sv],
                        });
                    }
                }
                for i in 0..subdivisions {
                    for j in 0..subdivisions {
                        let a = base + i * side + j;
                        let b = a + side;
                        let c = b + 1;
                        let d = a + 1;
                        for triangle in [[a, b, c], [a, c, d]] {
                            let [pa, pb, pc] = triangle.map(|k| mesh.vertices[k].point);
                            let area = norm(cross(sub(pb, pa), sub(pc, pa)));
                            if !area.is_finite() {
                                return Err(DataError::new("mesh", "triangle area overflow"));
                            }
                            if area == 0.0 {
                                mesh.degenerate_triangles += 1;
                            } else {
                                mesh.triangles.push(triangle);
                            }
                            // Probe both triangle interiors and all their edge midpoints;
                            // compare with the actual affine triangle, not a bilinear patch.
                            for bary in
                                [[1. / 3.; 3], [0.5, 0.5, 0.], [0., 0.5, 0.5], [0.5, 0., 0.5]]
                            {
                                let uv = std::array::from_fn(|axis| {
                                    (0..3)
                                        .map(|k| bary[k] * mesh.vertices[triangle[k]].uv[axis])
                                        .sum()
                                });
                                let approx = std::array::from_fn(|axis| {
                                    (0..3)
                                        .map(|k| bary[k] * mesh.vertices[triangle[k]].point[axis])
                                        .sum()
                                });
                                let exact = self.evaluate_in_spans(uv, [su, sv])?.point;
                                let error = norm(sub(exact, approx));
                                if !error.is_finite() {
                                    return Err(DataError::new("mesh", "sampled error overflow"));
                                }
                                mesh.sampled_error = mesh.sampled_error.max(error);
                            }
                        }
                    }
                }
            }
        }
        Ok(mesh)
    }

    /// Globally double the per-span grid until sampled positional error passes.
    /// No local quadtree/T-junctions; no rigorous global error or normal-angle bound.
    pub fn tessellate(&self, options: MeshOptions) -> Result<Mesh, DataError> {
        if !options.tolerance.is_finite()
            || options.tolerance <= 0.0
            || options.initial_subdivisions == 0
            || options.initial_subdivisions > options.max_subdivisions
        {
            return Err(DataError::new(
                "mesh_options",
                "positive finite tolerance and 1 <= initial <= maximum subdivisions required",
            ));
        }
        let mut n = options.initial_subdivisions;
        // doc:refine:start
        loop {
            let mesh = self.mesh_uniform(n, options.max_vertices)?;
            if mesh.sampled_error <= options.tolerance {
                return Ok(mesh);
            }
            if n == options.max_subdivisions {
                return Err(DataError::new(
                    "max_subdivisions",
                    &format!(
                        "sampled error {} exceeds tolerance {}",
                        mesh.sampled_error, options.tolerance
                    ),
                ));
            }
            n = n.saturating_mul(2).min(options.max_subdivisions);
        }
        // doc:refine:end
    }
}
