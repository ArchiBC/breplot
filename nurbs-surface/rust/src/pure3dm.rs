//! Pure Rust, bounded 3dm B-Rep reader. No native library, conversion, or cached mesh.
//! Supported archive geometry is intentionally narrower than openNURBS.
mod archive;
mod display;
mod geometry;
mod topology;
use crate::DataError;
use archive::{Reader, class};
pub use geometry::{Curve, NurbsCurve};
pub use topology::{Edge, Face, Loop, RawBrep, Trim, Vertex};
type Result<T> = std::result::Result<T, DataError>;
fn error(s: &str) -> DataError {
    DataError::new("pure3dm", s)
}
#[derive(Debug)]
pub struct Object {
    pub record: usize,
    pub brep: RawBrep,
}
#[derive(Debug)]
pub struct SkippedObject {
    pub record: usize,
    pub reason: String,
}
#[derive(Debug)]
pub struct Model {
    pub archive_version: u32,
    pub objects: Vec<Object>,
    pub skipped: Vec<SkippedObject>,
}
/// Geometry tables only. Metadata, user data and cached meshes are bounded opaque chunks.
/// CRCs are checked for decoded records; this is not a complete archive validator.
pub fn read(bytes: &[u8]) -> Result<Model> {
    if bytes.len() > 128 * 1024 * 1024 || bytes.get(..24) != Some(b"3D Geometry File Format ") {
        return Err(error("invalid header / file budget exceeded"));
    }
    let version = std::str::from_utf8(bytes.get(24..32).ok_or_else(|| error("truncated header"))?)
        .map_err(|_| error("invalid version"))?
        .trim()
        .parse::<u32>()
        .map_err(|_| error("invalid version"))?;
    if !matches!(version, 50 | 60 | 70 | 80) {
        return Err(error("only modern 3dm versions 50..80 are supported"));
    }
    let mut model = Model {
        archive_version: version,
        objects: vec![],
        skipped: vec![],
    };
    let mut r = Reader::new(&bytes[32..]);
    let mut found = false;
    let mut eof = false;
    while r.remaining() > 0 {
        let c = r.chunk()?;
        if c.kind == 0x10000013 {
            if found {
                return Err(error("duplicate object table"));
            }
            found = true;
            let mut table = c.reader;
            let mut index = 0;
            loop {
                let c = table.chunk()?;
                if c.kind == 0xffffffff {
                    break;
                }
                if c.kind != 0x20008070 {
                    return Err(error("invalid object table record"));
                }
                if index >= 4096 {
                    return Err(error("object budget exceeded"));
                }
                let mut rec = c.reader;
                let kind = rec.chunk()?;
                if kind.kind != 0x82000071 {
                    return Err(error("missing object type"));
                }
                let (id, data) = class(&mut rec)?;
                if id == "F06FC243-A32A-4608-9DD8-A7D2C4CE2A36"
                    || id == "60B5DBC5-E660-11D3-BFE4-0010830122F0"
                {
                    match topology::read(data)? {
                        Ok(brep) => model.objects.push(Object {
                            record: index,
                            brep,
                        }),
                        Err(reason) => model.skipped.push(SkippedObject {
                            record: index,
                            reason,
                        }),
                    }
                } else {
                    model.skipped.push(SkippedObject {
                        record: index,
                        reason: format!("unsupported class {id} (type {})", kind.value),
                    });
                }
                loop {
                    let c = rec.chunk()?;
                    if c.kind == 0x8200007f {
                        break;
                    }
                    if !matches!(c.kind, 0x02008072 | 0x02000073) {
                        return Err(error("unexpected object trailer"));
                    }
                }
                rec.finish()?;
                index += 1;
            }
            table.finish()?;
        } else if c.kind == 0x7fff {
            let mut end = c.reader;
            let recorded = u64::from_le_bytes(end.take(8)?.try_into().unwrap());
            end.finish()?;
            if recorded != bytes.len() as u64 {
                return Err(error("EOF file size mismatch"));
            }
            eof = true;
            if r.remaining() != 0 {
                return Err(error("trailing bytes after EOF"));
            }
        }
    }
    if !found || !eof {
        return Err(error("missing object table or EOF"));
    }
    Ok(model)
}
