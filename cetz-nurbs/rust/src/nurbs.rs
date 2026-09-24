//! Standalone NURBS input and exact homogeneous Bézier span extraction.
//! Typst's native curve has no general rational path command; display conversion is separate.

use brepkit_math::nurbs::basis::basis_funs_into;
use brepkit_math::nurbs::curve::NurbsCurve;
use brepkit_math::vec::Point3;
use serde::Deserialize;

#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum KnotFormat {
    #[default]
    Full,
    Rhino,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NurbsInput {
    #[serde(default)]
    pub degree: Option<usize>,
    #[serde(default)]
    pub knot_format: KnotFormat,
    /// Repeated knots in the selected full or Rhino storage convention.
    pub knots: Vec<f64>,
    /// Each point is [x, y] or [x, y, z].
    pub control_points: Vec<Vec<f64>>,
    /// Orthographic view; defaults to an XY drawing with Y pointing up.
    #[serde(default = "default_direction")]
    pub direction: [f64; 3],
    #[serde(default = "default_up")]
    pub up: [f64; 3],
    /// Omitted means all weights are one.
    #[serde(default)]
    pub weights: Option<Vec<f64>>,
    /// Maximum sampled distance between a rational/high-degree span and SVG cubic.
    #[serde(default = "default_tolerance")]
    pub tolerance: f64,
    #[serde(default)]
    pub style: CurveStyle,
}

fn default_tolerance() -> f64 {
    0.01
}

fn default_direction() -> [f64; 3] {
    [0.0, 0.0, -1.0]
}
fn default_up() -> [f64; 3] {
    [0.0, 1.0, 0.0]
}

#[cfg(test)]
mod input_tests {
    use super::*;

    #[test]
    fn full_and_rhino_infer_degree_without_altering_controls() {
        let mut input: NurbsInput = serde_json::from_str(
            r#"{
            "knots":[0,0.1,0.2,0.3,0.5,0.7,0.8,0.9,1],
            "control_points":[[0,0],[1,2],[2,-1],[3,3],[4,0]],
            "weights":[1,0.8,1.2,1,0.9]
        }"#,
        )
        .unwrap();
        let full = input.curve().unwrap();
        assert_eq!(full.degree(), 3);
        input.knots = input.knots[1..input.knots.len() - 1].to_vec();
        input.knot_format = KnotFormat::Rhino;
        let rhino = input.curve().unwrap();
        assert_eq!(full.control_points(), rhino.control_points());
        assert_eq!(full.weights(), rhino.weights());
        assert_eq!(full.domain(), rhino.domain());
        for i in 0..=40 {
            let u = 0.3 + i as f64 * 0.01;
            assert!((full.evaluate(u) - rhino.evaluate(u)).length() < 1e-12);
        }
        input.degree = Some(2);
        assert!(input.curve().err().unwrap().contains("conflicts"));
    }

    #[test]
    fn core_rejects_constructor_flags_and_unknown_knot_formats() {
        let input = serde_json::json!({"knots":[0,0,1,1], "control_points":[[0,0],[1,1]]});
        for name in ["close", "closed", "periodic"] {
            let mut invalid = input.clone();
            invalid[name] = true.into();
            assert!(serde_json::from_value::<NurbsInput>(invalid).is_err());
        }
        let mut invalid = input;
        invalid["knot_format"] = "unknown".into();
        assert!(serde_json::from_value::<NurbsInput>(invalid).is_err());
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(default)]
pub struct CurveStyle {
    pub stroke: String,
    /// In model units; None chooses 0.25% of the drawing extent.
    pub stroke_width: Option<f64>,
    pub opacity: f64,
    pub dash: Vec<f64>,
    pub show_control_points: bool,
    pub show_control_polygon: bool,
    /// Show projected positive world X/Y/Z axes from the world origin.
    pub show_axes: bool,
    /// World-space length of each positive axis. None uses 25% of model extent.
    pub axis_length: Option<f64>,
    pub show_control_labels: bool,
    pub control_polygon_stroke: String,
    pub control_point_fill: String,
    /// In model units; None chooses 0.6% of the drawing extent.
    pub control_point_radius: Option<f64>,
}

impl Default for CurveStyle {
    fn default() -> Self {
        Self {
            stroke: "#244b75".into(),
            stroke_width: None,
            opacity: 1.0,
            dash: Vec::new(),
            show_control_points: false,
            show_control_polygon: false,
            show_axes: false,
            axis_length: None,
            show_control_labels: false,
            control_polygon_stroke: "#8795a5".into(),
            control_point_fill: "#df7145".into(),
            control_point_radius: None,
        }
    }
}

impl NurbsInput {
    pub fn curve(&self) -> Result<NurbsCurve, String> {
        let count = self.control_points.len();
        let degree = match self.knot_format {
            KnotFormat::Full => self.knots.len().checked_sub(count + 1),
            KnotFormat::Rhino => (self.knots.len() + 1).checked_sub(count),
        }
        .filter(|p| (1..=8).contains(p))
        .ok_or("knots and control_points must imply a degree between 1 and 8")?;
        if self.degree.is_some_and(|explicit| explicit != degree) {
            return Err(format!(
                "degree conflicts with degree {degree} inferred from knots and control_points"
            ));
        }
        if count < degree + 1 {
            return Err("control_points must contain at least degree + 1 points".into());
        }
        let points = self
            .control_points
            .iter()
            .enumerate()
            .map(|(i, p)| match p.as_slice() {
                [x, y] if x.is_finite() && y.is_finite() => Ok(Point3::new(*x, *y, 0.0)),
                [x, y, z] if x.is_finite() && y.is_finite() && z.is_finite() => {
                    Ok(Point3::new(*x, *y, *z))
                }
                _ => Err(format!(
                    "control_points[{i}] must contain 2 or 3 finite numbers"
                )),
            })
            .collect::<Result<Vec<_>, _>>()?;
        let weights = self.weights.clone().unwrap_or_else(|| vec![1.0; count]);
        if weights.len() != count || !weights.iter().all(|w| w.is_finite() && *w > 0.0) {
            return Err("weights must contain one positive finite number per control point".into());
        }
        if !self.tolerance.is_finite() || self.tolerance <= 0.0 {
            return Err("tolerance must be a positive finite model-unit distance".into());
        }
        self.style.validate()?;
        let mut knots = self.knots.clone();
        if self.knot_format == KnotFormat::Rhino {
            // The two omitted outer knots do not affect the active curve,
            // including non-clamped and periodic input. Do not modify controls.
            let first = knots[0];
            let last = *knots.last().unwrap();
            knots.insert(0, first);
            knots.push(last);
        }
        if !knots.iter().all(|v| v.is_finite())
            || knots.windows(2).any(|w| w[0] > w[1])
            || knots[degree] >= knots[count]
        {
            return Err(
                "knots must be finite, nondecreasing, and have a nonempty active domain".into(),
            );
        }
        NurbsCurve::new(degree, knots, points, weights).map_err(|e| format!("NURBS curve: {e}"))
    }
}

impl CurveStyle {
    fn validate(&self) -> Result<(), String> {
        for (name, color) in [
            ("stroke", &self.stroke),
            ("control_polygon_stroke", &self.control_polygon_stroke),
            ("control_point_fill", &self.control_point_fill),
        ] {
            if !valid_hex_color(color) {
                return Err(format!("{name} must be a #RRGGBB color"));
            }
        }
        if !self.opacity.is_finite() || !(0.0..=1.0).contains(&self.opacity) {
            return Err("opacity must be between 0 and 1".into());
        }
        for (name, value) in [
            ("stroke_width", self.stroke_width),
            ("control_point_radius", self.control_point_radius),
            ("axis_length", self.axis_length),
        ] {
            if value.is_some_and(|v| !v.is_finite() || v <= 0.0) {
                return Err(format!("{name} must be positive and finite"));
            }
        }
        if self.dash.iter().any(|v| !v.is_finite() || *v <= 0.0) {
            return Err("dash entries must be positive and finite".into());
        }
        Ok(())
    }
}

fn valid_hex_color(color: &str) -> bool {
    color.len() == 7 && color.starts_with('#') && color[1..].bytes().all(|b| b.is_ascii_hexdigit())
}

/// A polynomial in homogeneous coordinates over one nonzero knot interval.
#[derive(Clone, Debug)]
pub struct BezierSpan {
    pub controls: Vec<[f64; 4]>,
    pub range: (f64, f64),
}

impl BezierSpan {
    pub fn degree(&self) -> usize {
        self.controls.len() - 1
    }

    pub fn is_polynomial(&self) -> bool {
        let w = self.controls[0][3];
        self.controls
            .iter()
            .all(|p| (p[3] - w).abs() <= 1e-11 * w.abs().max(1.0))
    }

    pub fn evaluate(&self, t: f64) -> Point3 {
        let mut points = self.controls.clone();
        for level in (1..points.len()).rev() {
            for i in 0..level {
                for c in 0..4 {
                    points[i][c] = points[i][c] * (1.0 - t) + points[i + 1][c] * t;
                }
            }
        }
        let h = points[0];
        Point3::new(h[0] / h[3], h[1] / h[3], h[2] / h[3])
    }

    pub fn split(&self) -> (Self, Self) {
        let p = self.degree();
        let mut row = self.controls.clone();
        let mut left = Vec::with_capacity(p + 1);
        let mut right = Vec::with_capacity(p + 1);
        left.push(row[0]);
        right.push(row[p]);
        for level in (1..=p).rev() {
            for i in 0..level {
                for c in 0..4 {
                    row[i][c] = (row[i][c] + row[i + 1][c]) * 0.5;
                }
            }
            left.push(row[0]);
            right.push(row[level - 1]);
        }
        right.reverse();
        let mid = (self.range.0 + self.range.1) * 0.5;
        (
            Self {
                controls: left,
                range: (self.range.0, mid),
            },
            Self {
                controls: right,
                range: (mid, self.range.1),
            },
        )
    }
}

/// Extract each active NURBS knot interval as an exact homogeneous Bézier span.
/// This also works for non-clamped knot vectors: the basis is evaluated with
/// the interval's fixed span index, then converted to Bernstein coefficients.
pub fn extract_spans(curve: &NurbsCurve) -> Result<Vec<BezierSpan>, String> {
    let p = curve.degree();
    let knots = curve.knots();
    let n = curve.control_points().len();
    let mut output = Vec::new();
    for span in p..n {
        let a = knots[span];
        let b = knots[span + 1];
        if b <= a {
            continue;
        }
        let mut values = Vec::with_capacity(p + 1);
        let mut matrix = Vec::with_capacity(p + 1);
        for sample in 0..=p {
            let t = sample as f64 / p as f64;
            let u = a + (b - a) * t;
            let mut basis = vec![0.0; p + 1];
            basis_funs_into(span, u, p, knots, &mut basis);
            let mut h = [0.0; 4];
            for (j, value) in basis.iter().enumerate() {
                let index = span - p + j;
                let point = curve.control_points()[index];
                let weight = curve.weights()[index];
                for (c, coordinate) in [point.x(), point.y(), point.z(), 1.0].iter().enumerate() {
                    h[c] += value * weight * coordinate;
                }
            }
            values.push(h);
            matrix.push((0..=p).map(|j| bernstein(p, j, t)).collect::<Vec<_>>());
        }
        let controls = solve_bernstein(matrix, values)?;
        if controls
            .iter()
            .any(|h| !h.iter().all(|v| v.is_finite()) || h[3] <= 0.0)
        {
            return Err("Bézier extraction produced an invalid homogeneous point".into());
        }
        output.push(BezierSpan {
            controls,
            range: (a, b),
        });
    }
    if output.is_empty() {
        return Err("NURBS curve has no nonzero knot span".into());
    }
    Ok(output)
}

fn bernstein(p: usize, j: usize, t: f64) -> f64 {
    let mut choose = 1.0;
    for k in 0..j {
        choose *= (p - k) as f64 / (k + 1) as f64;
    }
    choose * t.powi(j as i32) * (1.0 - t).powi((p - j) as i32)
}

fn solve_bernstein(mut a: Vec<Vec<f64>>, mut rhs: Vec<[f64; 4]>) -> Result<Vec<[f64; 4]>, String> {
    let n = rhs.len();
    for col in 0..n {
        let pivot = (col..n)
            .max_by(|&x, &y| a[x][col].abs().total_cmp(&a[y][col].abs()))
            .unwrap();
        if a[pivot][col].abs() < 1e-14 {
            return Err("singular Bernstein conversion".into());
        }
        a.swap(col, pivot);
        rhs.swap(col, pivot);
        let inv = 1.0 / a[col][col];
        for j in col..n {
            a[col][j] *= inv;
        }
        for c in 0..4 {
            rhs[col][c] *= inv;
        }
        for row in 0..n {
            if row == col {
                continue;
            }
            let factor = a[row][col];
            for j in col..n {
                a[row][j] -= factor * a[col][j];
            }
            for c in 0..4 {
                rhs[row][c] -= factor * rhs[col][c];
            }
        }
    }
    Ok(rhs)
}
