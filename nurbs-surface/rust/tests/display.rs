use nurbs_surface::*;
#[test]
fn disabled_layers_and_reversed_faces_preserve_geometry() {
    let mut b = demo::cube_brep();
    let d = DisplayOptions {
        structure_lines: false,
        boundaries: false,
        silhouettes: false,
        ..Default::default()
    };
    let a = b
        .display(d, MeshOptions::default(), SvgOptions::default())
        .unwrap();
    assert!(a.lines.is_empty());
    for f in &mut b.faces {
        f.reversed = true;
    }
    let r = b
        .display(d, MeshOptions::default(), SvgOptions::default())
        .unwrap();
    for (a, r) in a.mesh.vertices.iter().zip(&r.mesh.vertices) {
        assert_eq!(a.point, r.point);
        assert_eq!(a.normal.map(|n| n.map(|x| -x)), r.normal);
    }
    for (a, r) in a.mesh.triangles.iter().zip(&r.mesh.triangles) {
        assert_eq!([a[0], a[2], a[1]], *r);
    }
}
#[test]
fn shared_edges_are_drawn_once_and_interior_density_is_independent() {
    let b = demo::cube_brep();
    let d = DisplayOptions {
        structure_lines: false,
        silhouettes: false,
        ..Default::default()
    };
    let s = b
        .display(d, MeshOptions::default(), SvgOptions::default())
        .unwrap();
    assert_eq!(b.faces.len(), 6);
    assert_eq!(s.lines.len(), 12);
    let t = b
        .display(
            DisplayOptions {
                structure_lines: true,
                iso_count: [2, 3],
                ..d
            },
            MeshOptions::default(),
            SvgOptions::default(),
        )
        .unwrap();
    assert_eq!(
        t.lines.iter().filter(|l| l.kind == LineKind::IsoU).count(),
        12
    );
    assert_eq!(
        t.lines.iter().filter(|l| l.kind == LineKind::IsoV).count(),
        18
    );
    assert_eq!(s.mesh.triangles.len(), t.mesh.triangles.len());
}
#[test]
fn display_modes_hide_rear_edges_without_painting_faces() {
    let h = demo::display_scene("brep-hidden").unwrap().svg().unwrap();
    let w = demo::display_scene("brep-wire").unwrap().svg().unwrap();
    assert!(!h.contains("linearGradient"));
    assert!(!w.contains("linearGradient"));
    assert!(h.matches("<path").count() < w.matches("<path").count());
    let p = demo::display_scene("brep-hidden").unwrap().pdf().unwrap();
    assert!(!String::from_utf8_lossy(&p).contains("/ShadingType"));
}
#[test]
fn sphere_contour_and_control_handles_are_separate_layers() {
    let s = demo::display_scene("brep-silhouette").unwrap();
    assert!(!s.lines.is_empty());
    assert!(s.lines.iter().all(|l| l.kind == LineKind::Silhouette));
    let v = s.render.view;
    let n = v.iter().map(|x| x * x).sum::<f64>().sqrt();
    for l in &s.lines {
        for p in &l.points {
            assert!((p.iter().zip(v).map(|(a, b)| a * b).sum::<f64>() / n).abs() < 0.04);
        }
    }
    let c = demo::display_scene("brep-controls").unwrap().svg().unwrap();
    assert!(c.contains("#be501e"));
    assert!(!c.contains("NaN"));
}
#[test]
fn display_rejects_bad_references_density_and_total_budget() {
    let mut b = demo::cube_brep();
    assert!(
        b.display(
            DisplayOptions {
                iso_count: [129, 0],
                ..Default::default()
            },
            MeshOptions::default(),
            SvgOptions::default()
        )
        .is_err()
    );
    assert!(
        b.display(
            DisplayOptions::default(),
            MeshOptions {
                max_vertices: 20,
                ..Default::default()
            },
            SvgOptions::default()
        )
        .is_err()
    );
    b.faces[0].boundary_edges.push(99);
    assert!(
        b.display(
            DisplayOptions::default(),
            MeshOptions::default(),
            SvgOptions::default()
        )
        .is_err()
    );
}
