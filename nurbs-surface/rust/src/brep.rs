//! Display-oriented face/edge assembly. No sewing or solid-validity claims.
use crate::evaluate::{norm, sub};
use crate::{
    DataError, LineKind, Mesh, MeshOptions, NurbsSurface, SurfaceLine, SvgOptions, TrimRegion,
};
use std::collections::BTreeSet;

// doc:display:start
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DisplayMode {
    Shaded,
    HiddenLine,
    Wireframe,
}
#[derive(Clone, Copy, Debug)]
pub struct DisplayOptions {
    pub mode: DisplayMode,
    pub control_points: bool,
    pub control_net: bool,
    pub structure_lines: bool,
    /// Interior isocurves per face, distributed over its active parameter domain.
    pub iso_count: [usize; 2],
    pub boundaries: bool,
    /// Experimental view-dependent contours; disabled by default.
    pub silhouettes: bool,
    pub samples_per_span: usize,
}
// doc:display:end
impl Default for DisplayOptions {
    fn default() -> Self {
        Self {
            mode: DisplayMode::Shaded,
            control_points: false,
            control_net: false,
            structure_lines: true,
            iso_count: [4, 4],
            boundaries: true,
            silhouettes: false,
            samples_per_span: 16,
        }
    }
}
// doc:brep:start
#[derive(Clone, Debug)]
pub struct BrepFace {
    pub surface: NurbsSurface,
    pub trim: Option<TrimRegion>,
    pub reversed: bool,
    /// Shared edge IDs. Empty uses this face's parametric/trim boundaries.
    pub boundary_edges: Vec<usize>,
}
#[derive(Clone, Debug)]
pub struct BrepEdge {
    /// Explicit world-space display polyline, NOT a recovered exact edge curve.
    pub points: Vec<[f64; 3]>,
}
#[derive(Clone, Debug, Default)]
pub struct Brep {
    pub faces: Vec<BrepFace>,
    pub edges: Vec<BrepEdge>,
}
#[derive(Clone, Debug)]
pub struct DisplayScene {
    pub mesh: Mesh,
    pub lines: Vec<SurfaceLine>,
    pub render: SvgOptions,
}
// doc:brep:end
impl DisplayScene {
    pub fn svg(&self) -> Result<String, DataError> {
        crate::render_svg_with_lines(&self.mesh, &self.lines, self.render)
    }
    pub fn pdf(&self) -> Result<Vec<u8>, DataError> {
        crate::render_pdf_with_lines(&self.mesh, &self.lines, self.render)
    }
}
fn line(kind: LineKind, points: Vec<[f64; 3]>) -> SurfaceLine {
    SurfaceLine {
        kind,
        points,
        uv: vec![],
    }
}
impl Brep {
    pub fn display(
        &self,
        display: DisplayOptions,
        mesh_options: MeshOptions,
        mut render: SvgOptions,
    ) -> Result<DisplayScene, DataError> {
        if self.faces.is_empty() || self.faces.len() > 1024 {
            return Err(DataError::new("brep.faces", "requires 1..1024 faces"));
        }
        if display.iso_count.iter().any(|&n| n > 128)
            || !(1..=1024).contains(&display.samples_per_span)
        {
            return Err(DataError::new(
                "display",
                "iso counts must be 0..128; samples per span 1..1024",
            ));
        }
        let length = norm(render.view);
        if !length.is_finite() || length == 0. {
            return Err(DataError::new("view", "finite nonzero direction required"));
        }
        let view = render.view.map(|v| v / length);
        render.draw_surfaces = display.mode == DisplayMode::Shaded;
        render.occlude_lines = display.mode != DisplayMode::Wireframe;
        // HiddenLine intentionally has opaque occluders even without painted faces.
        if display.mode != DisplayMode::Shaded {
            render.opacity = 1.;
            render.wireframe = false;
        }
        let mut mesh = Mesh {
            vertices: vec![],
            triangles: vec![],
            subdivisions: 0,
            sampled_error: 0.,
            degenerate_triangles: 0,
        };
        let mut lines = vec![];
        let mut edge_ids = BTreeSet::new();
        for (face_id, face) in self.faces.iter().enumerate() {
            for &id in &face.boundary_edges {
                if id >= self.edges.len() {
                    return Err(DataError::new("brep.edges", "face references missing edge"));
                }
                if display.boundaries {
                    edge_ids.insert(id);
                }
            }
            let mut part = if let Some(r) = &face.trim {
                face.surface.tessellate_trimmed(r, mesh_options)?
            } else {
                face.surface.tessellate(mesh_options)?
            };
            if face.reversed {
                for v in &mut part.vertices {
                    v.normal = v.normal.map(|n| n.map(|x| -x));
                }
                for t in &mut part.triangles {
                    t.swap(1, 2);
                }
            }
            if mesh.vertices.len() + part.vertices.len() > mesh_options.max_vertices {
                return Err(DataError::new(
                    "brep.mesh",
                    "combined vertex budget exceeded",
                ));
            }
            if display.silhouettes {
                lines.extend(silhouettes(&face.surface, &part, view)?);
            }
            let d = face.surface.domain();
            let values = |axis: usize| -> Vec<f64> {
                if !display.structure_lines {
                    return vec![];
                }
                (1..=display.iso_count[axis])
                    .map(|i| {
                        d[axis][0]
                            + (d[axis][1] - d[axis][0]) * i as f64
                                / (display.iso_count[axis] + 1) as f64
                    })
                    .collect()
            };
            let mut iso = face.surface.structure_lines(
                face.trim.as_ref(),
                &values(0),
                &values(1),
                display.samples_per_span,
            )?;
            iso.retain(|l| {
                l.kind != LineKind::TrimBoundary
                    || (display.boundaries && face.boundary_edges.is_empty())
            });
            lines.extend(iso);
            if display.boundaries && face.boundary_edges.is_empty() && face.trim.is_none() {
                let mut boundary =
                    face.surface
                        .structure_lines(None, &d[0], &d[1], display.samples_per_span)?;
                for l in &mut boundary {
                    l.kind = LineKind::Boundary;
                }
                lines.extend(boundary);
            }
            if display.control_points {
                lines.push(line(
                    LineKind::ControlPoint,
                    face.surface.control_points().to_vec(),
                ));
            }
            if display.control_net {
                let [nu, nv] = face.surface.dimensions();
                for u in 0..nu {
                    lines.push(line(
                        LineKind::ControlNet,
                        (0..nv)
                            .map(|v| face.surface.control_point(u, v).unwrap())
                            .collect(),
                    ));
                }
                for v in 0..nv {
                    lines.push(line(
                        LineKind::ControlNet,
                        (0..nu)
                            .map(|u| face.surface.control_point(u, v).unwrap())
                            .collect(),
                    ));
                }
            }
            let offset = mesh.vertices.len();
            mesh.triangles
                .extend(part.triangles.iter().map(|t| t.map(|i| i + offset)));
            mesh.vertices.extend(part.vertices);
            mesh.sampled_error = mesh.sampled_error.max(part.sampled_error);
            mesh.subdivisions = mesh.subdivisions.max(part.subdivisions);
            mesh.degenerate_triangles += part.degenerate_triangles;
            if lines.iter().map(|l| l.points.len()).sum::<usize>() > 500_000 {
                return Err(DataError::new(
                    &format!("brep.faces[{face_id}]"),
                    "line point budget exceeded",
                ));
            }
        }
        let mut point_count = lines.iter().map(|l| l.points.len()).sum::<usize>();
        for id in edge_ids {
            let edge = &self.edges[id];
            if edge.points.len() > 500_000 - point_count {
                return Err(DataError::new("brep.edges", "line point budget exceeded"));
            }
            point_count += edge.points.len();
            if edge.points.len() < 2 || edge.points.iter().flatten().any(|x| !x.is_finite()) {
                return Err(DataError::new(
                    "brep.edges",
                    "edge requires at least two finite points",
                ));
            }
            lines.push(line(LineKind::Boundary, edge.points.clone()));
        }
        Ok(DisplayScene {
            mesh,
            lines,
            render,
        })
    }
}
/// Locate contour crossings in parameter triangles, then solve n dot view = 0
/// on the original surface. Mesh resolution still controls contour topology.
fn silhouettes(
    surface: &NurbsSurface,
    mesh: &Mesh,
    view: [f64; 3],
) -> Result<Vec<SurfaceLine>, DataError> {
    let mut result = vec![];
    let mut segments = BTreeSet::new();
    for t in &mesh.triangles {
        let v = t.map(|i| mesh.vertices[i]);
        let Some(n) = v.iter().map(|v| v.normal).collect::<Option<Vec<_>>>() else {
            continue;
        };
        let f: Vec<_> = n
            .iter()
            .map(|n| {
                let f = n.iter().zip(view).map(|(a, b)| a * b).sum::<f64>();
                if f.abs() < 1e-12 { 0. } else { f }
            })
            .collect();
        if f.iter().all(|x| x.abs() < 1e-12) {
            continue;
        }
        let mut hits = vec![];
        for (a, b) in [(0, 1), (1, 2), (2, 0)] {
            if f[a] == 0. {
                hits.push(v[a].point);
            }
            if f[a] * f[b] < 0. {
                // Solve on the original surface, not the chord inside it. Reversed
                // face normals do not affect the zero set; use analytic signs here.
                let spans = v[a].spans;
                let at = |s: f64| {
                    surface.evaluate_in_spans(
                        std::array::from_fn(|k| v[a].uv[k] + s * (v[b].uv[k] - v[a].uv[k])),
                        spans,
                    )
                };
                let mut lo = 0.;
                let mut hi = 1.;
                let start_normal = match at(lo)?.normal {
                    Some(n) => Some(n),
                    None => at(1e-6)?.normal,
                };
                let sign = start_normal
                    .map(|na| na.iter().zip(view).map(|(a, b)| a * b).sum::<f64>())
                    .unwrap_or(0.);
                if sign == 0. {
                    continue;
                }
                let mut sample = at(0.5)?;
                for _ in 0..40 {
                    let mid = (lo + hi) * 0.5;
                    sample = at(mid)?;
                    let Some(normal) = sample.normal else { break };
                    let value = normal.iter().zip(view).map(|(a, b)| a * b).sum::<f64>();
                    if value.abs() < 1e-12 {
                        break;
                    }
                    if value * sign > 0. {
                        lo = mid;
                    } else {
                        hi = mid;
                    }
                }
                hits.push(sample.point);
            }
        }
        if hits.len() >= 2 && norm(sub(hits[0], hits[1])) > 1e-12 {
            // A contour on a mesh edge belongs to both adjacent triangles.
            // Emit it once, including when the mesh duplicates knot-cell vertices.
            let key = |p: [f64; 3]| p.map(|x| if x == 0. { 0 } else { x.to_bits() });
            let mut edge = [key(hits[0]), key(hits[1])];
            edge.sort();
            if segments.insert(edge) {
                result.push(line(LineKind::Silhouette, vec![hits[0], hits[1]]));
            }
        }
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tangent_knot_edges_remain_complete_under_normal_roundoff() {
        let surface = crate::demo_surfaces::cylinder();
        let mut mesh = surface.tessellate(MeshOptions::default()).unwrap();
        // The two silhouettes coincide with knot-cell edges. Perturb their
        // normals on both sides of zero, as occurs on imported rational faces.
        for (i, vertex) in mesh.vertices.iter_mut().enumerate() {
            if let Some(n) = &mut vertex.normal {
                if n[1].abs() < 1e-12 {
                    n[1] = if i % 2 == 0 { 1e-15 } else { -1e-15 };
                }
            }
        }
        for reverse in [false, true] {
            if reverse {
                for v in &mut mesh.vertices {
                    v.normal = v.normal.map(|n| n.map(|x| -x));
                }
            }
            let lines = silhouettes(&surface, &mesh, [0., -1., 0.]).unwrap();
            for x in [-1., 1.] {
                let mut intervals: Vec<_> = lines
                    .iter()
                    .filter(|l| (l.points[0][0] - x).abs() < 1e-10)
                    .map(|l| {
                        let mut z = [l.points[0][2], l.points[1][2]];
                        z.sort_by(f64::total_cmp);
                        z
                    })
                    .collect();
                intervals.sort_by(|a, b| a[0].total_cmp(&b[0]));
                let mut end = -1.;
                for [lo, hi] in intervals {
                    assert!(
                        (lo - end).abs() < 1e-10,
                        "gap or duplicate contour: {lo} != {end}"
                    );
                    end = hi;
                }
                assert!((end - 1.).abs() < 1e-10);
            }
        }
    }

    #[test]
    fn oblique_sphere_contour_points_lie_on_surface_and_tangent_plane() {
        let surface = crate::demo_surfaces::sphere();
        let mesh = surface
            .tessellate(MeshOptions {
                tolerance: 0.08,
                ..Default::default()
            })
            .unwrap();
        let view = [1., 2., 3.].map(|x| x / 14_f64.sqrt());
        let lines = silhouettes(&surface, &mesh, view).unwrap();
        assert!(lines.len() > 8);
        for p in lines.iter().flat_map(|l| &l.points) {
            assert!((norm(*p) - 1.).abs() < 1e-10);
            assert!(p.iter().zip(view).map(|(a, b)| a * b).sum::<f64>().abs() < 1e-10);
        }
    }
}
