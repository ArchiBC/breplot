use crate::*;

fn grid(nu: usize, nv: usize) -> Vec<Vec<[f64; 3]>> {
    (0..nu)
        .map(|u| {
            (0..nv)
                .map(|v| [u as f64, v as f64, (u * v) as f64])
                .collect()
        })
        .collect()
}

#[test]
fn asymmetric_bezier_preserves_uv_order_and_domains() {
    let s = NurbsSurface::bezier(grid(4, 3), None).unwrap();
    assert_eq!(s.dimensions(), [4, 3]);
    assert_eq!((s.u().degree(), s.v().degree()), (3, 2));
    assert_eq!(s.domain(), [[0., 1.], [0., 1.]]);
    assert_eq!(s.control_point(2, 1), Some([2., 1., 2.]));
    assert_eq!(s.control_points()[7], [2., 1., 2.]);
    assert_eq!(s.weight(2, 1), Some(1.));
    assert_eq!(s.control_point(0, 3), None);
    assert_eq!(s.control_point(usize::MAX, 0), None);
    assert_eq!(s.weight(4, 0), None);
    assert!(s.is_bezier());
    assert!(!s.is_rational());
}

#[test]
fn full_and_rhino_have_identical_active_data() {
    let full = KnotAxis::new(vec![0., 0., 0., 1., 2., 2., 2.], 4, KnotFormat::Full).unwrap();
    let rhino = KnotAxis::new(vec![0., 0., 1., 2., 2.], 4, KnotFormat::Rhino).unwrap();
    assert_eq!(full, rhino);
    assert_eq!(full.degree(), 2);
    assert_eq!(
        full.spans().collect::<Vec<_>>(),
        vec![(2, [0., 1.]), (3, [1., 2.])]
    );
}

#[test]
fn nonclamped_rhino_preserves_active_spans_without_recovering_outer_knots() {
    let full = KnotAxis::new((0..9).map(f64::from).collect(), 5, KnotFormat::Full).unwrap();
    let rhino = KnotAxis::new((1..8).map(f64::from).collect(), 5, KnotFormat::Rhino).unwrap();
    assert_eq!(full.domain(), [3., 5.]);
    assert_eq!(full.domain(), rhino.domain());
    assert_eq!(
        full.spans().collect::<Vec<_>>(),
        rhino.spans().collect::<Vec<_>>()
    );
    assert_ne!(full.knots(), rhino.knots()); // omitted outer values are not reconstructed
    assert!(!full.is_clamped());
}

#[test]
fn discontinuous_internal_knots_keep_two_separate_spans() {
    let axis = KnotAxis::new(
        vec![0., 0., 0., 1., 1., 1., 2., 2., 2.],
        6,
        KnotFormat::Full,
    )
    .unwrap();
    assert_eq!(
        axis.spans().collect::<Vec<_>>(),
        vec![(2, [0., 1.]), (5, [1., 2.])]
    );
    let v = KnotAxis::new(vec![0., 0., 1., 1.], 2, KnotFormat::Full).unwrap();
    assert!(
        !NurbsSurface::new(axis, v, grid(6, 2), None)
            .unwrap()
            .is_bezier()
    );
}

#[test]
fn rejects_invalid_knots_and_degrees_without_panics() {
    for knots in [
        vec![],
        vec![0., 1.],
        vec![0., 1., 0., 1.],
        vec![0.; 4],
        vec![0., 0., f64::NAN, 1.],
        vec![0., 0., 1., f64::INFINITY],
        vec![-f64::MAX, -f64::MAX, f64::MAX, f64::MAX],
    ] {
        assert!(KnotAxis::new(knots, 2, KnotFormat::Full).is_err());
    }
    assert!(KnotAxis::new(vec![0., 0., 0., 1., 1.], 2, KnotFormat::Full).is_err());
    assert!(KnotAxis::new(vec![0., 1.], usize::MAX, KnotFormat::Full).is_err());
    assert!(KnotAxis::new(vec![0., 1.], usize::MAX, KnotFormat::Rhino).is_err());
    assert!(KnotAxis::new(vec![0., 0., 0., 0., 1., 1., 1.], 4, KnotFormat::Full).is_err());
    assert!(KnotAxis::new(vec![0.; 54], 27, KnotFormat::Full).is_err());
}

#[test]
fn rejects_ragged_points_and_weights() {
    assert!(NurbsSurface::bezier(vec![], None).is_err());
    assert!(NurbsSurface::bezier(grid(1, 4), None).is_err());
    let mut points = grid(4, 3);
    points[2].pop();
    assert_eq!(
        NurbsSurface::bezier(points, None).unwrap_err().field,
        "control_points"
    );
    for weights in [vec![], vec![vec![1.; 2]; 4], vec![vec![1.; 3]; 3]] {
        assert_eq!(
            NurbsSurface::bezier(grid(4, 3), Some(weights))
                .unwrap_err()
                .field,
            "weights"
        );
    }
}

#[test]
fn rejects_nonfinite_coordinates_and_nonpositive_weights_with_location() {
    for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        let mut points = grid(2, 2);
        points[1][0][2] = bad;
        assert_eq!(
            NurbsSurface::bezier(points, None).unwrap_err().field,
            "control_points[1][0]"
        );
    }
    for bad in [0., -1., f64::NAN, f64::INFINITY] {
        let mut weights = vec![vec![1.; 2]; 2];
        weights[0][1] = bad;
        assert_eq!(
            NurbsSurface::bezier(grid(2, 2), Some(weights))
                .unwrap_err()
                .field,
            "weights[0][1]"
        );
    }
}

#[test]
fn rational_directions_and_common_weight_scale() {
    let u = NurbsSurface::bezier(
        grid(3, 2),
        Some(vec![vec![1., 1.], vec![0.5, 0.5], vec![1., 1.]]),
    )
    .unwrap();
    assert!(u.is_u_rational());
    assert!(!u.is_v_rational());
    let v = NurbsSurface::bezier(grid(3, 2), Some(vec![vec![1., 2.]; 3])).unwrap();
    assert!(!v.is_u_rational());
    assert!(v.is_v_rational());
    let constant = NurbsSurface::bezier(grid(3, 2), Some(vec![vec![7.; 2]; 3])).unwrap();
    assert!(!constant.is_rational());
    assert_eq!(constant.weights(), &[7.; 6]); // never normalize user data silently
}

#[test]
fn explicitly_expanded_cyclic_data_is_not_modified_or_certified_periodic() {
    let mut points = grid(4, 2);
    points.push(points[0].clone());
    points.push(points[1].clone());
    let u = KnotAxis::new((0..9).map(f64::from).collect(), 6, KnotFormat::Full).unwrap();
    let v = KnotAxis::new(vec![0., 0., 1., 1.], 2, KnotFormat::Full).unwrap();
    let s = NurbsSurface::new(u, v, points.clone(), None).unwrap();
    assert_eq!(s.domain(), [[2., 6.], [0., 1.]]);
    assert_eq!(s.control_point(4, 1), Some(points[0][1]));
    assert_eq!(s.dimensions(), [6, 2]);
}

#[test]
fn validates_grid_against_each_independent_axis() {
    let u = KnotAxis::new(vec![0., 0., 1., 1.], 2, KnotFormat::Full).unwrap();
    let v = KnotAxis::new(vec![0., 0., 0., 1., 1., 1.], 3, KnotFormat::Full).unwrap();
    assert!(NurbsSurface::new(u, v, grid(3, 2), None).is_err());
}

#[test]
fn positive_tiny_weights_and_degenerate_geometry_are_valid_data() {
    let s = NurbsSurface::bezier(
        vec![vec![[0.; 3]; 2]; 2],
        Some(vec![vec![f64::MIN_POSITIVE; 2]; 2]),
    )
    .unwrap();
    assert!(s.is_bezier()); // regularity and numerical evaluation belong to later stages
}
