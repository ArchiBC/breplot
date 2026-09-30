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
            silhouettes: true,
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
                lines.extend(silhouettes(&part, view));
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
/// Piecewise-linear zero contour of n dot view on the display mesh. This is
/// view dependent and approximate; invalid pole normals are skipped explicitly.
fn silhouettes(mesh: &Mesh, view: [f64; 3]) -> Vec<SurfaceLine> {
    let mut result = vec![];
    for t in &mesh.triangles {
        let v = t.map(|i| mesh.vertices[i]);
        let Some(n) = v.iter().map(|v| v.normal).collect::<Option<Vec<_>>>() else {
            continue;
        };
        let f: Vec<_> = n
            .iter()
            .map(|n| n.iter().zip(view).map(|(a, b)| a * b).sum::<f64>())
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
                let u = f[a] / (f[a] - f[b]);
                hits.push(std::array::from_fn(|k| {
                    v[a].point[k] + u * (v[b].point[k] - v[a].point[k])
                }));
            }
        }
        if hits.len() >= 2 && norm(sub(hits[0], hits[1])) > 1e-12 {
            result.push(line(LineKind::Silhouette, vec![hits[0], hits[1]]));
        }
    }
    result
}
