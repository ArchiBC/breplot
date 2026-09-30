use nurbs_surface::{DisplayOptions, MeshOptions, SvgOptions, pure3dm};
const LOGO: &[u8] = include_bytes!("../../3dm/fixtures/logo.3dm");
#[test]
fn official_logo_census_topology_and_display_regions() {
    let m = pure3dm::read(LOGO).unwrap();
    assert_eq!(m.archive_version, 70);
    assert!(m.skipped.is_empty());
    let counts: Vec<_> = m
        .objects
        .iter()
        .map(|o| (o.brep.faces.len(), o.brep.edges.len(), o.brep.trims.len()))
        .collect();
    assert_eq!(
        counts,
        vec![
            (4, 6, 12),
            (3, 6, 16),
            (2, 2, 5),
            (6, 12, 28),
            (4, 6, 12),
            (3, 3, 6)
        ]
    );
    let b = m.to_brep(0.01).unwrap();
    assert_eq!(b.faces.len(), 22);
    assert_eq!(b.edges.len(), 35);
    let mut incidences = vec![0usize; 35];
    for f in &b.faces {
        assert!(f.trim.is_some());
        for &e in &f.boundary_edges {
            incidences[e] += 1;
        }
    }
    assert!(incidences.iter().all(|&n| n >= 1 && n <= 2));
    assert!(m.to_brep(f64::NAN).is_err());
}
#[test]
fn decoded_corruption_and_truncated_chunks_are_errors() {
    for n in [0, 24, 31, 32, 100, LOGO.len() / 2, LOGO.len() - 1] {
        assert!(
            pure3dm::read(&LOGO[..n]).is_err(),
            "accepted truncated length {n}"
        );
    }
    // Mutate a class UUID without touching its CRC. This must not turn damage
    // into a misleading unsupported-object report.
    let needle = [0xfb, 0xff, 0x02, 0x00, 0x14, 0, 0, 0, 0, 0, 0, 0];
    let offset = LOGO
        .windows(needle.len())
        .position(|w| w == needle)
        .unwrap();
    // The first class may be metadata, which is intentionally opaque. Locate
    // the first TL_Brep UUID instead.
    let id = [
        0x43, 0xc2, 0x6f, 0xf0, 0x2a, 0xa3, 0x08, 0x46, 0x9d, 0xd8, 0xa7, 0xd2, 0xc4, 0xce, 0x2a,
        0x36,
    ];
    let i = LOGO.windows(16).position(|w| w == id).unwrap();
    assert!(i > offset);
    let mut bad = LOGO.to_vec();
    bad[i] ^= 1;
    assert!(pure3dm::read(&bad).unwrap_err().message.contains("CRC"));
    bad = LOGO.to_vec();
    bad[36..44].copy_from_slice(&u64::MAX.to_le_bytes());
    assert!(pure3dm::read(&bad).is_err());
}
#[test]
fn unsupported_object_is_reported_and_never_converted() {
    let bytes = include_bytes!("../../3dm/fixtures/unsupported-point.3dm");
    let m = pure3dm::read(bytes).unwrap();
    assert!(m.objects.is_empty());
    assert_eq!(m.skipped.len(), 1);
    assert!(m.skipped[0].reason.contains("unsupported class"));
    // Sphere fixture stores a RevSurface: respecting the scope means skipping
    // the entire object, without silently converting it to NURBS.
    let m = pure3dm::read(include_bytes!("../../3dm/fixtures/sphere.3dm")).unwrap();
    assert!(m.objects.is_empty());
    assert_eq!(m.skipped.len(), 1);
}
#[test]
fn invalid_topology_is_rejected_before_display() {
    let mut m = pure3dm::read(LOGO).unwrap();
    m.objects[0].brep.faces[0].surface = i32::MAX;
    assert!(m.to_brep(0.01).is_err());
    let mut m = pure3dm::read(LOGO).unwrap();
    m.objects[0].brep.edges[0].proxy_domain = [9., -9.];
    assert!(m.to_brep(0.01).is_err());
}
#[test]
fn logo_end_to_end_native_pdf_mesh_without_dll() {
    let m = pure3dm::read(LOGO).unwrap();
    let b = m.to_brep(0.01).unwrap();
    let scene = b
        .display(
            DisplayOptions {
                iso_count: [0, 0],
                silhouettes: false,
                ..Default::default()
            },
            MeshOptions {
                tolerance: 0.15,
                ..Default::default()
            },
            SvgOptions {
                view: [0., -1., 0.],
                light: [-3., -4., 7.],
                ..Default::default()
            },
        )
        .unwrap();
    assert!(scene.mesh.triangles.len() > 1000);
    assert!(scene.mesh.sampled_error <= 0.15);
    let pdf = scene.pdf().unwrap();
    assert!(pdf.starts_with(b"%PDF-1.7"));
    assert!(pdf.windows(14).any(|w| w == b"/ShadingType 4"));
}
