//! Minimal in-memory PDF 1.7 Type 4 writer. No files and no external dependencies.
use crate::{DataError, Mesh, SurfaceLine, SvgOptions};
use std::fmt::Write;

pub fn render_pdf(mesh: &Mesh, options: SvgOptions) -> Result<Vec<u8>, DataError> {
    render_pdf_with_lines(mesh, &[], options)
}
pub fn render_pdf_with_lines(
    mesh: &Mesh,
    lines: &[SurfaceLine],
    options: SvgOptions,
) -> Result<Vec<u8>, DataError> {
    let mut pdf = Pdf::new(options);
    crate::svg::render_scene(mesh, lines, options, Some(&mut pdf))?;
    Ok(pdf.finish())
}
pub(crate) struct Pdf {
    options: SvgOptions,
    objects: Vec<Vec<u8>>,
    mesh: Vec<u8>,
    content: String,
    resources: String,
    count: usize,
    forms: String,
    previous: Option<[[u8; 14]; 3]>,
    coverage: Vec<[[f64; 2]; 3]>,
}
impl Pdf {
    fn new(options: SvgOptions) -> Self {
        Self {
            options,
            objects: vec![vec![], vec![], vec![]],
            mesh: vec![],
            content: String::new(),
            resources: String::new(),
            count: 0,
            forms: String::new(),
            previous: None,
            coverage: Vec::new(),
        }
    }
    pub(crate) fn triangle(&mut self, p: [[f64; 2]; 3], c: [[f64; 3]; 3]) {
        let vertices: [[u8; 14]; 3] = std::array::from_fn(|i| {
            let mut record = [0; 14];
            for (k, v) in [
                p[i][0] / self.options.width as f64,
                1. - p[i][1] / self.options.height as f64,
            ]
            .into_iter()
            .enumerate()
            {
                record[k * 4..k * 4 + 4].copy_from_slice(
                    &((v.clamp(0., 1.) * u32::MAX as f64).round() as u32).to_be_bytes(),
                );
            }
            for (k, v) in c[i].into_iter().enumerate() {
                record[8 + k * 2..10 + k * 2]
                    .copy_from_slice(&((v.clamp(0., 1.) * 65535.).round() as u16).to_be_bytes());
            }
            record
        });
        if let Some(prev) = self.previous {
            for (flag, a, b) in [(1, prev[1], prev[2]), (2, prev[0], prev[2])] {
                if a != b && vertices.contains(&a) && vertices.contains(&b) {
                    if let Some(&c) = vertices.iter().find(|&&v| v != a && v != b) {
                        self.mesh.push(flag);
                        self.mesh.extend(c);
                        self.previous = Some([a, b, c]);
                        return;
                    }
                }
            }
        }
        for v in vertices {
            self.mesh.push(0);
            self.mesh.extend(v);
        }
        self.previous = Some(vertices);
    }
    // Consecutive triangles may share one shading only when their projected
    // interiors do not overlap. Never reorder painter-depth operations.
    pub(crate) fn begin_parent(&mut self, p: [[f64; 2]; 3]) {
        if self.options.opacity < 1. {
            if self.coverage.iter().any(|&q| interiors_overlap(p, q)) {
                self.flush();
            }
            self.coverage.push(p);
        }
    }
    pub(crate) fn flush(&mut self) {
        if self.mesh.is_empty() {
            return;
        }
        self.count += 1;
        let attrs = format!(
            "/ShadingType 4 /ColorSpace /DeviceRGB /BitsPerCoordinate 32 /BitsPerComponent 16 /BitsPerFlag 8 /AntiAlias true /Decode [0 {} 0 {} 0 1 0 1 0 1]",
            self.options.width, self.options.height
        );
        let id = self.objects.len() + 1;
        self.objects.push(stream(&self.mesh, &attrs));
        self.mesh.clear();
        self.previous = None;
        self.coverage.clear();
        if self.options.opacity < 1. {
            // Render a parent's microtriangles opaque in an isolated group, then
            // apply material opacity once when painting the group.
            let form_id = self.objects.len() + 1;
            let attrs = format!(
                "/Type /XObject /Subtype /Form /BBox [0 0 {} {}] /Group << /S /Transparency /I true /CS /DeviceRGB >> /Resources << /Shading << /S {id} 0 R >> /ExtGState << /O << /ca 1 /CA 1 >> >> >>",
                self.options.width, self.options.height
            );
            self.objects.push(stream(b"/O gs /S sh", &attrs));
            write!(self.forms, "/F{} {form_id} 0 R ", self.count).unwrap();
            write!(self.content, "q /A gs /F{} Do Q\n", self.count).unwrap();
        } else {
            write!(self.resources, "/S{} {id} 0 R ", self.count).unwrap();
            write!(self.content, "q /S{} sh Q\n", self.count).unwrap();
        }
    }
    pub(crate) fn line(&mut self, p: &[[f64; 2]], width: f64, color: [u8; 3], close: bool) {
        self.flush();
        write!(
            self.content,
            "q /A gs {} {} {} RG {width} w ",
            color[0] as f64 / 255.,
            color[1] as f64 / 255.,
            color[2] as f64 / 255.
        )
        .unwrap();
        for (i, v) in p.iter().enumerate() {
            write!(
                self.content,
                "{:.6} {:.6} {} ",
                v[0],
                self.options.height as f64 - v[1],
                if i == 0 { "m" } else { "l" }
            )
            .unwrap();
        }
        self.content
            .push_str(if close { "h S Q\n" } else { "S Q\n" });
    }
    fn finish(mut self) -> Vec<u8> {
        self.flush();
        let content_id = self.objects.len() + 1;
        self.objects.push(stream(self.content.as_bytes(), ""));
        self.objects[0] = b"<< /Type /Catalog /Pages 2 0 R >>".to_vec();
        self.objects[1] = b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec();
        self.objects[2]=format!("<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {} {}] /Resources << /Shading << {} >> /XObject << {} >> /ExtGState << /A << /Type /ExtGState /ca {} /CA {} >> >> >> /Contents {content_id} 0 R >>",self.options.width,self.options.height,self.resources,self.forms,self.options.opacity,self.options.opacity).into_bytes();
        let mut bytes = b"%PDF-1.7\n".to_vec();
        let mut offsets = vec![0];
        for (i, obj) in self.objects.iter().enumerate() {
            offsets.push(bytes.len());
            bytes.extend(format!("{} 0 obj\n", i + 1).as_bytes());
            bytes.extend(obj);
            bytes.extend(b"\nendobj\n");
        }
        let xref = bytes.len();
        bytes.extend(format!("xref\n0 {}\n0000000000 65535 f \n", offsets.len()).as_bytes());
        for offset in &offsets[1..] {
            bytes.extend(format!("{offset:010} 00000 n \n").as_bytes());
        }
        bytes.extend(
            format!(
                "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF",
                offsets.len()
            )
            .as_bytes(),
        );
        bytes
    }
}
// Separating-axis test excludes touching edges. Any positive computed overlap
// forces a separate paint; no geometric tolerance hides overlap.
fn interiors_overlap(a: [[f64; 2]; 3], b: [[f64; 2]; 3]) -> bool {
    for t in [a, b] {
        for i in 0..3 {
            let d = [t[(i + 1) % 3][0] - t[i][0], t[(i + 1) % 3][1] - t[i][1]];
            let axis = [-d[1], d[0]];
            if axis == [0., 0.] {
                continue;
            }
            let range = |p: [[f64; 2]; 3]| {
                let v = p.map(|q| q[0] * axis[0] + q[1] * axis[1]);
                (
                    v.into_iter().fold(f64::INFINITY, f64::min),
                    v.into_iter().fold(f64::NEG_INFINITY, f64::max),
                )
            };
            let (amin, amax) = range(a);
            let (bmin, bmax) = range(b);
            if amax <= bmin || bmax <= amin {
                return false;
            }
        }
    }
    true
}
fn stream(data: &[u8], attrs: &str) -> Vec<u8> {
    let packed = crate::deflate::zlib(data);
    let (body, filter) = if packed.len() < data.len() {
        (packed.as_slice(), "/Filter /FlateDecode")
    } else {
        (data, "")
    };
    let mut bytes = format!("<< /Length {} {filter} {attrs} >>\nstream\n", body.len()).into_bytes();
    bytes.extend(body);
    bytes.extend(b"\nendstream");
    bytes
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn xref_offsets_and_type4_stream_are_valid() {
        let (m, l, o) = crate::demo::scene("plane").unwrap();
        let pdf = render_pdf_with_lines(&m, &l, o).unwrap();
        let text = String::from_utf8_lossy(&pdf);
        assert!(text.contains("/ShadingType 4"));
        assert!(!text.contains("/Subtype /Image"));
        let xref = text
            .rsplit("startxref\n")
            .next()
            .unwrap()
            .split('\n')
            .next()
            .unwrap()
            .parse::<usize>()
            .unwrap();
        assert_eq!(&pdf[xref..xref + 4], b"xref");
        let table = std::str::from_utf8(&pdf[xref..]).unwrap();
        for (i, line) in table
            .lines()
            .skip(3)
            .take_while(|l| l.ends_with("n "))
            .enumerate()
        {
            let offset = line[..10].parse::<usize>().unwrap();
            let marker = format!("{} 0 obj", i + 1);
            assert!(pdf[offset..].starts_with(marker.as_bytes()));
        }
    }
    #[test]
    fn alpha_uses_isolated_parent_groups_and_lines_stay_vector() {
        let (m, l, mut o) = crate::demo::scene("plane").unwrap();
        o.opacity = 0.35;
        o.wireframe = true;
        let bytes = render_pdf_with_lines(&m, &l, o).unwrap();
        let text = String::from_utf8_lossy(&bytes);
        assert!(text.contains("/S /Transparency /I true"));
        assert!(text.contains("/ca 0.35"));
        let mut writer = Pdf::new(o);
        writer.line(&[[0., 0.], [1., 0.], [0., 1.]], 0.5, [0, 0, 0], true);
        assert!(writer.content.contains("h S Q"));
        assert!(text.contains("/O gs /S sh"));
    }
    #[test]
    fn edge_flags_decode_to_the_same_ordered_triangles() {
        let mut writer = Pdf::new(SvgOptions::default());
        let a = [0., 0.];
        let b = [10., 0.];
        let c = [0., 10.];
        let d = [10., 10.];
        let e = [20., 10.];
        let color = [[0.2, 0.3, 0.4]; 3];
        for t in [[a, b, c], [b, c, d], [b, d, e]] {
            writer.triangle(t, color);
        }
        let records: Vec<_> = writer.mesh.chunks_exact(15).collect();
        assert_eq!(records.len(), 5);
        assert_eq!([records[0][0], records[3][0], records[4][0]], [0, 1, 2]);
        let decode = |r: &[u8]| {
            let x = u32::from_be_bytes(r[1..5].try_into().unwrap()) as f64 / u32::MAX as f64 * 600.;
            let y = 480.
                - u32::from_be_bytes(r[5..9].try_into().unwrap()) as f64 / u32::MAX as f64 * 480.;
            [x, y]
        };
        let mut previous = [decode(records[0]), decode(records[1]), decode(records[2])];
        for (r, expected) in records[3..].iter().zip([[b, c, d], [b, d, e]]) {
            previous = if r[0] == 1 {
                [previous[1], previous[2], decode(r)]
            } else {
                [previous[0], previous[2], decode(r)]
            };
            for i in 0..3 {
                for k in 0..2 {
                    assert!((previous[i][k] - expected[i][k]).abs() < 1e-6);
                }
            }
        }
        writer.flush();
        assert!(writer.previous.is_none());
    }
    #[test]
    fn touching_triangles_share_a_paint_but_overlapping_layers_do_not() {
        let a = [[0., 0.], [1., 0.], [0., 1.]];
        let b = [[1., 0.], [1., 1.], [0., 1.]];
        assert!(!interiors_overlap(a, b));
        assert!(interiors_overlap(a, a));
        let mut writer = Pdf::new(SvgOptions {
            opacity: 0.35,
            ..Default::default()
        });
        writer.begin_parent(a);
        writer.triangle(a, [[0.5; 3]; 3]);
        writer.begin_parent(b);
        writer.triangle(b, [[0.5; 3]; 3]);
        assert_eq!(writer.count, 0);
        writer.begin_parent(a);
        assert_eq!(writer.count, 1);
    }
    #[test]
    fn invalid_options_and_scene_ids_are_reported() {
        let (m, _, mut o) = crate::demo::scene("plane").unwrap();
        o.opacity = f64::NAN;
        assert!(render_pdf(&m, o).is_err());
        assert!(crate::demo::response("unknown").is_err());
    }
}
