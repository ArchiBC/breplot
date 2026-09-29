//! Export world-space meshes and native SVG Gouraud previews.
mod support;
use nurbs_surface::{MeshOptions, SvgOptions, TrimRegion, render_svg, render_svg_with_lines};
use std::fmt::Write;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 2 {
        return Err("usage: mesh <output.json>".into());
    }
    // doc:mesh-example:start
    let cases = [
        support::plane(),
        support::cylinder(),
        support::sphere(),
        support::saddle(),
    ];
    let options = MeshOptions {
        tolerance: 0.025,
        ..Default::default()
    };
    let meshes = cases
        .iter()
        .map(|surface| surface.tessellate(options))
        .collect::<Result<Vec<_>, _>>()?;
    // doc:mesh-example:end
    let mut json = String::from("[");
    for (i, mesh) in meshes.iter().enumerate() {
        if i > 0 {
            json.push(',');
        }
        let id = ["plane", "cylinder", "sphere", "saddle"][i];
        let folder = std::path::Path::new(&args[1])
            .parent()
            .unwrap_or(std::path::Path::new("."));
        for wireframe in [false, true] {
            let suffix = if wireframe { "-mesh" } else { "" };
            let svg = render_svg(
                mesh,
                SvgOptions {
                    wireframe,
                    ..Default::default()
                },
            )?;
            std::fs::write(folder.join(format!("{id}{suffix}.svg")), svg)?;
        }
        if id == "sphere" {
            // doc:material:start
            for (name, specular, shininess, opacity) in [
                ("matte", 0.0, 32.0, 1.0),
                ("glossy", 0.8, 48.0, 1.0),
                ("transparent", 0.8, 48.0, 0.35),
            ] {
                let svg = render_svg(
                    mesh,
                    SvgOptions {
                        specular,
                        shininess,
                        opacity,
                        ..Default::default()
                    },
                )?;
                std::fs::write(folder.join(format!("sphere-{name}.svg")), svg)?;
            }
            // doc:material:end
        }
        write!(
            json,
            "{{\"id\":\"{id}\",\"subdivisions\":{},\"sampled_error\":{},\"degenerate\":{},\"vertices\":[",
            mesh.subdivisions, mesh.sampled_error, mesh.degenerate_triangles
        )?;
        for (j, vertex) in mesh.vertices.iter().enumerate() {
            if j > 0 {
                json.push(',');
            }
            write!(
                json,
                "{{\"point\":{:?},\"uv\":{:?},\"normal\":",
                vertex.point, vertex.uv
            )?;
            match vertex.normal {
                Some(n) => write!(json, "{n:?}")?,
                None => json.push_str("null"),
            };
            json.push('}');
        }
        json.push_str("],\"triangles\":[");
        for (j, triangle) in mesh.triangles.iter().enumerate() {
            if j > 0 {
                json.push(',');
            }
            write!(json, "{triangle:?}")?;
        }
        json.push_str("]}");
        println!(
            "{id}: n={}, vertices={}, triangles={}, error={:.8}, degenerate={}",
            mesh.subdivisions,
            mesh.vertices.len(),
            mesh.triangles.len(),
            mesh.sampled_error,
            mesh.degenerate_triangles
        );
    }
    json.push(']');
    std::fs::write(&args[1], json)?;
    let folder = std::path::Path::new(&args[1]).parent().unwrap();
    // doc:trim-example:start
    let outer = vec![[0.05, 0.05], [0.95, 0.05], [0.95, 0.95], [0.05, 0.95]];
    let hole = (0..32)
        .map(|i| {
            let angle = i as f64 * std::f64::consts::TAU / 32.;
            [0.5 + 0.23 * angle.cos(), 0.5 + 0.23 * angle.sin()]
        })
        .collect();
    let region = TrimRegion::new(outer, vec![hole])?;
    let surface = support::saddle();
    let mesh = surface.tessellate_trimmed(
        &region,
        MeshOptions {
            tolerance: 0.015,
            ..Default::default()
        },
    )?;
    let values = [0.15, 0.3, 0.5, 0.7, 0.85];
    let lines = surface.structure_lines(Some(&region), &values, &values, 16)?;
    let svg = render_svg_with_lines(&mesh, &lines, SvgOptions::default())?;
    // doc:trim-example:end
    std::fs::write(folder.join("trimmed-saddle.svg"), svg)?;
    let sphere = support::sphere();
    let region = TrimRegion::new(
        vec![[0.15, 0.15], [3.85, 0.15], [3.85, 1.85], [0.15, 1.85]],
        vec![vec![[0.8, 0.7], [1.5, 0.7], [1.5, 1.3], [0.8, 1.3]]],
    )?;
    let mesh = sphere.tessellate_trimmed(
        &region,
        MeshOptions {
            tolerance: 0.025,
            ..Default::default()
        },
    )?;
    let lines = sphere.structure_lines(
        Some(&region),
        &[0.5, 1., 1.5, 2., 2.5, 3., 3.5],
        &[0.4, 0.7, 1., 1.3, 1.6],
        16,
    )?;
    std::fs::write(
        folder.join("trimmed-sphere.svg"),
        render_svg_with_lines(&mesh, &lines, SvgOptions::default())?,
    )?;
    let mesh = sphere.tessellate(MeshOptions {
        tolerance: 0.025,
        ..Default::default()
    })?;
    let lines = sphere.structure_lines(
        None,
        &[0., 0.5, 1., 1.5, 2., 2.5, 3., 3.5],
        &[0.3, 0.6, 1., 1.4, 1.7],
        16,
    )?;
    std::fs::write(
        folder.join("sphere-structure.svg"),
        render_svg_with_lines(&mesh, &lines, SvgOptions::default())?,
    )?;
    Ok(())
}
