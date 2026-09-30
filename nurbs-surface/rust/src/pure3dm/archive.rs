//! Bounded modern 3dm chunks. CRC covers direct bytes, excluding nested chunks.
use super::{Result, error};
pub const ANON: u32 = 0x40008000;

#[cfg(test)]
#[test]
fn nested_crc_excludes_child_headers_and_payloads() {
    fn chunk(data: &[u8]) -> Vec<u8> {
        let mut b = ANON.to_le_bytes().to_vec();
        b.extend(((data.len() + 4) as u64).to_le_bytes());
        b.extend(data);
        b.extend(0xcbf43926u32.to_le_bytes());
        b
    }
    let mut body = b"123456789".to_vec();
    body.extend(chunk(b"123456789"));
    let encoded = chunk(&body);
    let mut root = Reader::new(&encoded);
    let mut parent = root.expect(ANON).unwrap();
    assert_eq!(parent.take(9).unwrap(), b"123456789");
    let mut child = parent.expect(ANON).unwrap();
    child.take(9).unwrap();
    child.finish().unwrap();
    parent.finish().unwrap();
    root.finish().unwrap();
}
pub struct Reader<'a> {
    pub bytes: &'a [u8],
    pos: usize,
    crc: u32,
    expected: Option<u32>,
}
pub struct Chunk<'a> {
    pub kind: u32,
    pub value: u64,
    pub reader: Reader<'a>,
}
impl<'a> Reader<'a> {
    pub fn new(bytes: &'a [u8]) -> Self {
        Self {
            bytes,
            pos: 0,
            crc: !0,
            expected: None,
        }
    }
    pub fn remaining(&self) -> usize {
        self.bytes.len() - self.pos
    }
    fn raw(&mut self, n: usize) -> Result<&'a [u8]> {
        let end = self
            .pos
            .checked_add(n)
            .ok_or_else(|| error("length overflow"))?;
        let b = self
            .bytes
            .get(self.pos..end)
            .ok_or_else(|| error("truncated archive"))?;
        self.pos = end;
        Ok(b)
    }
    pub fn take(&mut self, n: usize) -> Result<&'a [u8]> {
        let b = self.raw(n)?;
        for &v in b {
            self.crc ^= v as u32;
            for _ in 0..8 {
                self.crc = (self.crc >> 1) ^ (0xedb88320u32.wrapping_mul(self.crc & 1));
            }
        }
        Ok(b)
    }
    pub fn u8(&mut self) -> Result<u8> {
        Ok(self.take(1)?[0])
    }
    pub fn i32(&mut self) -> Result<i32> {
        Ok(i32::from_le_bytes(self.take(4)?.try_into().unwrap()))
    }
    pub fn f64(&mut self) -> Result<f64> {
        let x = f64::from_le_bytes(self.take(8)?.try_into().unwrap());
        if !x.is_finite() {
            return Err(error("non-finite number"));
        }
        Ok(x)
    }
    pub fn point<const N: usize>(&mut self) -> Result<[f64; N]> {
        let mut p = [0.; N];
        for x in &mut p {
            *x = self.f64()?;
        }
        Ok(p)
    }
    pub fn count(&mut self, max: usize) -> Result<usize> {
        let n = self.i32()?;
        if n < 0 || n as usize > max || n as usize > self.remaining() {
            return Err(error("invalid count / budget exceeded"));
        }
        Ok(n as usize)
    }
    pub fn indices(&mut self) -> Result<Vec<i32>> {
        let n = self.count(100_000)?;
        (0..n).map(|_| self.i32()).collect()
    }
    pub fn version(&mut self, major: u8, max_minor: u8) -> Result<u8> {
        let v = self.u8()?;
        if v >> 4 != major || v & 15 > max_minor {
            return Err(error(&format!(
                "unsupported record version {}.{}",
                v >> 4,
                v & 15
            )));
        }
        Ok(v & 15)
    }
    pub fn chunk(&mut self) -> Result<Chunk<'a>> {
        let kind = u32::from_le_bytes(self.raw(4)?.try_into().unwrap());
        let value = u64::from_le_bytes(self.raw(8)?.try_into().unwrap());
        if kind & 0x80000000 != 0 {
            return Ok(Chunk {
                kind,
                value,
                reader: Self::new(&[]),
            });
        }
        let n = usize::try_from(value).map_err(|_| error("chunk length overflow"))?;
        let bytes = self.raw(n)?;
        let mut r = Self::new(bytes);
        if kind & 0x8000 != 0 {
            if n < 4 {
                return Err(error("missing CRC"));
            }
            r.expected = Some(u32::from_le_bytes(bytes[n - 4..].try_into().unwrap()));
            r.bytes = &bytes[..n - 4];
        }
        Ok(Chunk {
            kind,
            value,
            reader: r,
        })
    }
    pub fn expect(&mut self, kind: u32) -> Result<Reader<'a>> {
        let c = self.chunk()?;
        if c.kind != kind {
            return Err(error(&format!(
                "expected chunk {kind:08x}, found {:08x}",
                c.kind
            )));
        }
        Ok(c.reader)
    }
    pub fn finish(self) -> Result<()> {
        if self.remaining() != 0 {
            return Err(error(&format!("{} unconsumed bytes", self.remaining())));
        }
        if self.expected.is_some_and(|c| c != !self.crc) {
            return Err(error("CRC32 mismatch"));
        }
        Ok(())
    }
}
pub fn uuid(b: &[u8]) -> String {
    format!(
        "{:08X}-{:04X}-{:04X}-{:02X}{:02X}-{}",
        u32::from_le_bytes(b[0..4].try_into().unwrap()),
        u16::from_le_bytes(b[4..6].try_into().unwrap()),
        u16::from_le_bytes(b[6..8].try_into().unwrap()),
        b[8],
        b[9],
        b[10..16]
            .iter()
            .map(|v| format!("{v:02X}"))
            .collect::<String>()
    )
}
pub fn class<'a>(r: &mut Reader<'a>) -> Result<(String, Reader<'a>)> {
    let mut c = r.expect(0x27ffa)?;
    let mut id = c.expect(0x2fffb)?;
    let name = uuid(id.take(16)?);
    id.finish()?;
    let data = c.expect(0x2fffc)?;
    loop {
        let tail = c.chunk()?;
        if tail.kind == 0x80027fff {
            break;
        }
        if tail.kind != 0x27ffd {
            return Err(error("unexpected class trailer"));
        }
    }
    c.finish()?;
    Ok((name, data))
}
