//! Article fixtures for the memory-only API experiment (not a general input codec).
use crate::{DataError, Mesh, MeshOptions, SurfaceLine, SvgOptions, TrimRegion};
pub fn scene(id: &str) -> Result<(Mesh, Vec<SurfaceLine>, SvgOptions), DataError> {
    let mut o = SvgOptions::default();
    let mut name = id;
    if let Some(s) = name.strip_suffix("-mesh") {
        name = s;
        o.wireframe = true;
    }
    match name {
        "sphere-matte" => {
            name = "sphere";
            o.specular = 0.;
        }
        "sphere-glossy" => {
            name = "sphere";
            o.specular = 0.8;
            o.shininess = 48.;
        }
        "sphere-transparent" => {
            name = "sphere";
            o.specular = 0.8;
            o.shininess = 48.;
            o.opacity = 0.35;
        }
        _ => {}
    }
    let surface = match name {
        "plane" => crate::demo_surfaces::plane(),
        "cylinder" => crate::demo_surfaces::cylinder(),
        "sphere" | "sphere-structure" | "trimmed-sphere" => crate::demo_surfaces::sphere(),
        "saddle" | "trimmed-saddle" => crate::demo_surfaces::saddle(),
        _ => return Err(DataError::new("demo", "unknown scene")),
    };
    let region = match name {
        "trimmed-saddle" => Some(TrimRegion::new(
            vec![[0.05, 0.05], [0.95, 0.05], [0.95, 0.95], [0.05, 0.95]],
            vec![
                (0..32)
                    .map(|i| {
                        let a = i as f64 * std::f64::consts::TAU / 32.;
                        [0.5 + 0.23 * a.cos(), 0.5 + 0.23 * a.sin()]
                    })
                    .collect(),
            ],
        )?),
        "trimmed-sphere" => Some(TrimRegion::new(
            vec![[0.15, 0.15], [3.85, 0.15], [3.85, 1.85], [0.15, 1.85]],
            vec![vec![[0.8, 0.7], [1.5, 0.7], [1.5, 1.3], [0.8, 1.3]]],
        )?),
        _ => None,
    };
    let opts = MeshOptions {
        tolerance: if name == "trimmed-saddle" {
            0.015
        } else {
            0.025
        },
        ..Default::default()
    };
    let mesh = if let Some(r) = &region {
        surface.tessellate_trimmed(r, opts)?
    } else {
        surface.tessellate(opts)?
    };
    let (u, v): (&[f64], &[f64]) = match name {
        "trimmed-saddle" => (&[0.15, 0.3, 0.5, 0.7, 0.85], &[0.15, 0.3, 0.5, 0.7, 0.85]),
        "trimmed-sphere" => (&[0.5, 1., 1.5, 2., 2.5, 3., 3.5], &[0.4, 0.7, 1., 1.3, 1.6]),
        "sphere-structure" => (
            &[0., 0.5, 1., 1.5, 2., 2.5, 3., 3.5],
            &[0.3, 0.6, 1., 1.4, 1.7],
        ),
        _ => (&[], &[]),
    };
    let lines = if u.is_empty() {
        vec![]
    } else {
        surface.structure_lines(region.as_ref(), u, v, 16)?
    };
    Ok((mesh, lines, o))
}
pub fn response(id: &str) -> Result<Vec<u8>, DataError> {
    if id.starts_with("brep-") {
        return display_scene(id)?.pdf();
    }
    if id == "stats" {
        let entries = ["plane", "cylinder", "sphere", "saddle"].map(|id| {
            let (m, _, _) = scene(id).unwrap();
            format!(
                "{{\"id\":\"{id}\",\"triangle_count\":{},\"sampled_error\":{}}}",
                m.triangles.len(),
                m.sampled_error
            )
        });
        return Ok(format!("[{}]", entries.join(",")).into_bytes());
    }
    let (m, l, o) = scene(id)?;
    crate::render_pdf_with_lines(&m, &l, o)
}

/// Six independent NURBS faces, with twelve explicitly shared display edges.
pub fn cube_brep() -> crate::Brep {
    let p = [
        [0., 0., 0.],
        [1., 0., 0.],
        [1., 1., 0.],
        [0., 1., 0.],
        [0., 0., 1.],
        [1., 0., 1.],
        [1., 1., 1.],
        [0., 1., 1.],
    ];
    let mut brep = crate::Brep::default();
    let mut edges = std::collections::BTreeMap::new();
    for q in [
        [0, 3, 2, 1],
        [4, 5, 6, 7],
        [0, 1, 5, 4],
        [1, 2, 6, 5],
        [2, 3, 7, 6],
        [3, 0, 4, 7],
    ] {
        let mut ids = vec![];
        for i in 0..4 {
            let (a, b) = (q[i].min(q[(i + 1) % 4]), q[i].max(q[(i + 1) % 4]));
            let id = *edges.entry((a, b)).or_insert_with(|| {
                brep.edges.push(crate::BrepEdge {
                    points: vec![p[a], p[b]],
                });
                brep.edges.len() - 1
            });
            ids.push(id);
        }
        brep.faces.push(crate::BrepFace {
            surface: crate::NurbsSurface::bezier(
                vec![vec![p[q[0]], p[q[3]]], vec![p[q[1]], p[q[2]]]],
                None,
            )
            .unwrap(),
            trim: None,
            reversed: false,
            boundary_edges: ids,
        });
    }
    brep
}
pub fn display_scene(id: &str) -> Result<crate::DisplayScene, DataError> {
    let mut d = crate::DisplayOptions::default();
    let mut b = cube_brep();
    match id {
        "brep-shaded" => {}
        "brep-hidden" => d.mode = crate::DisplayMode::HiddenLine,
        "brep-wire" => d.mode = crate::DisplayMode::Wireframe,
        "brep-controls" => {
            d.control_points = true;
            d.control_net = true;
            d.iso_count = [2, 2];
        }
        "brep-silhouette" => {
            d.silhouettes = true;
            b = crate::Brep {
                faces: vec![crate::BrepFace {
                    surface: crate::demo_surfaces::sphere(),
                    trim: None,
                    reversed: false,
                    boundary_edges: vec![],
                }],
                edges: vec![],
            };
            d.boundaries = false;
            d.structure_lines = false;
        }
        _ => return Err(DataError::new("demo", "unknown display scene")),
    }
    b.display(
        d,
        MeshOptions {
            tolerance: 0.025,
            ..Default::default()
        },
        SvgOptions::default(),
    )
}
