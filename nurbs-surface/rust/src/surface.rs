use crate::{DataError, KnotAxis, KnotFormat, MAX_DEGREE};

// doc:surface:start
/// Tensor-product surface. Flat storage uses index = u * v_count + v.
/// Geometry only: no camera, material, trim loops, topology or display tolerance.
#[derive(Clone, Debug, PartialEq)]
pub struct NurbsSurface {
    u: KnotAxis,
    v: KnotAxis,
    control_points: Vec<[f64; 3]>,
    weights: Vec<f64>,
}
// doc:surface:end

impl NurbsSurface {
    /// Rows are U, columns are V. Missing weights mean exactly one at every pole.
    pub fn new(
        u: KnotAxis,
        v: KnotAxis,
        control_points: Vec<Vec<[f64; 3]>>,
        weights: Option<Vec<Vec<f64>>>,
    ) -> Result<Self, DataError> {
        let (nu, nv) = (u.pole_count(), v.pole_count());
        if control_points.len() != nu || control_points.iter().any(|row| row.len() != nv) {
            return Err(DataError::new(
                "control_points",
                "expected a rectangular [u_count][v_count] grid matching both axes",
            ));
        }
        if let Some(ref grid) = weights {
            if grid.len() != nu || grid.iter().any(|row| row.len() != nv) {
                return Err(DataError::new("weights", "shape must match control_points"));
            }
        }
        for (i, row) in control_points.iter().enumerate() {
            for (j, point) in row.iter().enumerate() {
                if point.iter().any(|x| !x.is_finite()) {
                    return Err(DataError::new(
                        &format!("control_points[{i}][{j}]"),
                        "coordinates must be finite",
                    ));
                }
                let w = weights.as_ref().map_or(1.0, |grid| grid[i][j]);
                if !w.is_finite() || w <= 0.0 {
                    return Err(DataError::new(
                        &format!("weights[{i}][{j}]"),
                        "must be positive and finite",
                    ));
                }
            }
        }
        let control_points: Vec<_> = control_points.into_iter().flatten().collect();
        let weights = weights
            .map(|grid| grid.into_iter().flatten().collect())
            .unwrap_or_else(|| vec![1.0; control_points.len()]);
        Ok(Self {
            u,
            v,
            control_points,
            weights,
        })
    }

    /// A clamped single-span surface on [0,1]^2; degree follows the grid size.
    /// This creates knots but never inserts or removes control points.
    pub fn bezier(
        control_points: Vec<Vec<[f64; 3]>>,
        weights: Option<Vec<Vec<f64>>>,
    ) -> Result<Self, DataError> {
        let nu = control_points.len();
        let nv = control_points.first().map_or(0, Vec::len);
        if !(2..=MAX_DEGREE + 1).contains(&nu) || !(2..=MAX_DEGREE + 1).contains(&nv) {
            return Err(DataError::new(
                "control_points",
                "Bezier grid dimensions must each be between 2 and 26",
            ));
        }
        let knots = |n| {
            std::iter::repeat_n(0.0, n)
                .chain(std::iter::repeat_n(1.0, n))
                .collect()
        };
        let u = KnotAxis::new(knots(nu), nu, KnotFormat::Full).map_err(|e| e.prefix("u"))?;
        let v = KnotAxis::new(knots(nv), nv, KnotFormat::Full).map_err(|e| e.prefix("v"))?;
        Self::new(u, v, control_points, weights)
    }

    pub fn u(&self) -> &KnotAxis {
        &self.u
    }
    pub fn v(&self) -> &KnotAxis {
        &self.v
    }
    pub fn dimensions(&self) -> [usize; 2] {
        [self.u.pole_count(), self.v.pole_count()]
    }
    pub fn domain(&self) -> [[f64; 2]; 2] {
        [self.u.domain(), self.v.domain()]
    }
    pub fn control_points(&self) -> &[[f64; 3]] {
        &self.control_points
    }
    pub fn weights(&self) -> &[f64] {
        &self.weights
    }

    fn index(&self, u: usize, v: usize) -> Option<usize> {
        (u < self.u.pole_count() && v < self.v.pole_count()).then(|| u * self.v.pole_count() + v)
    }
    pub fn control_point(&self, u: usize, v: usize) -> Option<[f64; 3]> {
        self.index(u, v).map(|i| self.control_points[i])
    }
    pub fn weight(&self, u: usize, v: usize) -> Option<f64> {
        self.index(u, v).map(|i| self.weights[i])
    }

    /// Structural rationality: weights vary along U for at least one V column.
    /// Exact comparison; no tolerance or claim about special geometric cancellation.
    pub fn is_u_rational(&self) -> bool {
        (1..self.u.pole_count())
            .any(|i| (0..self.v.pole_count()).any(|j| self.weight(i, j) != self.weight(0, j)))
    }
    pub fn is_v_rational(&self) -> bool {
        (0..self.u.pole_count())
            .any(|i| (1..self.v.pole_count()).any(|j| self.weight(i, j) != self.weight(i, 0)))
    }
    pub fn is_rational(&self) -> bool {
        self.is_u_rational() || self.is_v_rational()
    }
    pub fn is_bezier(&self) -> bool {
        self.u.is_clamped()
            && self.v.is_clamped()
            && self.u.pole_count() == self.u.degree() + 1
            && self.v.pole_count() == self.v.degree() + 1
    }
}
