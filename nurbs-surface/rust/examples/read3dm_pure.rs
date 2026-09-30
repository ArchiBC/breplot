use nurbs_surface::{DisplayMode, pure3dm};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().collect();
    let path = args.get(1).ok_or("usage: breplot input.3dm [output.pdf|svg] [shaded|hidden|wire] [isoU] [isoV] [controls 0|1] [mesh tolerance] [curve tolerance]")?;
    if args.len() > 9 {
        return Err("too many arguments".into());
    }
    if std::fs::metadata(path)?.len() > 128 * 1024 * 1024 {
        return Err("file exceeds 128 MiB".into());
    }
    let m = pure3dm::read(&std::fs::read(path)?)?;
    println!(
        "archive={} objects={} skipped={}",
        m.archive_version,
        m.objects.len(),
        m.skipped.len()
    );
    for o in &m.objects {
        let b = &o.brep;
        println!(
            "record={} faces={} edges={} vertices={} loops={} trims={} c2={} c3={} surfaces={}",
            o.record,
            b.faces.len(),
            b.edges.len(),
            b.vertices.len(),
            b.loops.len(),
            b.trims.len(),
            b.curves2.len(),
            b.curves3.len(),
            b.surfaces.len()
        );
    }
    for s in &m.skipped {
        println!("SKIP record={}: {}", s.record, s.reason);
    }
    if let Some(output) = args.get(2) {
        if !output.ends_with(".pdf") && !output.ends_with(".svg") {
            return Err("output extension must be .pdf or .svg".into());
        }
        if std::path::Path::new(output).exists()
            && std::fs::canonicalize(output)? == std::fs::canonicalize(path)?
        {
            return Err("output must differ from input".into());
        }
        let mode = match args.get(3).map(String::as_str).unwrap_or("shaded") {
            "shaded" => DisplayMode::Shaded,
            "hidden" => DisplayMode::HiddenLine,
            "wire" => DisplayMode::Wireframe,
            _ => return Err("unknown display mode".into()),
        };
        let iso_u = args
            .get(4)
            .map(|s| s.parse::<usize>())
            .transpose()?
            .unwrap_or(2);
        let iso_v = args
            .get(5)
            .map(|s| s.parse::<usize>())
            .transpose()?
            .unwrap_or(2);
        let controls = match args.get(6).map(String::as_str).unwrap_or("0") {
            "0" => false,
            "1" => true,
            _ => return Err("controls must be 0 or 1".into()),
        };
        let mesh_tolerance = args
            .get(7)
            .map(|s| s.parse::<f64>())
            .transpose()?
            .unwrap_or(0.15);
        let curve_tolerance = args
            .get(8)
            .map(|s| s.parse::<f64>())
            .transpose()?
            .unwrap_or(0.01);
        let b = m.to_brep(curve_tolerance)?;
        let scene = b.display(
            nurbs_surface::DisplayOptions {
                mode,
                iso_count: [iso_u, iso_v],
                control_points: controls,
                control_net: controls,
                ..Default::default()
            },
            nurbs_surface::MeshOptions {
                tolerance: mesh_tolerance,
                ..Default::default()
            },
            nurbs_surface::SvgOptions {
                view: [0., -1., 0.],
                light: [-3., -4., 7.],
                ..Default::default()
            },
        )?;
        println!(
            "triangles={} lines={}",
            scene.mesh.triangles.len(),
            scene.lines.len()
        );
        if output.ends_with(".pdf") {
            std::fs::write(output, scene.pdf()?)?;
        } else {
            std::fs::write(output, scene.svg()?)?;
        }
    }
    Ok(())
}
