use nurbs_surface::{KnotAxis, KnotFormat, NurbsSurface};

fn circle() -> ([[f64; 2]; 9], [f64; 9]) {
    let a = 0.5_f64.sqrt();
    (
        [
            [1., 0.],
            [1., 1.],
            [0., 1.],
            [-1., 1.],
            [-1., 0.],
            [-1., -1.],
            [0., -1.],
            [1., -1.],
            [1., 0.],
        ],
        [1., a, 1., a, 1., a, 1., a, 1.],
    )
}
fn circle_axis() -> KnotAxis {
    KnotAxis::new(
        vec![0., 0., 0., 1., 1., 2., 2., 3., 3., 4., 4., 4.],
        9,
        KnotFormat::Full,
    )
    .unwrap()
}
pub fn cylinder() -> NurbsSurface {
    let (xy, weights) = circle();
    NurbsSurface::new(
        circle_axis(),
        KnotAxis::new(vec![0., 0., 1., 1.], 2, KnotFormat::Full).unwrap(),
        xy.iter()
            .map(|p| vec![[p[0], p[1], -1.], [p[0], p[1], 1.]])
            .collect(),
        Some(weights.iter().map(|w| vec![*w; 2]).collect()),
    )
    .unwrap()
}

// doc:sphere:start
pub fn sphere() -> NurbsSurface {
    let (xy, wu) = circle();
    // Meridian (radius, z), from south pole to north pole: two rational quarters.
    let rz = [[0., -1.], [1., -1.], [1., 0.], [1., 1.], [0., 1.]];
    let wv = [1., 0.5_f64.sqrt(), 1., 0.5_f64.sqrt(), 1.];
    let points = xy
        .iter()
        .map(|p| {
            rz.iter()
                .map(|m| [m[0] * p[0], m[0] * p[1], m[1]])
                .collect()
        })
        .collect();
    let weights = wu
        .iter()
        .map(|u| wv.iter().map(|v| u * v).collect())
        .collect();
    NurbsSurface::new(
        circle_axis(),
        KnotAxis::new(vec![0., 0., 0., 1., 1., 2., 2., 2.], 5, KnotFormat::Full).unwrap(),
        points,
        Some(weights),
    )
    .unwrap()
}
// doc:sphere:end

pub fn plane() -> NurbsSurface {
    NurbsSurface::bezier(
        vec![
            vec![[-1., -1., 0.], [-1., 1., 0.]],
            vec![[1., -1., 0.], [1., 1., 0.]],
        ],
        None,
    )
    .unwrap()
}
pub fn saddle() -> NurbsSurface {
    NurbsSurface::bezier(
        vec![
            vec![[-1., -1., 0.7], [-1., 1., -0.7]],
            vec![[1., -1., -0.7], [1., 1., 0.7]],
        ],
        None,
    )
    .unwrap()
}
