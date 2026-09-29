use nurbs_surface::{KnotAxis, KnotFormat, NurbsSurface};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // doc:example:start
    // Quadratic U, linear V: a rational quarter-cylinder control net.
    let u = KnotAxis::new(vec![0., 0., 0., 1., 1., 1.], 3, KnotFormat::Full)?;
    let v = KnotAxis::new(vec![0., 0., 1., 1.], 2, KnotFormat::Full)?;
    let surface = NurbsSurface::new(
        u,
        v,
        vec![
            vec![[1., 0., 0.], [1., 0., 2.]],
            vec![[1., 1., 0.], [1., 1., 2.]],
            vec![[0., 1., 0.], [0., 1., 2.]],
        ],
        Some(vec![vec![1.; 2], vec![0.5_f64.sqrt(); 2], vec![1.; 2]]),
    )?;
    assert_eq!(surface.dimensions(), [3, 2]);
    assert_eq!(surface.domain(), [[0., 1.], [0., 1.]]);
    assert!(surface.is_u_rational() && !surface.is_v_rational());
    assert!(surface.is_bezier());
    // doc:example:end
    println!(
        "dimensions={:?}, degree=({}, {}), domain={:?}, rational=({}, {})",
        surface.dimensions(),
        surface.u().degree(),
        surface.v().degree(),
        surface.domain(),
        surface.is_u_rational(),
        surface.is_v_rational()
    );
    Ok(())
}
