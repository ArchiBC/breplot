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
