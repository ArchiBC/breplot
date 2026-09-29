#[path = "../examples/support/mod.rs"]
mod support;

#[test]
fn complete_cylinder_radius_seam_and_normals() {
    let s = support::cylinder();
    for i in 0..=80 {
        let p = s.evaluate(i as f64 / 20., 0.37).unwrap();
        assert!((p.point[0].hypot(p.point[1]) - 1.).abs() < 1e-12);
        assert!((p.point[2] + 0.26).abs() < 1e-12);
        let n = p.normal.unwrap();
        assert!((n[0] * p.point[0] + n[1] * p.point[1] - 1.).abs() < 1e-12);
    }
    assert_eq!(
        s.evaluate(0., 0.5).unwrap().point,
        s.evaluate(4., 0.5).unwrap().point
    );
}
#[test]
fn complete_sphere_radius_normals_and_poles() {
    let s = support::sphere();
    for i in 0..=40 {
        for j in 0..=20 {
            let p = s.evaluate(i as f64 / 10., j as f64 / 10.).unwrap();
            assert!((p.point.iter().map(|x| x * x).sum::<f64>() - 1.).abs() < 1e-12);
            if j == 0 || j == 20 {
                assert!(p.normal.is_none(), "pole u={i}, v={j}: {:?}", p);
            } else {
                assert!(
                    (p.normal
                        .unwrap()
                        .iter()
                        .zip(p.point)
                        .map(|(a, b)| a * b)
                        .sum::<f64>()
                        - 1.)
                        .abs()
                        < 1e-11
                );
            }
        }
    }
    for v in [0., 0.3, 1., 1.8, 2.] {
        assert_eq!(
            s.evaluate(0., v).unwrap().point,
            s.evaluate(4., v).unwrap().point
        );
    }
    let mesh = s.mesh_uniform(4, 10000).unwrap();
    assert!(mesh.degenerate_triangles > 0);
    assert!(!mesh.triangles.is_empty());
}
#[test]
fn remaining_cases_are_executable() {
    for s in [support::plane(), support::saddle()] {
        assert!(!s.mesh_uniform(4, 1000).unwrap().triangles.is_empty());
    }
}

#[test]
fn native_svg_has_gradients_and_validates_indices() {
    use nurbs_surface::{SvgOptions, render_svg};
    let mut mesh = support::sphere().mesh_uniform(4, 10000).unwrap();
    let svg = render_svg(&mesh, SvgOptions::default()).unwrap();
    assert!(svg.contains("linearGradient"));
    assert!(!svg.contains("NaN") && !svg.contains("inf"));
    assert!(svg.contains("gradientUnits=\"userSpaceOnUse\""));
    let wire = render_svg(
        &mesh,
        SvgOptions {
            wireframe: true,
            ..Default::default()
        },
    )
    .unwrap();
    assert!(wire.contains("stroke=\"#37556a\""));
    mesh.triangles[0][0] = usize::MAX;
    assert!(render_svg(&mesh, SvgOptions::default()).is_err());
}

#[test]
fn material_parameters_validate_and_transparency_is_per_fragment() {
    use nurbs_surface::{SvgOptions, render_svg};
    let mesh = support::sphere().mesh_uniform(4, 10000).unwrap();
    for value in [f64::NAN, -0.1, 1.1, f64::INFINITY] {
        assert!(
            render_svg(
                &mesh,
                SvgOptions {
                    opacity: value,
                    ..Default::default()
                }
            )
            .is_err()
        );
        assert!(
            render_svg(
                &mesh,
                SvgOptions {
                    specular: value,
                    ..Default::default()
                }
            )
            .is_err()
        );
    }
    for value in [0., -1., f64::NAN, 10001.] {
        assert!(
            render_svg(
                &mesh,
                SvgOptions {
                    shininess: value,
                    ..Default::default()
                }
            )
            .is_err()
        );
    }
    let matte = render_svg(
        &mesh,
        SvgOptions {
            specular: 0.,
            ..Default::default()
        },
    )
    .unwrap();
    assert!(!matte.contains("stop-color=\"white\""));
    let glossy = render_svg(&mesh, SvgOptions::default()).unwrap();
    assert_ne!(
        matte, glossy,
        "baked RGB must retain the specular contribution"
    );
    assert!(!glossy.contains("stop-opacity="));
    let transparent = render_svg(
        &mesh,
        SvgOptions {
            opacity: 0.35,
            ..Default::default()
        },
    )
    .unwrap();
    assert!(
        transparent
            .matches("<g opacity=\"0.35\" clip-path=")
            .count()
            > 1
    );
    for clip in transparent.split("<clipPath id=\"c").skip(1) {
        let id: usize = clip.split('"').next().unwrap().parse().unwrap();
        if id >= 1_000_000 {
            let body = clip.split("</clipPath>").next().unwrap();
            assert!(!body.contains("<circle")); // only parent geometry controls transparency
            assert_eq!(body.matches("<path").count(), 1);
        }
    }
    assert!(!transparent.contains("<g opacity=\"1\" clip-path="));
    assert!(!transparent.contains("stop-opacity="));
    assert_ne!(matte, transparent);
    assert!(
        render_svg(
            &mesh,
            SvgOptions {
                opacity: 0.,
                ..Default::default()
            }
        )
        .is_ok()
    );
}

#[test]
fn diffuse_and_highlight_share_one_coverage_clip() {
    use nurbs_surface::{SvgOptions, render_svg};
    let mesh = support::sphere().mesh_uniform(4, 10000).unwrap();
    let svg = render_svg(
        &mesh,
        SvgOptions {
            specular: 0.8,
            ..Default::default()
        },
    )
    .unwrap();
    assert!(svg.contains("<linearGradient"));
    assert!(!svg.contains("<rect "));
    assert!(!svg.contains("stop-opacity="));
    assert!(svg.contains("fill=\"url(#g"));
    // Only original triangles retain clips; shading patches are directly painted.
    for clip in svg.split("<clipPath id=\"c").skip(1) {
        let id: usize = clip.split('"').next().unwrap().parse().unwrap();
        assert!(id >= 1_000_000);
    }
    assert!(!svg.contains("<circle")); // single convex coverage polygon
}
