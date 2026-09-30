use crate::*;
#[test]
fn nearly_collapsed_boundary_does_not_invent_a_normal() {
    let s = NurbsSurface::bezier(
        vec![
            vec![[10., 20., 30.], [10., 21., 31.]],
            vec![[10. + 2e-15, 20., 30.], [11., 21., 31.]],
        ],
        None,
    )
    .unwrap();
    assert!(s.evaluate(0.37, 0.).unwrap().normal.is_none());
    assert!(s.evaluate(0.37, 0.2).unwrap().normal.is_some());
}
fn close(a: [f64; 3], b: [f64; 3]) {
    for i in 0..3 {
        assert!((a[i] - b[i]).abs() < 1e-10, "{a:?} != {b:?}");
    }
}
fn plane() -> NurbsSurface {
    NurbsSurface::bezier(
        vec![
            vec![[0., 0., 0.], [0., 3., 0.]],
            vec![[2., 0., 0.], [2., 3., 0.]],
        ],
        None,
    )
    .unwrap()
}
pub(crate) fn cylinder() -> NurbsSurface {
    NurbsSurface::bezier(
        vec![
            vec![[1., 0., 0.], [1., 0., 2.]],
            vec![[1., 1., 0.], [1., 1., 2.]],
            vec![[0., 1., 0.], [0., 1., 2.]],
        ],
        Some(vec![vec![1.; 2], vec![0.5_f64.sqrt(); 2], vec![1.; 2]]),
    )
    .unwrap()
}
#[test]
fn plane_points_derivatives_and_endpoints() {
    for u in [0., 0.25, 1.] {
        for v in [0., 0.7, 1.] {
            let s = plane().evaluate(u, v).unwrap();
            close(s.point, [2. * u, 3. * v, 0.]);
            close(s.du, [2., 0., 0.]);
            close(s.dv, [0., 3., 0.]);
            close(s.normal.unwrap(), [0., 0., 1.]);
        }
    }
}
#[test]
fn rational_cylinder_radius_derivatives_and_orientation() {
    let surface = cylinder();
    for i in 0..=40 {
        let u = i as f64 / 40.;
        let s = surface.evaluate(u, 0.3).unwrap();
        assert!((s.point[0].hypot(s.point[1]) - 1.).abs() < 1e-12);
        close(s.dv, [0., 0., 2.]);
        close(s.normal.unwrap(), [s.point[0], s.point[1], 0.]);
        assert!((s.du[0] * s.point[0] + s.du[1] * s.point[1]).abs() < 1e-12);
        if i > 0 && i < 40 {
            let a = surface.evaluate(u - 1e-6, 0.3).unwrap().point;
            let b = surface.evaluate(u + 1e-6, 0.3).unwrap().point;
            for c in 0..3 {
                assert!(((b[c] - a[c]) / 2e-6 - s.du[c]).abs() < 1e-8);
            }
        }
    }
}
#[test]
fn derivatives_respect_original_parameter_scale() {
    let surface = NurbsSurface::new(
        KnotAxis::new(vec![2., 2., 6., 6.], 2, KnotFormat::Full).unwrap(),
        KnotAxis::new(vec![-1., -1., 1., 1.], 2, KnotFormat::Full).unwrap(),
        vec![
            vec![[0., 0., 0.], [0., 3., 0.]],
            vec![[2., 0., 0.], [2., 3., 0.]],
        ],
        None,
    )
    .unwrap();
    let s = surface.evaluate(4., 0.).unwrap();
    close(s.point, [1., 1.5, 0.]);
    close(s.du, [0.5, 0., 0.]);
    close(s.dv, [0., 1.5, 0.]);
    for bad in [f64::NAN, f64::INFINITY, 1.9, 6.1] {
        assert!(surface.evaluate(bad, 0.).is_err());
    }
}
#[test]
fn knot_discontinuities_are_not_bridged_by_mesh() {
    let u = KnotAxis::new(vec![0., 0., 1., 1., 2., 2.], 4, KnotFormat::Full).unwrap();
    let v = KnotAxis::new(vec![0., 0., 1., 1.], 2, KnotFormat::Full).unwrap();
    let s = NurbsSurface::new(
        u,
        v,
        vec![
            vec![[0., 0., 0.], [0., 1., 0.]],
            vec![[1., 0., 0.], [1., 1., 0.]],
            vec![[3., 0., 0.], [3., 1., 0.]],
            vec![[4., 0., 0.], [4., 1., 0.]],
        ],
        None,
    )
    .unwrap();
    close(s.evaluate(1., 0.).unwrap().point, [3., 0., 0.]);
    let mesh = s.mesh_uniform(1, 100).unwrap();
    assert_eq!(mesh.triangles.len(), 4);
    assert_eq!(mesh.vertices[2].point, [1., 0., 0.]);
    assert_eq!(mesh.vertices[4].point, [3., 0., 0.]);
    for t in mesh.triangles {
        assert!(
            t.iter()
                .all(|&i| mesh.vertices[i].spans == mesh.vertices[t[0]].spans)
        );
    }
}
#[test]
fn mesh_plane_counts_uv_and_winding() {
    let mesh = plane().mesh_uniform(3, 100).unwrap();
    assert_eq!(mesh.vertices.len(), 16);
    assert_eq!(mesh.triangles.len(), 18);
    assert!(mesh.sampled_error < 1e-12);
    for t in mesh.triangles {
        let [a, b, c] = t.map(|i| mesh.vertices[i].uv);
        assert!((b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0]) > 0.);
    }
}
#[test]
fn mesh_refinement_reduces_cylinder_error_and_honors_budgets() {
    let s = cylinder();
    let coarse = s.mesh_uniform(2, 10000).unwrap();
    let fine = s.mesh_uniform(8, 10000).unwrap();
    assert!(fine.sampled_error < coarse.sampled_error / 8.);
    let mesh = s
        .tessellate(MeshOptions {
            tolerance: 0.002,
            ..Default::default()
        })
        .unwrap();
    assert!(mesh.subdivisions > 2 && mesh.sampled_error <= 0.002);
    assert!(s.mesh_uniform(usize::MAX, 100).is_err());
    assert!(s.mesh_uniform(10, 10).is_err());
    assert!(s.mesh_uniform(0, 100).is_err());
    assert!(
        s.tessellate(MeshOptions {
            tolerance: 1e-8,
            max_subdivisions: 2,
            ..Default::default()
        })
        .is_err()
    );
    assert!(
        s.tessellate(MeshOptions {
            tolerance: f64::NAN,
            ..Default::default()
        })
        .is_err()
    );
}
#[test]
fn singular_surface_returns_point_without_inventing_normal() {
    let s = NurbsSurface::bezier(vec![vec![[1., 2., 3.]; 2]; 2], None).unwrap();
    let sample = s.evaluate(0.5, 0.5).unwrap();
    close(sample.point, [1., 2., 3.]);
    assert!(sample.normal.is_none());
    let mesh = s.mesh_uniform(2, 100).unwrap();
    assert!(mesh.triangles.is_empty());
    assert_eq!(mesh.degenerate_triangles, 8);
}
#[test]
fn scaled_weights_preserve_evaluation() {
    let s = cylinder();
    let points = s.control_points().chunks(2).map(|r| r.to_vec()).collect();
    let weights = s
        .weights()
        .chunks(2)
        .map(|r| r.iter().map(|w| w * 1e250).collect())
        .collect();
    let scaled = NurbsSurface::new(s.u().clone(), s.v().clone(), points, Some(weights)).unwrap();
    close(
        s.evaluate(0.3, 0.6).unwrap().point,
        scaled.evaluate(0.3, 0.6).unwrap().point,
    );
    close(
        s.evaluate(0.3, 0.6).unwrap().du,
        scaled.evaluate(0.3, 0.6).unwrap().du,
    );
}

#[test]
fn nonclamped_partition_of_unity_and_linear_reproduction() {
    let u = KnotAxis::new((0..9).map(f64::from).collect(), 5, KnotFormat::Full).unwrap();
    let v = KnotAxis::new(vec![0., 0., 1., 1.], 2, KnotFormat::Full).unwrap();
    // Greville abscissae reproduce the parameter exactly, including domain endpoints.
    let points = (0..5)
        .map(|i| vec![[i as f64 + 2., 0., 1.], [i as f64 + 2., 1., 1.]])
        .collect();
    let s = NurbsSurface::new(u, v, points, None).unwrap();
    for u in [3., 3.2, 4., 4.8, 5.] {
        let sample = s.evaluate(u, 0.4).unwrap();
        close(sample.point, [u, 0.4, 1.]);
        close(sample.du, [1., 0., 0.]);
        close(sample.dv, [0., 1., 0.]);
    }
}

#[test]
fn bilinear_saddle_needs_triangle_refinement_despite_exact_bilinear_fit() {
    let s = NurbsSurface::bezier(
        vec![
            vec![[0., 0., 0.], [0., 1., 0.]],
            vec![[1., 0., 0.], [1., 1., 1.]],
        ],
        None,
    )
    .unwrap();
    close(s.evaluate(0.3, 0.6).unwrap().point, [0.3, 0.6, 0.18]);
    let coarse = s.mesh_uniform(1, 100).unwrap();
    assert!(coarse.sampled_error > 0.2);
    let mesh = s
        .tessellate(MeshOptions {
            tolerance: 0.01,
            ..Default::default()
        })
        .unwrap();
    assert!(mesh.subdivisions > 2);
}

#[test]
fn continuous_knot_boundaries_agree_but_crease_normals_remain_separate() {
    let u = KnotAxis::new(vec![0., 0., 1., 2., 2.], 3, KnotFormat::Full).unwrap();
    let v = KnotAxis::new(vec![0., 0., 1., 1.], 2, KnotFormat::Full).unwrap();
    let s = NurbsSurface::new(
        u,
        v,
        vec![
            vec![[0., 0., 0.], [0., 1., 0.]],
            vec![[1., 0., 0.], [1., 1., 0.]],
            vec![[2., 0., 1.], [2., 1., 1.]],
        ],
        None,
    )
    .unwrap();
    let mesh = s.mesh_uniform(2, 100).unwrap();
    for j in 0..3 {
        close(mesh.vertices[6 + j].point, mesh.vertices[9 + j].point);
        assert_ne!(mesh.vertices[6 + j].normal, mesh.vertices[9 + j].normal);
    }
}
