//! STEP to a smooth-surface and vector-edge orthographic drawing for Typst.

mod bezier;
mod camera;
mod curve_display;
mod hlr;
mod raster;
mod shade;
mod step;

use std::collections::HashMap;

use bezier::{Cubic3, approximate_edge, flatten};
use brepkit_io::step::read_step;
use brepkit_math::vec::Vec3;
use brepkit_operations::tessellate::tessellate_solid;
use brepkit_topology::Topology;
use brepkit_topology::edge::EdgeCurve;
use brepkit_topology::explorer::solid_edges;
use camera::Camera;
use hlr::{Point, Scene, Triangle};
use serde::{Deserialize, Serialize};
use shade::Facet;

pub use cetz_nurbs::display::native_cubics;
pub use cetz_nurbs::nurbs::{CurveStyle, NurbsInput};
pub use curve_display::render_nurbs;

#[cfg(target_arch = "wasm32")]
use wasm_minimal_protocol::*;
#[cfg(target_arch = "wasm32")]
initiate_protocol!();

#[derive(Clone, Debug, Deserialize)]
#[serde(default)]
pub struct Config {
    /// CeTZ draw.ortho rotation angles x/y/z, in degrees. Overrides direction/up.
    pub view: Option<[f64; 3]>,
    /// From the viewer into the model.
    pub direction: [f64; 3],
    pub up: [f64; 3],
    /// Sample-checked edge cubic approximation tolerance, in STEP units.
    pub deflection: f64,
    /// Face tessellation tolerance for visibility tests, in STEP units.
    pub visibility_deflection: f64,
    /// Face tessellation tolerance for shading and optional silhouettes.
    pub surface_deflection: f64,
    /// Maximum HLR proxy chord length, in STEP units.
    pub max_segment_length: f64,
    pub hidden: bool,
    pub silhouette: bool,
    /// Draw shaded surfaces below the technical linework.
    pub surface: bool,
    /// "raster" for smooth per-pixel light, "flat" for legacy vector facets.
    pub surface_mode: SurfaceMode,
    /// Width of the transparent surface raster layer in pixels.
    pub surface_pixels: u32,
    /// Strength of the view-dependent surface highlight, in [0, 1].
    pub specular: f64,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SurfaceMode {
    #[default]
    Raster,
    Flat,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            view: None,
            direction: [1.0, 1.0, -1.0],
            up: [0.0, 0.0, 1.0],
            deflection: 0.005,
            visibility_deflection: 0.001,
            surface_deflection: 0.02,
            max_segment_length: 0.5,
            hidden: false,
            silhouette: false,
            surface: true,
            surface_mode: SurfaceMode::Raster,
            surface_pixels: 1600,
            specular: 0.18,
        }
    }
}

#[derive(Default, Debug)]
pub struct Stats {
    pub solids: usize,
    pub normalized_axes: usize,
    pub triangles: usize,
    pub edge_curves: usize,
    pub bezier_segments: usize,
    /// Number of short proxy chords tested against the HLR mesh.
    pub edge_segments: usize,
    pub visible_fragments: usize,
    pub hidden_fragments: usize,
    pub silhouette_fragments: usize,
    pub shaded_facets: usize,
    pub max_edge_segment_length: f64,
}

#[derive(Clone, Copy)]
struct Cubic2 {
    points: [Point; 4],
}

impl Cubic2 {
    fn from_world(curve: Cubic3, camera: &Camera) -> Self {
        Self {
            points: curve.points.map(|p| camera.project(p)),
        }
    }
}

fn segment_fraction(a: Point, b: Point, p: Point) -> f64 {
    let dx = b.x - a.x;
    let dy = b.y - a.y;
    let denom = dx * dx + dy * dy;
    if denom <= 1e-20 {
        0.0
    } else {
        (((p.x - a.x) * dx + (p.y - a.y) * dy) / denom).clamp(0.0, 1.0)
    }
}

struct StepDrawing {
    visible: Vec<Cubic2>,
    hidden: Vec<Cubic2>,
    silhouettes: Vec<(Point, Point)>,
    facets: Vec<Facet>,
    light: Vec3,
    fill: Vec3,
    viewer: Vec3,
}

fn build_step(step_bytes: &[u8], config: &Config) -> Result<(StepDrawing, Stats), String> {
    for (name, value) in [
        ("deflection", config.deflection),
        ("visibility_deflection", config.visibility_deflection),
        ("surface_deflection", config.surface_deflection),
        ("max_segment_length", config.max_segment_length),
    ] {
        if !value.is_finite() || value <= 0.0 {
            return Err(format!("{name} must be a positive finite number"));
        }
    }
    if !config.specular.is_finite() || !(0.0..=1.0).contains(&config.specular) {
        return Err("specular must be between 0 and 1".into());
    }
    if !(256..=4096).contains(&config.surface_pixels) {
        return Err("surface_pixels must be between 256 and 4096".into());
    }
    let camera = if let Some(view) = config.view {
        Camera::from_ortho(view)?
    } else {
        Camera::new(config.direction, config.up)?
    };
    let viewer = -camera.toward_scene;
    let light = (-camera.x * 0.48 + camera.y * 0.62 + viewer * 0.72)
        .normalize()
        .map_err(|_| "invalid key light")?;
    let fill = (camera.x * 0.65 + camera.y * 0.2 + viewer * 0.35)
        .normalize()
        .map_err(|_| "invalid fill light")?;
    let source = std::str::from_utf8(step_bytes).map_err(|_| "STEP input must be UTF-8")?;
    let (normalized, normalized_axes) = step::normalize_optional_axis_directions(source)?;
    let mut topo = Topology::new();
    let solids = read_step(&normalized, &mut topo).map_err(|e| format!("STEP import: {e}"))?;
    if solids.is_empty() {
        return Err("STEP import found no MANIFOLD_SOLID_BREP".into());
    }

    let mut stats = Stats {
        solids: solids.len(),
        normalized_axes,
        ..Stats::default()
    };
    let mut triangles = Vec::new();
    let mut facets = Vec::<Facet>::new();
    let mut edge_cubics: Vec<Cubic3> = Vec::new();
    let mut silhouettes = Vec::<(Point, Point)>::new();

    for solid in solids {
        let visibility_mesh = tessellate_solid(&topo, solid, config.visibility_deflection)
            .map_err(|e| format!("visibility tessellation: {e}"))?;
        let visibility_points: Vec<Point> = visibility_mesh
            .positions
            .iter()
            .map(|&p| camera.project(p))
            .collect();
        for ids in visibility_mesh.indices.chunks_exact(3) {
            let p = std::array::from_fn(|i| visibility_points[ids[i] as usize]);
            if let Some(tri) = Triangle::new(p) {
                triangles.push(tri);
            }
        }
        drop(visibility_mesh);
        drop(visibility_points);

        let mesh = tessellate_solid(&topo, solid, config.surface_deflection)
            .map_err(|e| format!("surface tessellation: {e}"))?;
        let projected: Vec<Point> = mesh.positions.iter().map(|&p| camera.project(p)).collect();
        // A mesh edge shared by one front and one back triangle approximates a
        // smooth surface silhouette. Keep it separate from true B-Rep edges.
        let mut adjacency = HashMap::<(u32, u32), Vec<bool>>::new();
        for ids in mesh.indices.chunks_exact(3) {
            let p = [
                projected[ids[0] as usize],
                projected[ids[1] as usize],
                projected[ids[2] as usize],
            ];
            if Triangle::new(p).is_some() {
                if config.surface {
                    let world = [
                        mesh.positions[ids[0] as usize],
                        mesh.positions[ids[1] as usize],
                        mesh.positions[ids[2] as usize],
                    ];
                    let normals = std::array::from_fn(|i| {
                        mesh.normals
                            .get(ids[i] as usize)
                            .copied()
                            .unwrap_or(Vec3::new(0.0, 0.0, 0.0))
                    });
                    if let Some(facet) =
                        Facet::new(p, world, normals, light, fill, viewer, config.specular)
                    {
                        facets.push(facet);
                    }
                }
                let orientation =
                    (p[1].x - p[0].x) * (p[2].y - p[0].y) - (p[1].y - p[0].y) * (p[2].x - p[0].x);
                let front = orientation < 0.0; // screen y points downward
                for e in 0..3 {
                    let a = ids[e];
                    let b = ids[(e + 1) % 3];
                    let key = (a.min(b), a.max(b));
                    adjacency.entry(key).or_default().push(front);
                }
            }
        }
        if config.silhouette {
            for ((a, b), sides) in adjacency {
                if sides.len() == 2 && sides[0] != sides[1] {
                    silhouettes.push((projected[a as usize], projected[b as usize]));
                }
            }
        }

        for edge_id in solid_edges(&topo, solid).map_err(|e| format!("edge traversal: {e}"))? {
            let edge = topo
                .edge(edge_id)
                .map_err(|e| format!("edge lookup: {e}"))?;
            let start = topo
                .vertex(edge.start())
                .map_err(|e| format!("vertex lookup: {e}"))?
                .point();
            let end = topo
                .vertex(edge.end())
                .map_err(|e| format!("vertex lookup: {e}"))?
                .point();
            let curves = match edge.curve() {
                EdgeCurve::NurbsCurve(curve) => curve_display::edge_cubics(
                    curve,
                    edge.curve().domain_with_endpoints(start, end),
                    config.deflection,
                )?,
                other => approximate_edge(other, start, end, config.deflection),
            };
            if !curves.is_empty() {
                stats.edge_curves += 1;
                stats.bezier_segments += curves.len();
                edge_cubics.extend(curves);
            }
        }
    }

    let scene = Scene::new(triangles);
    stats.triangles = scene.triangle_count();
    // A tolerance at the same scale as the face mesh prevents an edge chord
    // from being hidden by the adjacent approximated face. It can also hide
    // very close genuine occlusions; this is a documented demo limitation.
    let depth_eps = config.deflection * 1.5;
    let mut visible = Vec::<Cubic2>::new();
    let mut hidden = Vec::<Cubic2>::new();
    for curve in edge_cubics {
        let proxy = flatten(&curve, config.max_segment_length, config.deflection * 0.2);
        for pair in proxy.windows(2) {
            let (t0, w0) = pair[0];
            let (t1, w1) = pair[1];
            stats.edge_segments += 1;
            stats.max_edge_segment_length = stats.max_edge_segment_length.max((w1 - w0).length());
            let a = camera.project(w0);
            let b = camera.project(w1);
            for part in scene.split(a, b, depth_eps) {
                let start_t = t0 + (t1 - t0) * segment_fraction(a, b, part.a);
                let end_t = t0 + (t1 - t0) * segment_fraction(a, b, part.b);
                let piece = Cubic2::from_world(curve.portion(start_t, end_t), &camera);
                if part.hidden {
                    hidden.push(piece);
                } else {
                    visible.push(piece);
                }
            }
        }
    }
    stats.visible_fragments = visible.len();
    stats.hidden_fragments = hidden.len();

    let mut silhouette_visible = Vec::new();
    // The displayed silhouette comes from a coarser surface mesh. Give it a
    // matching depth allowance when tested against the denser visibility mesh.
    let silhouette_depth_eps = depth_eps.max(config.surface_deflection * 2.0);
    for (a, b) in silhouettes {
        for part in scene.split(a, b, silhouette_depth_eps) {
            if !part.hidden {
                silhouette_visible.push((part.a, part.b));
            }
        }
    }
    stats.silhouette_fragments = silhouette_visible.len();
    facets.sort_by(|a, b| a.depth.total_cmp(&b.depth));
    stats.shaded_facets = facets.len();
    Ok((
        StepDrawing {
            visible,
            hidden,
            silhouettes: silhouette_visible,
            facets,
            light,
            fill,
            viewer,
        },
        stats,
    ))
}

pub fn render_step(step_bytes: &[u8], config: &Config) -> Result<(String, Stats), String> {
    let (drawing, stats) = build_step(step_bytes, config)?;
    let svg = write_svg(
        &drawing.visible,
        &drawing.hidden,
        &drawing.silhouettes,
        &drawing.facets,
        config.hidden,
        config.surface,
        config.surface_mode,
        config.surface_pixels,
        drawing.light,
        drawing.fill,
        drawing.viewer,
        config.specular,
    )?;
    Ok((svg, stats))
}

fn drawing_bounds(drawing: &StepDrawing) -> Result<[f64; 4], String> {
    let mut bounds = [
        f64::INFINITY,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NEG_INFINITY,
    ];
    for p in drawing
        .visible
        .iter()
        .chain(&drawing.hidden)
        .flat_map(|c| c.points)
        .chain(drawing.silhouettes.iter().flat_map(|&(a, b)| [a, b]))
    {
        bounds[0] = bounds[0].min(p.x);
        bounds[1] = bounds[1].min(p.y);
        bounds[2] = bounds[2].max(p.x);
        bounds[3] = bounds[3].max(p.y);
    }
    if !bounds[0].is_finite() {
        return Err("projection produced no drawable paths".into());
    }
    let width = (bounds[2] - bounds[0]).max(1e-3);
    let height = (bounds[3] - bounds[1]).max(1e-3);
    let pad = width.max(height) * 0.04;
    Ok([
        bounds[0] - pad,
        bounds[1] - pad,
        width + 2.0 * pad,
        height + 2.0 * pad,
    ])
}

#[derive(Serialize)]
struct NativeStepData {
    view: [f64; 4],
    visible: Vec<[[f64; 2]; 4]>,
    hidden: Vec<[[f64; 2]; 4]>,
    silhouette: Vec<[[f64; 2]; 2]>,
    facets: Vec<NativeFacet>,
}

#[derive(Serialize)]
struct NativeFacet {
    points: [[f64; 2]; 3],
    color: [u8; 3],
}

pub fn render_step_data(step_bytes: &[u8], config: &Config) -> Result<Vec<u8>, String> {
    let (drawing, _) = build_step(step_bytes, config)?;
    let view = drawing_bounds(&drawing)?;
    let png = if config.surface && config.surface_mode == SurfaceMode::Raster {
        raster::render_png(
            &drawing.facets,
            view,
            drawing.light,
            drawing.fill,
            drawing.viewer,
            config.specular,
            config.surface_pixels,
        )?
        .0
    } else {
        Vec::new()
    };
    let point = |p: Point| [p.x, p.y];
    let cubics = |curves: &[Cubic2]| curves.iter().map(|c| c.points.map(point)).collect();
    let facets = if config.surface && config.surface_mode == SurfaceMode::Flat {
        drawing
            .facets
            .iter()
            .map(|f| NativeFacet {
                points: f.points.map(point),
                color: f.color,
            })
            .collect()
    } else {
        Vec::new()
    };
    let data = NativeStepData {
        view,
        visible: cubics(&drawing.visible),
        hidden: cubics(&drawing.hidden),
        silhouette: drawing
            .silhouettes
            .iter()
            .map(|&(a, b)| [point(a), point(b)])
            .collect(),
        facets,
    };
    let json = serde_json::to_vec(&data).map_err(|e| format!("encode STEP drawing: {e}"))?;
    let json_len = u32::try_from(json.len()).map_err(|_| "STEP drawing metadata too large")?;
    let mut payload = Vec::with_capacity(4 + json.len() + png.len());
    payload.extend_from_slice(&json_len.to_le_bytes());
    payload.extend_from_slice(&json);
    payload.extend_from_slice(&png);
    Ok(payload)
}

fn write_svg(
    visible: &[Cubic2],
    hidden: &[Cubic2],
    silhouettes: &[(Point, Point)],
    facets: &[Facet],
    show_hidden: bool,
    surface: bool,
    surface_mode: SurfaceMode,
    surface_pixels: u32,
    light: Vec3,
    fill: Vec3,
    viewer: Vec3,
    specular: f64,
) -> Result<String, String> {
    let points = visible
        .iter()
        .chain(hidden)
        .flat_map(|c| c.points)
        .chain(silhouettes.iter().flat_map(|&(a, b)| [a, b]));
    let mut bounds = [
        f64::INFINITY,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NEG_INFINITY,
    ];
    for p in points {
        bounds[0] = bounds[0].min(p.x);
        bounds[1] = bounds[1].min(p.y);
        bounds[2] = bounds[2].max(p.x);
        bounds[3] = bounds[3].max(p.y);
    }
    if !bounds[0].is_finite() {
        return Err("projection produced no drawable paths".into());
    }
    let width = (bounds[2] - bounds[0]).max(1e-3);
    let height = (bounds[3] - bounds[1]).max(1e-3);
    let pad = width.max(height) * 0.04;
    let stroke = width.max(height) * 0.0018;
    let mut out = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"{:.6} {:.6} {:.6} {:.6}\" width=\"800\" height=\"{:.3}\">\n",
        bounds[0] - pad,
        bounds[1] - pad,
        width + 2.0 * pad,
        height + 2.0 * pad,
        800.0 * (height + 2.0 * pad) / (width + 2.0 * pad)
    );
    out.push_str("<g fill=\"none\" stroke-linecap=\"butt\" stroke-linejoin=\"round\">\n");
    if surface {
        if surface_mode == SurfaceMode::Raster {
            let view = [
                bounds[0] - pad,
                bounds[1] - pad,
                width + 2.0 * pad,
                height + 2.0 * pad,
            ];
            let (png, _) = raster::render_png_base64(
                facets,
                view,
                light,
                fill,
                viewer,
                specular,
                surface_pixels,
            )?;
            out.push_str(&format!("<image id=\"surfaces\" x=\"{:.6}\" y=\"{:.6}\" width=\"{:.6}\" height=\"{:.6}\" href=\"data:image/png;base64,{}\"/>\n",view[0],view[1],view[2],view[3],png));
        } else {
            out.push_str("<g id=\"surfaces\">\n");
            let seam_width = width.max(height) * 0.00045;
            for facet in facets {
                let [a, b, c] = facet.points;
                let [r, g, b_color] = facet.color;
                out.push_str(&format!(
                    "<path d=\"M{:.4},{:.4}L{:.4},{:.4}L{:.4},{:.4}Z\" fill=\"#{r:02x}{g:02x}{b_color:02x}\" stroke=\"#{r:02x}{g:02x}{b_color:02x}\" stroke-width=\"{seam_width:.5}\"/>\n",
                    a.x, a.y, b.x, b.y, c.x, c.y
                ));
            }
            out.push_str("</g>\n");
        }
    }
    if show_hidden {
        write_curve_layer(
            &mut out,
            "hidden",
            hidden,
            "#61758c",
            stroke * 0.6,
            true,
            if surface { 0.5 } else { 1.0 },
        );
    }
    write_curve_layer(
        &mut out,
        "visible",
        visible,
        if surface { "#31475e" } else { "#172232" },
        stroke * if surface { 0.72 } else { 1.0 },
        false,
        if surface { 0.72 } else { 1.0 },
    );
    write_layer(
        &mut out,
        "silhouette",
        silhouettes,
        if surface { "#1b2c40" } else { "#111827" },
        stroke * if surface { 1.0 } else { 1.35 },
        false,
        1.0,
    );
    out.push_str("</g></svg>\n");
    Ok(out)
}

fn write_curve_layer(
    out: &mut String,
    id: &str,
    curves: &[Cubic2],
    color: &str,
    stroke: f64,
    dashed: bool,
    opacity: f64,
) {
    let dash = if dashed {
        format!(
            " stroke-dasharray=\"{:.5} {:.5}\"",
            stroke * 4.0,
            stroke * 3.0
        )
    } else {
        String::new()
    };
    out.push_str(&format!("<g id=\"{id}\" stroke=\"{color}\" stroke-opacity=\"{opacity:.3}\" stroke-width=\"{stroke:.6}\"{dash}><path d=\""));
    let mut last_end: Option<Point> = None;
    for curve in curves {
        let [a, b, c, d] = curve.points;
        if !last_end.is_some_and(|p| (p.x - a.x).abs() <= 1e-8 && (p.y - a.y).abs() <= 1e-8) {
            out.push_str(&format!("M{:.5},{:.5}", a.x, a.y));
        }
        out.push_str(&format!(
            "C{:.5},{:.5} {:.5},{:.5} {:.5},{:.5}",
            b.x, b.y, c.x, c.y, d.x, d.y
        ));
        last_end = Some(d);
    }
    out.push_str("\"/></g>\n");
}

fn write_layer(
    out: &mut String,
    id: &str,
    lines: &[(Point, Point)],
    color: &str,
    stroke: f64,
    dashed: bool,
    opacity: f64,
) {
    let dash = if dashed {
        format!(
            " stroke-dasharray=\"{:.5} {:.5}\"",
            stroke * 4.0,
            stroke * 3.0
        )
    } else {
        String::new()
    };
    out.push_str(&format!(
        "<g id=\"{id}\" stroke=\"{color}\" stroke-opacity=\"{opacity:.3}\" stroke-width=\"{stroke:.6}\"{dash}><path d=\""
    ));
    let mut last_end: Option<Point> = None;
    for &(a, b) in lines {
        let joins_previous =
            last_end.is_some_and(|p| (p.x - a.x).abs() <= 1e-8 && (p.y - a.y).abs() <= 1e-8);
        if !joins_previous {
            out.push_str(&format!("M{:.5},{:.5}", a.x, a.y));
        }
        out.push_str(&format!("L{:.5},{:.5}", b.x, b.y));
        last_end = Some(b);
    }
    out.push_str("\"/></g>\n");
}

#[cfg(target_arch = "wasm32")]
#[wasm_func]
fn render_step_native(step_bytes: &[u8], config_json: &[u8]) -> Result<Vec<u8>, String> {
    let config: Config = serde_json::from_slice(config_json).map_err(|e| format!("config: {e}"))?;
    render_step_data(step_bytes, &config)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn segment_fraction_uses_projected_chord() {
        let a = Point {
            x: 1.0,
            y: 2.0,
            depth: 0.0,
        };
        let b = Point {
            x: 3.0,
            y: 4.0,
            depth: 0.0,
        };
        assert!(
            (segment_fraction(
                a,
                b,
                Point {
                    x: 1.5,
                    y: 2.5,
                    depth: 0.0
                }
            ) - 0.25)
                .abs()
                < 1e-12
        );
    }
}
