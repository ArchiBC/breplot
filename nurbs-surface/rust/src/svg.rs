//! Orthographic Gouraud preview. Affine scalar lighting is encoded by one SVG
//! linear gradient per triangle. Painter sorting is not a general visibility solver.
use crate::evaluate::{cross, norm, sub};
use crate::{DataError, Mesh};
use std::fmt::Write;

#[derive(Clone, Copy, Debug)]
pub struct SvgOptions {
    pub width: usize,
    pub height: usize,
    /// Vector from model towards viewer.
    pub view: [f64; 3],
    pub light: [f64; 3],
    pub base_color: [u8; 3],
    pub wireframe: bool,
    pub draw_surfaces: bool,
    pub occlude_lines: bool,
    /// White highlight mixing strength, in [0, 1]. Zero disables highlights.
    pub specular: f64,
    /// Blinn-Phong exponent; higher values produce narrower highlights.
    pub shininess: f64,
    /// Per-surface-fragment opacity, in [0, 1], not whole-image opacity.
    pub opacity: f64,
    /// Sampled final RGB error for normal-interpolated shading (0, 1].
    pub shading_tolerance: f64,
}
impl Default for SvgOptions {
    fn default() -> Self {
        Self {
            width: 600,
            height: 480,
            view: [4., 6., 4.],
            light: [-3., 4., 7.],
            base_color: [84, 155, 194],
            wireframe: false,
            draw_surfaces: true,
            occlude_lines: true,
            specular: 0.25,
            shininess: 32.0,
            opacity: 1.0,
            shading_tolerance: 0.001,
        }
    }
}

fn highlight(n: [f64; 3], light: [f64; 3], view: [f64; 3], strength: f64, exponent: f64) -> f64 {
    if dot(n, light) <= 0. || dot(n, view) <= 0. || strength == 0. {
        return 0.;
    }
    let half = unit(std::array::from_fn(|i| light[i] + view[i])).ok();
    half.map_or(0., |h| strength * dot(n, h).clamp(0., 1.).powf(exponent))
}
fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a.iter().zip(b).map(|(x, y)| x * y).sum()
}
fn unit(a: [f64; 3]) -> Result<[f64; 3], DataError> {
    let n = norm(a);
    if !n.is_finite() || n == 0. {
        return Err(DataError::new("svg", "finite nonzero direction required"));
    }
    Ok(a.map(|x| x / n))
}

/// Gradient direction for affine scalar interpolation over a projected triangle.
fn gradient(p: [[f64; 2]; 3], intensity: [f64; 3]) -> Option<([f64; 2], [f64; 2], f64, f64)> {
    let x1 = p[1][0] - p[0][0];
    let y1 = p[1][1] - p[0][1];
    let x2 = p[2][0] - p[0][0];
    let y2 = p[2][1] - p[0][1];
    let det = x1 * y2 - x2 * y1;
    if det.abs() < 1e-14 {
        return None;
    }
    // doc:gradient:start
    let d1 = intensity[1] - intensity[0];
    let d2 = intensity[2] - intensity[0];
    let g = [(d1 * y2 - d2 * y1) / det, (x1 * d2 - x2 * d1) / det];
    let length2 = g[0] * g[0] + g[1] * g[1];
    let low = intensity.into_iter().fold(f64::INFINITY, f64::min);
    let high = intensity.into_iter().fold(f64::NEG_INFINITY, f64::max);
    if high - low < 1e-10 || length2 == 0. {
        return None;
    }
    let start = std::array::from_fn(|c| p[0][c] + g[c] * (low - intensity[0]) / length2);
    let end = std::array::from_fn(|c| p[0][c] + g[c] * (high - intensity[0]) / length2);
    // doc:gradient:end
    Some((start, end, low, high))
}

// A single convex clip avoids cracks inside unions of separately rasterized paths.
// Bevel corners bound expansion even for extremely thin triangles.
fn triangle_path(p: [[f64; 2]; 3], expand: bool) -> String {
    let mut svg = String::new();
    let area =
        (p[1][0] - p[0][0]) * (p[2][1] - p[0][1]) - (p[1][1] - p[0][1]) * (p[2][0] - p[0][0]);
    let mut outline = Vec::new();
    if expand && area != 0. {
        let normals: [[f64; 2]; 3] = std::array::from_fn(|i| {
            let a = p[i];
            let b = p[(i + 1) % 3];
            let length = (b[0] - a[0]).hypot(b[1] - a[1]);
            if length == 0. {
                [0., 0.]
            } else {
                [
                    (b[1] - a[1]) * 0.6 * area.signum() / length,
                    -(b[0] - a[0]) * 0.6 * area.signum() / length,
                ]
            }
        });
        for i in 0..3 {
            for n in [normals[(i + 2) % 3], normals[i]] {
                outline.push([p[i][0] + n[0], p[i][1] + n[1]]);
            }
        }
    } else {
        outline.extend(p);
    }
    for (i, v) in outline.iter().enumerate() {
        write!(svg, "{} {} {} ", if i == 0 { "M" } else { "L" }, v[0], v[1]).unwrap();
    }
    svg.push('Z');
    svg
}
fn triangle_clip(svg: &mut String, id: usize, p: [[f64; 2]; 3], expand: bool) {
    write!(svg,"<defs><clipPath id=\"c{id}\" clipPathUnits=\"userSpaceOnUse\"><path d=\"{}\"/></clipPath></defs>",triangle_path(p,expand)).unwrap();
}

#[derive(Clone, Copy)]
struct ShadeVertex {
    p: [f64; 2],
    normal: [f64; 3],
}
fn lighting(normal: [f64; 3], view: [f64; 3], light: [f64; 3], o: SvgOptions) -> [f64; 2] {
    let n = unit(normal).unwrap_or(view);
    [
        0.28 + 0.72 * dot(n, light).max(0.),
        highlight(n, light, view, o.specular, o.shininess),
    ]
}
fn final_color(fields: [f64; 2], o: SvgOptions) -> [f64; 3] {
    o.base_color
        .map(|c| (1. - fields[1]) * fields[0] * c as f64 / 255. + fields[1])
}
// Fit one opaque RGB gradient to the final colors. Any non-collinearity is
// included in the refinement error, rather than delegated to an alpha mask.
fn color_axis(colors: [[f64; 3]; 3]) -> ([f64; 3], [f64; 3], [f64; 3]) {
    let (a, b) = [(0, 1), (1, 2), (2, 0)]
        .into_iter()
        .max_by(|&(a, b), &(c, d)| {
            let x = sub(colors[b], colors[a]);
            let y = sub(colors[d], colors[c]);
            dot(x, x).total_cmp(&dot(y, y))
        })
        .unwrap();
    let base = colors[a];
    let axis = sub(colors[b], base);
    let len = dot(axis, axis);
    let values = colors.map(|c| {
        if len > 1e-24 {
            dot(sub(c, base), axis) / len
        } else {
            0.
        }
    });
    (base, axis, values)
}
fn axis_color(base: [f64; 3], axis: [f64; 3], t: f64) -> [f64; 3] {
    std::array::from_fn(|c| (base[c] + axis[c] * t).clamp(0., 1.))
}
fn shade_mix(t: [ShadeVertex; 3], w: [f64; 3]) -> ShadeVertex {
    ShadeVertex {
        p: std::array::from_fn(|c| (0..3).map(|i| t[i].p[c] * w[i]).sum()),
        normal: std::array::from_fn(|c| (0..3).map(|i| t[i].normal[c] * w[i]).sum()),
    }
}
// Lighting refinement leaves original geometry, visibility and structure lines unchanged.
fn shade_patches(
    t: [ShadeVertex; 3],
    view: [f64; 3],
    light: [f64; 3],
    o: SvgOptions,
    depth: usize,
    rgb_mesh: bool,
    out: &mut Vec<[ShadeVertex; 3]>,
) -> Result<(), DataError> {
    let colors = t.map(|v| final_color(lighting(v.normal, view, light, o), o));
    let (base, axis, values) = color_axis(colors);
    let mut error = 0.0_f64;
    for w in [
        [1., 0., 0.],
        [0., 1., 0.],
        [0., 0., 1.],
        [0.5, 0.5, 0.],
        [0., 0.5, 0.5],
        [0.5, 0., 0.5],
        [1. / 3.; 3],
        [0.6, 0.2, 0.2],
        [0.2, 0.6, 0.2],
        [0.2, 0.2, 0.6],
    ] {
        let exact = final_color(lighting(shade_mix(t, w).normal, view, light, o), o);
        let approx = if rgb_mesh {
            std::array::from_fn(|c| (0..3).map(|i| colors[i][c] * w[i]).sum())
        } else {
            axis_color(base, axis, dot(values, w))
        };
        for c in 0..3 {
            error = error.max((exact[c] - approx[c]).abs());
        }
    }
    if error <= o.shading_tolerance {
        if out.len() >= 65536 {
            return Err(DataError::new(
                "shading",
                "per-triangle patch budget exceeded",
            ));
        }
        out.push(t);
        return Ok(());
    }
    if depth == 12 {
        return Err(DataError::new(
            "shading",
            &format!(
                "sampled lighting error {error} exceeds depth budget; normals {:?}",
                t.map(|v| v.normal)
            ),
        ));
    }
    let a = shade_mix(t, [0.5, 0.5, 0.]);
    let b = shade_mix(t, [0., 0.5, 0.5]);
    let c = shade_mix(t, [0.5, 0., 0.5]);
    // These children have disjoint interiors. Visiting the centre second gives
    // Type 4 a connected edge sequence without changing parent depth order.
    let children = if rgb_mesh {
        [[t[0], a, c], [a, b, c], [a, t[1], b], [c, b, t[2]]]
    } else {
        [[t[0], a, c], [a, t[1], b], [c, b, t[2]], [a, b, c]]
    };
    for child in children {
        shade_patches(child, view, light, o, depth + 1, rgb_mesh, out)?;
    }
    Ok(())
}

pub fn render_svg(mesh: &Mesh, options: SvgOptions) -> Result<String, DataError> {
    render_svg_with_lines(mesh, &[], options)
}

/// Display clipped surface lines, hiding rear chords against this mesh when opaque.
/// Transparent previews show all supplied lines. Chords, like the mesh, are approximate.
pub fn render_svg_with_lines(
    mesh: &Mesh,
    lines: &[crate::SurfaceLine],
    options: SvgOptions,
) -> Result<String, DataError> {
    render_scene(mesh, lines, options, None)
}
pub(crate) fn render_scene(
    mesh: &Mesh,
    lines: &[crate::SurfaceLine],
    options: SvgOptions,
    mut pdf: Option<&mut crate::pdf::Pdf>,
) -> Result<String, DataError> {
    if !options.shading_tolerance.is_finite()
        || options.shading_tolerance <= 0.
        || options.shading_tolerance > 1.
    {
        return Err(DataError::new(
            "shading_tolerance",
            "must be finite and in (0, 1]",
        ));
    }
    for (name, value) in [("specular", options.specular), ("opacity", options.opacity)] {
        if !value.is_finite() || !(0.0..=1.0).contains(&value) {
            return Err(DataError::new(name, "must be finite and between 0 and 1"));
        }
    }
    if !options.shininess.is_finite() || options.shininess <= 0. || options.shininess > 10000. {
        return Err(DataError::new(
            "shininess",
            "must be finite and in (0, 10000]",
        ));
    }
    if options.width < 32
        || options.height < 32
        || options.width > 8192
        || options.height > 8192
        || mesh.triangles.is_empty()
    {
        return Err(DataError::new(
            "svg",
            "requires nonempty mesh and dimensions between 32 and 8192",
        ));
    }
    let view = unit(options.view)?;
    let light = unit(options.light)?;
    let up = if view[2].abs() > 0.99 {
        [0., 1., 0.]
    } else {
        [0., 0., 1.]
    };
    let right = unit(cross(up, view))?;
    let up = cross(view, right);
    let projected: Vec<[f64; 3]> = mesh
        .vertices
        .iter()
        .map(|v| [dot(v.point, right), -dot(v.point, up), dot(v.point, view)])
        .collect();
    if projected.iter().flatten().any(|x| !x.is_finite()) {
        return Err(DataError::new("svg", "nonfinite vertex"));
    }
    let mut bounds = projected.clone();
    for line in lines {
        for &p in &line.points {
            if p.iter().any(|v| !v.is_finite()) {
                return Err(DataError::new("lines", "nonfinite point"));
            }
            bounds.push([dot(p, right), -dot(p, up), dot(p, view)]);
        }
    }
    let lo: [f64; 2] =
        std::array::from_fn(|c| bounds.iter().map(|p| p[c]).fold(f64::INFINITY, f64::min));
    let hi: [f64; 2] = std::array::from_fn(|c| {
        bounds
            .iter()
            .map(|p| p[c])
            .fold(f64::NEG_INFINITY, f64::max)
    });
    let scale = ((options.width as f64 - 32.) / (hi[0] - lo[0]))
        .min((options.height as f64 - 32.) / (hi[1] - lo[1]));
    if !scale.is_finite() || scale <= 0. {
        return Err(DataError::new("svg", "degenerate projected bounds"));
    }
    let screen = |i: usize| {
        [
            (projected[i][0] - (lo[0] + hi[0]) / 2.) * scale + options.width as f64 / 2.,
            (projected[i][1] - (lo[1] + hi[1]) / 2.) * scale + options.height as f64 / 2.,
        ]
    };
    // Average adjacent face normals only where analytic normals are undefined (poles).
    // Group coincident positions across knot rectangles; do not replace valid normals.
    let mut fallback = vec![[0.; 3]; mesh.vertices.len()];
    for t in &mesh.triangles {
        if t.iter().any(|&i| i >= mesh.vertices.len()) {
            return Err(DataError::new("svg", "triangle index out of bounds"));
        }
        let [a, b, c] = t.map(|i| mesh.vertices[i].point);
        let face = cross(sub(b, a), sub(c, a));
        for &i in t {
            for j in 0..3 {
                fallback[i][j] += face[j];
            }
        }
    }
    let normals: Vec<_> = mesh
        .vertices
        .iter()
        .map(|v| {
            v.normal
                .and_then(|n| unit(n).ok())
                .or_else(|| {
                    let mut sum = [0.; 3];
                    for (j, w) in mesh.vertices.iter().enumerate() {
                        if norm(sub(v.point, w.point)) < 1e-12 {
                            for c in 0..3 {
                                sum[c] += fallback[j][c];
                            }
                        }
                    }
                    unit(sum).ok()
                })
                .unwrap_or(view)
        })
        .collect();
    let color = |rgb: [f64; 3]| {
        format!(
            "rgb({:.6}%,{:.6}%,{:.6}%)",
            rgb[0] * 100.,
            rgb[1] * 100.,
            rgb[2] * 100.
        )
    };
    let mut order: Vec<_> = (0..mesh.triangles.len()).collect();
    order.sort_by(|&a, &b| {
        let depth = |i: usize| {
            mesh.triangles[i]
                .iter()
                .map(|&k| projected[k][2])
                .sum::<f64>()
        };
        depth(a).total_cmp(&depth(b))
    });
    let mut svg = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\" viewBox=\"0 0 {} {}\" color-interpolation=\"sRGB\">",
        options.width, options.height, options.width, options.height
    );
    let mut next_id = 0usize;
    for original_id in order {
        if !options.draw_surfaces {
            continue;
        }
        let t = mesh.triangles[original_id];
        let original_p = t.map(screen);
        let mut patches = Vec::new();
        shade_patches(
            t.map(|k| ShadeVertex {
                p: screen(k),
                normal: normals[k],
            }),
            view,
            light,
            options,
            0,
            pdf.is_some(),
            &mut patches,
        )
        .map_err(|e| e.prefix(&format!("triangle[{original_id}]")))?;
        if next_id + patches.len() > 1_000_000 {
            return Err(DataError::new("shading", "render patch budget exceeded"));
        }
        if let Some(pdf) = pdf.as_deref_mut() {
            pdf.begin_parent(original_p);
            for patch in &patches {
                pdf.triangle(
                    patch.map(|v| v.p),
                    patch.map(|v| final_color(lighting(v.normal, view, light, options), options)),
                );
            }
            next_id += patches.len();
            // Non-overlapping projected parents can share one transparency paint.
            if options.wireframe {
                pdf.line(&original_p, 0.5, [55, 85, 106], true);
            }
            continue;
        }
        // Apply opacity once to the complete parent triangle, never per shading patch.
        let parent_id = next_id + 1000000;
        triangle_clip(
            &mut svg,
            parent_id,
            original_p,
            options.opacity == 1. && !options.wireframe,
        );
        write!(
            svg,
            "<g opacity=\"{}\" clip-path=\"url(#c{parent_id})\">",
            options.opacity
        )
        .unwrap();
        for patch in patches {
            let id = next_id;
            next_id += 1;
            let p = patch.map(|v| v.p);
            let (base, axis, i) = color_axis(
                patch.map(|v| final_color(lighting(v.normal, view, light, options), options)),
            );
            let area = (p[1][0] - p[0][0]) * (p[2][1] - p[0][1])
                - (p[1][1] - p[0][1]) * (p[2][0] - p[0][0]);
            if area.abs() < 1e-12 {
                continue;
            }
            // doc:coverage:start
            // Bake diffuse and highlight into a single opaque RGB field.
            // Avoid reader-dependent quantization of a separate highlight soft mask.
            // doc:coverage:end
            let fill = if let Some((a, b, lo, hi)) = gradient(p, i) {
                write!(svg,"<defs><linearGradient id=\"g{id}\" gradientUnits=\"userSpaceOnUse\" x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\"><stop stop-color=\"{}\"/><stop offset=\"1\" stop-color=\"{}\"/></linearGradient></defs>",a[0],a[1],b[0],b[1],color(axis_color(base,axis,lo)),color(axis_color(base,axis,hi))).unwrap();
                format!("url(#g{id})")
            } else {
                color(axis_color(base, axis, (i[0] + i[1] + i[2]) / 3.))
            };
            write!(
                svg,
                "<path d=\"{}\" fill=\"{fill}\"/>",
                triangle_path(p, true)
            )
            .unwrap();
        }
        svg.push_str("</g>");
        let p = original_p;
        if options.wireframe {
            write!(svg,"<path d=\"M {} {} L {} {} L {} {} Z\" fill=\"none\" stroke=\"#37556a\" stroke-width=\"0.5\" opacity=\"{}\"/>",p[0][0],p[0][1],p[1][0],p[1][1],p[2][0],p[2][1],options.opacity).unwrap();
        }
    }
    let project = |p: [f64; 3]| [dot(p, right), -dot(p, up), dot(p, view)];
    let screen_line = |p: [f64; 3]| {
        [
            (p[0] - (lo[0] + hi[0]) / 2.) * scale + options.width as f64 / 2.,
            (p[1] - (lo[1] + hi[1]) / 2.) * scale + options.height as f64 / 2.,
        ]
    };
    // World-depth bias accounts for the geometric distance from surface to mesh.
    let bias = mesh.sampled_error * 2. + (hi[0] - lo[0]).max(hi[1] - lo[1]) * 1e-9;
    for line in lines {
        if line.points.iter().flatten().any(|v| !v.is_finite()) {
            return Err(DataError::new("lines", "nonfinite point"));
        }
        let edit = matches!(
            line.kind,
            crate::LineKind::ControlPoint | crate::LineKind::ControlNet
        );
        if line.kind == crate::LineKind::ControlPoint {
            for &p in &line.points {
                let p = screen_line(project(p));
                let mark = [
                    [p[0] - 2.5, p[1] - 2.5],
                    [p[0] + 2.5, p[1] - 2.5],
                    [p[0] + 2.5, p[1] + 2.5],
                    [p[0] - 2.5, p[1] + 2.5],
                ];
                if let Some(pdf) = pdf.as_deref_mut() {
                    pdf.line(&mark, 1.2, [190, 80, 30], true);
                } else {
                    write!(
                        svg,
                        "<path d=\"{}\" fill=\"none\" stroke=\"#be501e\" stroke-width=\"1.2\" opacity=\"{}\"/>",
                        format!(
                            "M {} {} L {} {} L {} {} L {} {} Z",
                            mark[0][0],
                            mark[0][1],
                            mark[1][0],
                            mark[1][1],
                            mark[2][0],
                            mark[2][1],
                            mark[3][0],
                            mark[3][1]
                        ),
                        options.opacity
                    )
                    .unwrap();
                }
            }
            continue;
        }
        let width = if matches!(
            line.kind,
            crate::LineKind::TrimBoundary | crate::LineKind::Boundary | crate::LineKind::Silhouette
        ) {
            1.5
        } else {
            0.8
        };
        for chord in line.points.windows(2) {
            let a = project(chord[0]);
            let b = project(chord[1]);
            let mut hidden = Vec::new();
            if options.occlude_lines && !edit && options.opacity == 1. {
                for t in &mesh.triangles {
                    if let Some(interval) =
                        crate::lines::hidden_interval(a, b, t.map(|i| projected[i]), bias)
                    {
                        hidden.push(interval);
                    }
                }
            }
            hidden.sort_by(|a, b| a[0].total_cmp(&b[0]));
            let mut visible = Vec::new();
            let mut start = 0.;
            for [lo, hi] in hidden {
                if lo > start {
                    visible.push([start, lo]);
                }
                start = f64::max(start, hi);
            }
            if start < 1. {
                visible.push([start, 1.]);
            }
            for [lo, hi] in visible {
                let p = screen_line(std::array::from_fn(|i| a[i] + lo * (b[i] - a[i])));
                let q = screen_line(std::array::from_fn(|i| a[i] + hi * (b[i] - a[i])));
                if let Some(pdf) = pdf.as_deref_mut() {
                    pdf.line(&[p, q], width, [22, 59, 80], false);
                    continue;
                }
                write!(svg,"<path d=\"M {} {} L {} {}\" fill=\"none\" stroke=\"#163b50\" stroke-width=\"{width}\" opacity=\"{}\"/>",p[0],p[1],q[0],q[1],options.opacity).unwrap();
            }
        }
    }
    svg.push_str("</svg>");
    Ok(svg)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn normal_refinement_reduces_interior_lighting_error() {
        let t = [[-0.6, -0.3, 1.], [0.6, -0.3, 1.], [0., 0.7, 1.]].map(|n| ShadeVertex {
            p: [n[0], n[1]],
            normal: unit(n).unwrap(),
        });
        let front = [0., 0., 1.];
        for specular in [0., 0.8] {
            let o = SvgOptions {
                specular,
                shininess: 48.,
                ..Default::default()
            };
            let error = |tri: [ShadeVertex; 3]| {
                let (base, axis, values) =
                    color_axis(tri.map(|v| final_color(lighting(v.normal, front, front, o), o)));
                let mut max = 0.0_f64;
                for a in 0..=16 {
                    for b in 0..=16 - a {
                        let w = [a as f64 / 16., b as f64 / 16., (16 - a - b) as f64 / 16.];
                        let exact =
                            final_color(lighting(shade_mix(tri, w).normal, front, front, o), o);
                        let approx = axis_color(base, axis, dot(values, w));
                        for c in 0..3 {
                            max = max.max((exact[c] - approx[c]).abs());
                        }
                    }
                }
                max
            };
            assert!(error(t) > 0.01);
            let mut patches = Vec::new();
            shade_patches(t, front, front, o, 0, false, &mut patches).unwrap();
            assert!(patches.len() > 1);
            let refined = patches.into_iter().map(error).fold(0.0_f64, f64::max);
            assert!(refined < 0.0015, "dense sampled error: {refined}");
        }
        let flat = t.map(|v| ShadeVertex { normal: front, ..v });
        let mut patches = Vec::new();
        shade_patches(
            flat,
            front,
            front,
            Default::default(),
            0,
            false,
            &mut patches,
        )
        .unwrap();
        assert_eq!(patches.len(), 1);
    }
    #[test]
    fn pdf_rgb_interpolation_meets_dense_lighting_samples() {
        let t = [[-0.6, -0.3, 1.], [0.6, -0.3, 1.], [0., 0.7, 1.]].map(|n| ShadeVertex {
            p: [n[0], n[1]],
            normal: unit(n).unwrap(),
        });
        let front = [0., 0., 1.];
        let o = SvgOptions {
            specular: 0.8,
            shininess: 48.,
            ..Default::default()
        };
        let mut patches = Vec::new();
        shade_patches(t, front, front, o, 0, true, &mut patches).unwrap();
        let mut worst = 0.0_f64;
        for patch in patches {
            let colors = patch.map(|v| final_color(lighting(v.normal, front, front, o), o));
            for a in 0..=16 {
                for b in 0..=16 - a {
                    let w = [a as f64 / 16., b as f64 / 16., (16 - a - b) as f64 / 16.];
                    let expected =
                        final_color(lighting(shade_mix(patch, w).normal, front, front, o), o);
                    for c in 0..3 {
                        let actual: f64 = (0..3).map(|i| colors[i][c] * w[i]).sum();
                        worst = worst.max((actual - expected[c]).abs());
                    }
                }
            }
        }
        assert!(worst < 0.0015, "{worst}");
    }
    #[test]
    fn gradient_reproduces_vertex_values() {
        let p = [[1., 2.], [5., 3.], [2., 8.]];
        let values = [0.3, 0.8, 0.5];
        let (a, b, lo, hi) = gradient(p, values).unwrap();
        let d = [b[0] - a[0], b[1] - a[1]];
        for k in 0..3 {
            let t =
                ((p[k][0] - a[0]) * d[0] + (p[k][1] - a[1]) * d[1]) / (d[0] * d[0] + d[1] * d[1]);
            assert!((lo + t * (hi - lo) - values[k]).abs() < 1e-12);
        }
        assert!(gradient(p, [0.5; 3]).is_none());
    }

    #[test]
    fn highlight_strength_width_and_backside() {
        let front = [0., 0., 1.];
        assert_eq!(highlight(front, front, front, 0.6, 32.), 0.6);
        assert_eq!(highlight(front, front, front, 0., 32.), 0.);
        let n = unit([0.4, 0., 1.]).unwrap();
        assert!(highlight(n, front, front, 1., 64.) < highlight(n, front, front, 1., 8.));
        assert_eq!(highlight(front, [0., 0., -1.], front, 1., 32.), 0.);
    }

    #[test]
    fn coverage_of_thin_triangles_uses_one_beveled_polygon() {
        let p = [[0., 0.], [100., 0.], [0., 1e-9]];
        let mut opaque = String::new();
        triangle_clip(&mut opaque, 0, p, true);
        assert_eq!(opaque.matches("<path").count(), 1);
        assert_eq!(opaque.matches("L ").count(), 5);
        assert!(!opaque.contains("NaN") && !opaque.contains("inf"));
        let mut transparent = String::new();
        triangle_clip(&mut transparent, 0, p, false);
        assert_eq!(transparent.matches("<path").count(), 1);
        assert!(!transparent.contains("<circle"));
    }
}
