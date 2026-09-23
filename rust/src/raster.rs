//! Smooth surface rendering via Maquette's WASM-capable z-buffer rasterizer.
//! The resulting PNG is placed below independently vector-drawn B-Rep edges.

use base64::Engine;
use brepkit_math::vec::Vec3;
#[cfg(target_arch = "wasm32")]
use core::arch::wasm32::*;
use maquette_core::math::Vec3 as MVec3;
use maquette_core::rasterizer::{BlendMode, PixelBuffer, PixelShader, ShadeIn4, ShadeOut4};
#[cfg(not(target_arch = "wasm32"))]
use maquette_core::simd::*;
use std::sync::Once;

use crate::shade::Facet;

struct SmoothShader {
    light: [f32; 3],
    fill: [f32; 3],
    half: [f32; 3],
    specular: f32,
}

impl SmoothShader {
    fn new(light: Vec3, fill: Vec3, viewer: Vec3, specular: f64) -> Self {
        let half = (light + viewer).normalize().unwrap_or(viewer);
        let arr = |v: Vec3| [v.x() as f32, v.y() as f32, v.z() as f32];
        Self {
            light: arr(light),
            fill: arr(fill),
            half: arr(half),
            specular: specular as f32,
        }
    }

    fn color(&self, x: f32, y: f32, z: f32) -> [f32; 4] {
        let inv = (x * x + y * y + z * z).max(1e-12).sqrt().recip();
        let (x, y, z) = (x * inv, y * inv, z * inv);
        let dot = |v: [f32; 3]| (x * v[0] + y * v[1] + z * v[2]).max(0.0);
        let intensity = 0.58 + 0.34 * dot(self.light) + 0.09 * dot(self.fill);
        let highlight = dot(self.half).powi(18) * self.specular;
        [
            0.55 * intensity + highlight,
            0.64 * intensity + highlight,
            0.73 * intensity + highlight,
            1.0,
        ]
    }
}

impl PixelShader for SmoothShader {
    fn shade4(&self, input: ShadeIn4) -> ShadeOut4 {
        let one = f32x4_splat(1.0);
        let zero = f32x4_splat(0.0);
        let norm2 = f32x4_add(
            f32x4_add(
                f32x4_mul(input.n_x, input.n_x),
                f32x4_mul(input.n_y, input.n_y),
            ),
            f32x4_mul(input.n_z, input.n_z),
        );
        let inv = f32x4_div(one, f32x4_sqrt(f32x4_max(norm2, f32x4_splat(1e-12))));
        let x = f32x4_mul(input.n_x, inv);
        let y = f32x4_mul(input.n_y, inv);
        let z = f32x4_mul(input.n_z, inv);
        let dot = |v: [f32; 3]| {
            f32x4_max(
                f32x4_add(
                    f32x4_add(
                        f32x4_mul(x, f32x4_splat(v[0])),
                        f32x4_mul(y, f32x4_splat(v[1])),
                    ),
                    f32x4_mul(z, f32x4_splat(v[2])),
                ),
                zero,
            )
        };
        let intensity = f32x4_add(
            f32x4_splat(0.58),
            f32x4_add(
                f32x4_mul(dot(self.light), f32x4_splat(0.34)),
                f32x4_mul(dot(self.fill), f32x4_splat(0.09)),
            ),
        );
        let h = dot(self.half);
        let h2 = f32x4_mul(h, h);
        let h4 = f32x4_mul(h2, h2);
        let h8 = f32x4_mul(h4, h4);
        let h16 = f32x4_mul(h8, h8);
        let highlight = f32x4_mul(f32x4_mul(h16, h2), f32x4_splat(self.specular));
        let channel = |base: f32| f32x4_add(f32x4_mul(intensity, f32x4_splat(base)), highlight);
        ShadeOut4 {
            r: channel(0.55),
            g: channel(0.64),
            b: channel(0.73),
            a: one,
            keep: i32x4_splat(-1),
        }
    }

    fn shade_scalar(
        &self,
        _pos: MVec3,
        normal: MVec3,
        _uv: [f32; 2],
        _uv1: [f32; 2],
        _uv2: [f32; 2],
        _color: [f32; 4],
        _tangent: [f32; 4],
    ) -> Option<[f32; 4]> {
        Some(self.color(normal.x as f32, normal.y as f32, normal.z as f32))
    }
}

pub fn render_png(
    facets: &[Facet],
    view: [f64; 4],
    light: Vec3,
    fill: Vec3,
    viewer: Vec3,
    specular: f64,
    pixel_width: u32,
) -> Result<(Vec<u8>, u32), String> {
    static COLOR_INIT: Once = Once::new();
    COLOR_INIT.call_once(maquette_core::color::init_color_luts);
    let [x0, y0, width, height] = view;
    let pixel_height = ((pixel_width as f64 * height / width).round() as u32).max(1);
    let scale = pixel_width as f64 / width;
    let mut buffer = PixelBuffer::new(pixel_width as usize, pixel_height as usize, (255, 255, 255));
    let shader = SmoothShader::new(light, fill, viewer, specular);
    let zeros = [[0.0_f32; 2]; 3];
    let colors = [[1.0_f32; 4]; 3];
    let tangents = [[0.0_f32; 4]; 3];
    for facet in facets {
        let screen = facet
            .points
            .map(|p| ((p.x - x0) * scale, (p.y - y0) * scale));
        let depths = [1.0; 3]; // orthographic interpolation
        let zbuf = facet.points.map(|p| p.depth);
        let positions = facet.world.map(|p| MVec3::new(p.x(), p.y(), p.z()));
        let normals = facet.normals.map(|n| MVec3::new(n.x(), n.y(), n.z()));
        buffer.rasterize_triangle_shaded(
            &screen,
            &depths,
            &zbuf,
            &positions,
            &normals,
            &zeros,
            &zeros,
            &zeros,
            &colors,
            &tangents,
            BlendMode::Overwrite,
            &shader,
        );
    }
    let (_, _, rgba) = buffer.to_rgba8_transparent();
    let mut png_bytes = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut png_bytes, pixel_width, pixel_height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder
            .write_header()
            .map_err(|e| format!("PNG header: {e}"))?;
        writer
            .write_image_data(&rgba)
            .map_err(|e| format!("PNG data: {e}"))?;
    }
    Ok((png_bytes, pixel_height))
}

pub fn render_png_base64(
    facets: &[Facet],
    view: [f64; 4],
    light: Vec3,
    fill: Vec3,
    viewer: Vec3,
    specular: f64,
    pixel_width: u32,
) -> Result<(String, u32), String> {
    let (png, height) = render_png(facets, view, light, fill, viewer, specular, pixel_width)?;
    Ok((
        base64::engine::general_purpose::STANDARD.encode(png),
        height,
    ))
}
