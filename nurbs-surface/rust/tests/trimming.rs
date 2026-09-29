#[path = "../examples/support/mod.rs"]
#[allow(dead_code)]
mod support;
use nurbs_surface::{LineKind, MeshOptions, SvgOptions, TrimRegion, render_svg_with_lines};
fn outer() -> Vec<[f64; 2]> {
    vec![[0., 0.], [1., 0.], [1., 1.], [0., 1.]]
}
fn hole() -> Vec<[f64; 2]> {
    vec![[0.4, 0.4], [0.6, 0.4], [0.6, 0.6], [0.4, 0.6]]
}
fn uv_area(mesh: &nurbs_surface::Mesh) -> f64 {
    mesh.triangles
        .iter()
        .map(|t| {
            let [a, b, c] = t.map(|i| mesh.vertices[i].uv);
            ((b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])) / 2.
        })
        .sum()
}
#[test]
fn hole_crossing_one_coarse_cell_is_preserved() {
    let r = TrimRegion::new(outer(), vec![hole()]).unwrap();
    let mesh = support::plane().mesh_trimmed(&r, 1, 10000).unwrap();
    assert!((uv_area(&mesh) - 0.96).abs() < 1e-10);
    for t in &mesh.triangles {
        let uv =
            std::array::from_fn(|c| t.iter().map(|&i| mesh.vertices[i].uv[c]).sum::<f64>() / 3.);
        assert!(r.contains(uv));
    }
    assert!(!r.contains([0.5, 0.5]));
    assert!(r.contains([0.4, 0.5]));
}
#[test]
fn concave_outline_and_multiple_holes_match_area() {
    let outline = vec![
        [0., 0.],
        [1., 0.],
        [1., 0.4],
        [0.4, 0.4],
        [0.4, 1.],
        [0., 1.],
    ];
    let holes = vec![
        vec![[0.1, 0.1], [0.2, 0.1], [0.2, 0.2], [0.1, 0.2]],
        vec![[0.1, 0.6], [0.2, 0.6], [0.2, 0.7], [0.1, 0.7]],
    ];
    let r = TrimRegion::new(outline, holes).unwrap();
    let m = support::plane().mesh_trimmed(&r, 3, 10000).unwrap();
    assert!((uv_area(&m) - 0.62).abs() < 1e-9);
}
#[test]
fn structure_lines_stop_at_hole_and_keep_semantics() {
    let r = TrimRegion::new(outer(), vec![hole()]).unwrap();
    let s = support::plane();
    let lines = s.structure_lines(Some(&r), &[0.5], &[0.5], 8).unwrap();
    assert_eq!(lines.iter().filter(|l| l.kind == LineKind::IsoU).count(), 2);
    assert_eq!(lines.iter().filter(|l| l.kind == LineKind::IsoV).count(), 2);
    assert_eq!(
        lines
            .iter()
            .filter(|l| l.kind == LineKind::TrimBoundary)
            .count(),
        8
    );
    for l in &lines {
        for &uv in &l.uv {
            assert!(r.contains(uv));
        }
    }
    let mesh = s.mesh_trimmed(&r, 2, 10000).unwrap();
    let svg = render_svg_with_lines(&mesh, &lines, SvgOptions::default()).unwrap();
    assert!(svg.contains("stroke-width=\"1.5\""));
}
#[test]
fn rejects_invalid_rings_domain_and_budgets() {
    assert!(TrimRegion::new(vec![[0., 0.], [1., 1.], [0., 1.], [1., 0.]], vec![]).is_err());
    assert!(TrimRegion::new(outer(), vec![vec![[0., 0.2], [0.2, 0.2], [0.2, 0.4]]]).is_err());
    assert!(TrimRegion::new(outer(), vec![hole(), hole()]).is_err());
    assert!(TrimRegion::new(vec![[f64::NAN, 0.], [1., 0.], [1., 1.]], vec![]).is_err());
    let r = TrimRegion::new(outer(), vec![hole()]).unwrap();
    assert!(support::plane().mesh_trimmed(&r, 1, 4).is_err());
    assert!(
        support::plane()
            .structure_lines(Some(&r), &[-1.], &[], 8)
            .is_err()
    );
    assert!(
        support::plane()
            .structure_lines(None, &[0.5], &[], 0)
            .is_err()
    );
    let outside = TrimRegion::new(vec![[-1., 0.], [1., 0.], [1., 1.]], vec![]).unwrap();
    assert!(support::plane().mesh_trimmed(&outside, 1, 100).is_err());
}
#[test]
fn winding_does_not_change_result_and_refinement_checks_new_triangles() {
    let mut a = outer();
    a.reverse();
    let mut h = hole();
    h.reverse();
    let r = TrimRegion::new(a, vec![h]).unwrap();
    let mesh = support::saddle()
        .tessellate_trimmed(
            &r,
            MeshOptions {
                tolerance: 0.015,
                ..Default::default()
            },
        )
        .unwrap();
    assert!(mesh.sampled_error <= 0.015);
    assert!((uv_area(&mesh) - 0.96).abs() < 1e-9);
}

#[test]
fn concave_hole_and_closed_loop_input() {
    let mut outline = outer();
    outline.push(outline[0]);
    let hole = vec![
        [0.2, 0.2],
        [0.8, 0.2],
        [0.8, 0.4],
        [0.4, 0.4],
        [0.4, 0.8],
        [0.2, 0.8],
    ];
    let region = TrimRegion::new(outline, vec![hole]).unwrap();
    let mesh = support::plane().mesh_trimmed(&region, 2, 10000).unwrap();
    assert!((uv_area(&mesh) - 0.8).abs() < 1e-9);
}

#[test]
fn isocurves_split_at_disconnected_knots() {
    use nurbs_surface::{KnotAxis, KnotFormat, NurbsSurface};
    let u = KnotAxis::new(vec![0., 0., 1., 1., 2., 2.], 4, KnotFormat::Full).unwrap();
    let v = KnotAxis::new(vec![0., 0., 1., 1.], 2, KnotFormat::Full).unwrap();
    let points = [0., 1., 3., 4.]
        .iter()
        .map(|&x| vec![[x, 0., 0.], [x, 1., 0.]])
        .collect();
    let surface = NurbsSurface::new(u, v, points, None).unwrap();
    let lines = surface.structure_lines(None, &[], &[0.5], 8).unwrap();
    assert_eq!(lines.len(), 2);
    assert_eq!(lines[0].points.last().unwrap()[0], 1.);
    assert_eq!(lines[1].points[0][0], 3.);
}
