//! Independent tensor-product NURBS surface data, without a CAD kernel.
//!
//! See ../../main.typ for the mathematics, storage conventions and source walkthrough.
//! Validated data, analytic first derivatives and UV-preserving tessellation.
mod axis;
mod brep;
pub mod pure3dm;
pub use brep::{Brep, BrepEdge, BrepFace, DisplayMode, DisplayOptions, DisplayScene};
mod deflate;
mod evaluate;
mod mesh;
mod pdf;
mod svg;
pub use pdf::{render_pdf, render_pdf_with_lines};
pub use svg::{SvgOptions, render_svg, render_svg_with_lines};
mod lines;
mod surface;
mod trim;
pub use lines::{LineKind, SurfaceLine};
pub use trim::TrimRegion;

pub use evaluate::SurfaceSample;
pub use mesh::{Mesh, MeshOptions, MeshVertex};

pub use axis::{KnotAxis, KnotFormat, MAX_DEGREE};
pub use surface::NurbsSurface;

#[cfg(test)]
mod geometry_tests;
#[cfg(test)]
mod tests;

/// Invalid input, identified by a stable field path and an explanatory message.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DataError {
    pub field: String,
    pub message: String,
}

impl DataError {
    pub(crate) fn new(field: &str, message: &str) -> Self {
        Self {
            field: field.into(),
            message: message.into(),
        }
    }

    pub(crate) fn prefix(mut self, prefix: &str) -> Self {
        self.field = format!("{prefix}.{}", self.field);
        self
    }
}

impl std::fmt::Display for DataError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.field, self.message)
    }
}
impl std::error::Error for DataError {}

extern crate self as nurbs_surface;
pub mod demo;
#[path = "../examples/support/mod.rs"]
pub mod demo_surfaces;
#[cfg(target_arch = "wasm32")]
mod plugin;
