//! Small deterministic zlib encoder: RFC 1950 + RFC 1951 fixed Huffman block.
//! Single-candidate LZ77 dictionary bounds compression work; no external runtime.
struct Bits {
    data: Vec<u8>,
    pending: u64,
    count: u32,
}
impl Bits {
    fn put(&mut self, value: u32, n: u32) {
        self.pending |= (value as u64) << self.count;
        self.count += n;
        while self.count >= 8 {
            self.data.push(self.pending as u8);
            self.pending >>= 8;
            self.count -= 8;
        }
    }
    fn symbol(&mut self, s: u32) {
        let (code, n) = match s {
            0..=143 => (s + 0x30, 8),
            144..=255 => (s - 144 + 0x190, 9),
            256..=279 => (s - 256, 7),
            _ => (s - 280 + 0xc0, 8),
        };
        self.put(code.reverse_bits() >> (32 - n), n);
    }
}
pub(crate) fn zlib(input: &[u8]) -> Vec<u8> {
    let mut w = Bits {
        data: vec![0x78, 0x01],
        pending: 0,
        count: 0,
    };
    w.put(3, 3);
    let mut table = vec![usize::MAX; 65536];
    let hash = |i: usize| {
        ((input[i] as usize * 251 + input[i + 1] as usize) * 251 + input[i + 2] as usize) & 65535
    };
    let lengths = [
        3, 4, 5, 6, 7, 8, 9, 10, 11, 13, 15, 17, 19, 23, 27, 31, 35, 43, 51, 59, 67, 83, 99, 115,
        131, 163, 195, 227, 258,
    ];
    let lextra = [
        0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 4, 5, 5, 5, 5, 0,
    ];
    let distances = [
        1, 2, 3, 4, 5, 7, 9, 13, 17, 25, 33, 49, 65, 97, 129, 193, 257, 385, 513, 769, 1025, 1537,
        2049, 3073, 4097, 6145, 8193, 12289, 16385, 24577,
    ];
    let dextra = [
        0, 0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10, 10, 11, 11, 12, 12,
        13, 13,
    ];
    let mut i = 0;
    while i < input.len() {
        let mut length = 0;
        let mut distance = 0;
        if i + 2 < input.len() {
            let key = hash(i);
            let prev = table[key];
            table[key] = i;
            if prev != usize::MAX && i - prev <= 32768 {
                while length < 258
                    && i + length < input.len()
                    && input[prev + length] == input[i + length]
                {
                    length += 1;
                }
                distance = i - prev;
            }
        }
        if length >= 3 {
            let li = lengths.iter().rposition(|&n| n <= length).unwrap();
            w.symbol(257 + li as u32);
            w.put((length - lengths[li]) as u32, lextra[li]);
            let di = distances.iter().rposition(|&n| n <= distance).unwrap();
            w.put((di as u32).reverse_bits() >> 27, 5);
            w.put((distance - distances[di]) as u32, dextra[di]);
            for j in i + 1..i + length {
                if j + 2 < input.len() {
                    table[hash(j)] = j;
                }
            }
            i += length;
        } else {
            w.symbol(input[i] as u32);
            i += 1;
        }
    }
    w.symbol(256);
    if w.count > 0 {
        w.data.push(w.pending as u8);
    }
    let (mut a, mut b) = (1u32, 0u32);
    for &v in input {
        a = (a + v as u32) % 65521;
        b = (b + a) % 65521;
    }
    w.data.extend(((b << 16) | a).to_be_bytes());
    w.data
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn standard_empty_stream() {
        assert_eq!(zlib(b""), [0x78, 1, 3, 0, 0, 0, 0, 1]);
    }
    #[test]
    fn repeated_data_compresses() {
        assert!(zlib(&vec![42; 65536]).len() < 1024);
    }
}
