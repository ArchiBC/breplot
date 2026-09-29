use crate::DataError;

/// Implementation limit, not a mathematical limit.
pub const MAX_DEGREE: usize = 25;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KnotFormat {
    Full,
    Rhino,
}

// doc:axis:start
/// Complete repeated knot vector. No multiplicity array or implicit cyclic poles.
#[derive(Clone, Debug, PartialEq)]
pub struct KnotAxis {
    knots: Vec<f64>,
    pole_count: usize,
    degree: usize,
}
// doc:axis:end

impl KnotAxis {
    /// Infer degree, then validate; control points are never added or removed.
    pub fn new(
        mut knots: Vec<f64>,
        pole_count: usize,
        format: KnotFormat,
    ) -> Result<Self, DataError> {
        // doc:degree:start
        let degree = match format {
            KnotFormat::Full => knots
                .len()
                .checked_sub(pole_count)
                .and_then(|n| n.checked_sub(1)),
            KnotFormat::Rhino => knots
                .len()
                .checked_add(1)
                .and_then(|n| n.checked_sub(pole_count)),
        }
        .filter(|p| (1..=MAX_DEGREE).contains(p))
        .ok_or_else(|| DataError::new("knots", "lengths must imply a degree between 1 and 25"))?;
        // doc:degree:end
        if pole_count <= degree {
            return Err(DataError::new(
                "pole_count",
                "requires at least degree + 1 control points",
            ));
        }
        if knots.is_empty()
            || knots.iter().any(|k| !k.is_finite())
            || knots.windows(2).any(|w| w[0] > w[1])
        {
            return Err(DataError::new(
                "knots",
                "must be nonempty, finite and nondecreasing",
            ));
        }
        if format == KnotFormat::Rhino {
            knots.insert(0, knots[0]);
            knots.push(*knots.last().unwrap());
        }
        // No run-length storage: count equal adjacent values only during validation.
        let mut run = 1;
        for pair in knots.windows(2) {
            run = if pair[0] == pair[1] { run + 1 } else { 1 };
            if run > degree + 1 {
                return Err(DataError::new(
                    "knots",
                    "a knot may occur at most degree + 1 times",
                ));
            }
        }
        let domain = [knots[degree], knots[pole_count]];
        if domain[0] >= domain[1] || !(domain[1] - domain[0]).is_finite() {
            return Err(DataError::new("domain", "must have positive finite length"));
        }
        Ok(Self {
            knots,
            pole_count,
            degree,
        })
    }

    pub fn degree(&self) -> usize {
        self.degree
    }
    pub fn knots(&self) -> &[f64] {
        &self.knots
    }
    pub fn pole_count(&self) -> usize {
        self.pole_count
    }
    pub fn domain(&self) -> [f64; 2] {
        [self.knots[self.degree], self.knots[self.pole_count]]
    }

    /// Nonzero spans and their indices in the full knot vector, in original UV units.
    pub fn spans(&self) -> impl Iterator<Item = (usize, [f64; 2])> + '_ {
        (self.degree..self.pole_count).filter_map(|i| {
            let interval = [self.knots[i], self.knots[i + 1]];
            (interval[0] < interval[1]).then_some((i, interval))
        })
    }

    pub fn is_clamped(&self) -> bool {
        self.knots[..=self.degree]
            .iter()
            .all(|k| *k == self.domain()[0])
            && self.knots[self.pole_count..]
                .iter()
                .all(|k| *k == self.domain()[1])
    }
}
