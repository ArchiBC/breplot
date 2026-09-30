use super::{
    Curve, Result,
    archive::{ANON, Reader},
    error, geometry,
};
use crate::NurbsSurface;
#[derive(Debug)]
pub struct Vertex {
    pub point: [f64; 3],
    pub edges: Vec<i32>,
    pub tolerance: f64,
}
#[derive(Debug)]
pub struct Edge {
    pub curve: i32,
    pub reversed: bool,
    pub proxy_domain: [f64; 2],
    pub vertices: [i32; 2],
    pub trims: Vec<i32>,
    pub tolerance: f64,
    pub domain: [f64; 2],
}
#[derive(Debug)]
pub struct Trim {
    pub curve: i32,
    pub proxy_domain: [f64; 2],
    pub edge: i32,
    pub vertices: [i32; 2],
    pub rev3d: bool,
    pub kind: i32,
    pub iso: i32,
    pub loop_id: i32,
    pub tolerance: [f64; 2],
    pub domain: [f64; 2],
    pub reversed: bool,
}
#[derive(Debug)]
pub struct Loop {
    pub trims: Vec<i32>,
    pub kind: i32,
    pub face: i32,
}
#[derive(Debug)]
pub struct Face {
    pub loops: Vec<i32>,
    pub surface: i32,
    pub reversed: bool,
    pub material: i32,
}
#[derive(Debug)]
pub struct RawBrep {
    pub curves2: Vec<Option<Curve>>,
    pub curves3: Vec<Option<Curve>>,
    pub surfaces: Vec<Option<NurbsSurface>>,
    pub vertices: Vec<Vertex>,
    pub edges: Vec<Edge>,
    pub trims: Vec<Trim>,
    pub loops: Vec<Loop>,
    pub faces: Vec<Face>,
    pub solid_flag: i32,
}
fn start<'a>(r: &mut Reader<'a>) -> Result<(Reader<'a>, u8, usize)> {
    let mut a = r.expect(ANON)?;
    let minor = a.version(1, 2)?;
    let n = a.count(100_000)?;
    Ok((a, minor, n))
}
fn index(r: &mut Reader, i: usize) -> Result<()> {
    if r.i32()? != i as i32 {
        return Err(error("noncanonical topology index"));
    }
    Ok(())
}
pub fn read(mut r: Reader) -> Result<std::result::Result<RawBrep, String>> {
    let minor = r.version(3, 3)?;
    let mut unsupported = vec![];
    let curves2 = geometry::curves(&mut r, &mut unsupported)?;
    let curves3 = geometry::curves(&mut r, &mut unsupported)?;
    let surfaces = geometry::surfaces(&mut r, &mut unsupported)?;
    let (mut a, _, n) = start(&mut r)?;
    let mut vertices = vec![];
    for i in 0..n {
        index(&mut a, i)?;
        vertices.push(Vertex {
            point: a.point()?,
            edges: a.indices()?,
            tolerance: a.f64()?,
        });
    }
    a.finish()?;
    let (mut a, _, n) = start(&mut r)?;
    let mut edges = vec![];
    for i in 0..n {
        index(&mut a, i)?;
        edges.push(Edge {
            curve: a.i32()?,
            reversed: a.i32()? != 0,
            proxy_domain: a.point()?,
            vertices: [a.i32()?, a.i32()?],
            trims: a.indices()?,
            tolerance: a.f64()?,
            domain: a.point()?,
        });
    }
    a.finish()?;
    let (mut a, _, n) = start(&mut r)?;
    let mut trims = vec![];
    for i in 0..n {
        index(&mut a, i)?;
        let mut t = Trim {
            curve: a.i32()?,
            proxy_domain: a.point()?,
            edge: a.i32()?,
            vertices: [a.i32()?, a.i32()?],
            rev3d: a.i32()? != 0,
            kind: a.i32()?,
            iso: a.i32()?,
            loop_id: a.i32()?,
            tolerance: a.point()?,
            domain: a.point()?,
            reversed: false,
        };
        t.reversed = a.take(8)?[0] == 1;
        a.take(24 + 16)?;
        trims.push(t);
    }
    a.finish()?;
    let (mut a, _, n) = start(&mut r)?;
    let mut loops = vec![];
    for i in 0..n {
        index(&mut a, i)?;
        loops.push(Loop {
            trims: a.indices()?,
            kind: a.i32()?,
            face: a.i32()?,
        });
    }
    a.finish()?;
    let (mut a, m, n) = start(&mut r)?;
    if n > 1024 {
        return Err(error("face budget"));
    }
    let mut faces = vec![];
    for i in 0..n {
        index(&mut a, i)?;
        faces.push(Face {
            loops: a.indices()?,
            surface: a.i32()?,
            reversed: a.i32()? != 0,
            material: a.i32()?,
        });
    }
    if m >= 1 {
        a.take(16 * n)?;
    }
    if m >= 2 && a.u8()? != 0 {
        a.take(4 * n)?;
    }
    a.finish()?;
    r.take(48)?;
    if minor >= 1 {
        r.expect(ANON)?;
        r.expect(ANON)?;
    }
    let solid_flag = if minor >= 2 { r.i32()? } else { 0 };
    if minor >= 3 {
        r.expect(ANON)?;
    }
    r.finish()?;
    if !unsupported.is_empty() {
        unsupported.sort();
        unsupported.dedup();
        return Ok(Err(unsupported.join(", ")));
    }
    let b = RawBrep {
        curves2,
        curves3,
        surfaces,
        vertices,
        edges,
        trims,
        loops,
        faces,
        solid_flag,
    };
    b.validate()?;
    Ok(Ok(b))
}
fn get<T>(v: &[T], i: i32) -> Result<&T> {
    if i < 0 {
        return Err(error("negative topology index"));
    }
    v.get(i as usize)
        .ok_or_else(|| error("topology index out of bounds"))
}
fn domain(d: [f64; 2], c: &Curve) -> Result<()> {
    let a = c.domain();
    if d[0] >= d[1] || d[0] < a[0] || d[1] > a[1] {
        return Err(error("proxy domain outside curve"));
    }
    Ok(())
}
impl RawBrep {
    pub fn validate(&self) -> Result<()> {
        for (i, v) in self.vertices.iter().enumerate() {
            for &e in &v.edges {
                if !get(&self.edges, e)?.vertices.contains(&(i as i32)) {
                    return Err(error("vertex/edge incidence mismatch"));
                }
            }
        }
        for (i, e) in self.edges.iter().enumerate() {
            let c = get(&self.curves3, e.curve)?
                .as_ref()
                .ok_or_else(|| error("null edge curve"))?;
            if c.dimension() != 3 {
                return Err(error("edge curve dimension"));
            }
            domain(e.proxy_domain, c)?;
            if e.domain[0] >= e.domain[1] {
                return Err(error("invalid edge domain"));
            }
            for &v in &e.vertices {
                if !get(&self.vertices, v)?.edges.contains(&(i as i32)) {
                    return Err(error("edge/vertex incidence mismatch"));
                }
            }
            for &t in &e.trims {
                if get(&self.trims, t)?.edge != i as i32 {
                    return Err(error("edge/trim incidence mismatch"));
                }
            }
        }
        for (i, t) in self.trims.iter().enumerate() {
            let c = get(&self.curves2, t.curve)?
                .as_ref()
                .ok_or_else(|| error("null trim curve"))?;
            if c.dimension() != 2 {
                return Err(error("trim curve dimension"));
            }
            domain(t.proxy_domain, c)?;
            if t.domain[0] >= t.domain[1] {
                return Err(error("invalid trim domain"));
            }
            for &v in &t.vertices {
                get(&self.vertices, v)?;
            }
            if t.edge >= 0 {
                if !get(&self.edges, t.edge)?.trims.contains(&(i as i32)) {
                    return Err(error("trim/edge incidence mismatch"));
                }
            } else if t.edge != -1 || t.kind != 4 {
                return Err(error("non-singular trim without edge"));
            }
            if !get(&self.loops, t.loop_id)?.trims.contains(&(i as i32)) {
                return Err(error("trim/loop incidence mismatch"));
            }
        }
        for (i, l) in self.loops.iter().enumerate() {
            if !get(&self.faces, l.face)?.loops.contains(&(i as i32)) {
                return Err(error("loop/face incidence mismatch"));
            }
            for &t in &l.trims {
                if get(&self.trims, t)?.loop_id != i as i32 {
                    return Err(error("loop/trim incidence mismatch"));
                }
            }
        }
        for (i, f) in self.faces.iter().enumerate() {
            get(&self.surfaces, f.surface)?
                .as_ref()
                .ok_or_else(|| error("null face surface"))?;
            for &l in &f.loops {
                if get(&self.loops, l)?.face != i as i32 {
                    return Err(error("face/loop incidence mismatch"));
                }
            }
        }
        Ok(())
    }
}
