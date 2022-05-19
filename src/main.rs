use std::env;
use std::fs::File;
use std::io::{self, BufRead, BufReader, Cursor, Read};

fn main() {
    let mut args = env::args().peekable();
    let me = args.next().unwrap_or_default();

    fn do_stdin() {
        let mut buffer = Vec::new();
        io::stdin().read_to_end(&mut buffer).unwrap();
        println!("{}  -", md5(&mut Cursor::new(&buffer), buffer.len()));
    }

    if args.peek().is_none() {
        do_stdin();
        return;
    }

    for filename in args {
        if filename == "-" {
            do_stdin();
        } else {
            let f = match File::open(&filename) {
                Ok(f) => f,
                Err(e) => {
                    eprintln!("{}: {}: {}", me, filename, e);
                    continue;
                }
            };
            let length = f.metadata().unwrap().len() as usize;
            println!("{}  {}", md5(&mut BufReader::new(f), length), filename);
        }
    }
}

struct State {
    a: u32,
    b: u32,
    c: u32,
    d: u32,
}

// https://en.wikipedia.org/w/index.php?title=MD5&oldid=1085416737#Pseudocode
fn md5<R: BufRead>(message: &mut R, length: usize) -> String {
    let mut state = State {
        a: 0x67452301,
        b: 0xefcdab89,
        c: 0x98badcfe,
        d: 0x10325476,
    };

    let mut chunk = [0; 64];
    let mut bytes_left = length;
    while bytes_left >= 64 {
        message.read_exact(&mut chunk).unwrap();
        state = md5_chunked(state, &chunk);
        bytes_left -= 64;
    }

    let mut chunk = [0; 128];
    message.read_exact(&mut chunk[..bytes_left]).unwrap();
    chunk[bytes_left] = 0x80;

    let length_in_bits = &u64::to_le_bytes((length * 8) as u64);
    if bytes_left <= 64 - 9 {
        chunk[56..64].clone_from_slice(length_in_bits);
    }
    state = md5_chunked(state, chunk[..64].try_into().unwrap());
    if bytes_left > 64 - 9 {
        chunk[120..128].clone_from_slice(length_in_bits);
        state = md5_chunked(state, chunk[64..].try_into().unwrap());
    }

    format!(
        "{:08x}{:08x}{:08x}{:08x}",
        state.a.swap_bytes(),
        state.b.swap_bytes(),
        state.c.swap_bytes(),
        state.d.swap_bytes()
    )
}

fn md5_chunked(state: State, chunk: &[u8; 64]) -> State {
    const S: [u32; 64] = [
        7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 5, 9, 14, 20, 5, 9, 14, 20, 5,
        9, 14, 20, 5, 9, 14, 20, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 6, 10,
        15, 21, 6, 10, 15, 21, 6, 10, 15, 21, 6, 10, 15, 21,
    ];

    const K: [u32; 64] = [
        0xd76aa478, 0xe8c7b756, 0x242070db, 0xc1bdceee, 0xf57c0faf, 0x4787c62a, 0xa8304613,
        0xfd469501, 0x698098d8, 0x8b44f7af, 0xffff5bb1, 0x895cd7be, 0x6b901122, 0xfd987193,
        0xa679438e, 0x49b40821, 0xf61e2562, 0xc040b340, 0x265e5a51, 0xe9b6c7aa, 0xd62f105d,
        0x02441453, 0xd8a1e681, 0xe7d3fbc8, 0x21e1cde6, 0xc33707d6, 0xf4d50d87, 0x455a14ed,
        0xa9e3e905, 0xfcefa3f8, 0x676f02d9, 0x8d2a4c8a, 0xfffa3942, 0x8771f681, 0x6d9d6122,
        0xfde5380c, 0xa4beea44, 0x4bdecfa9, 0xf6bb4b60, 0xbebfbc70, 0x289b7ec6, 0xeaa127fa,
        0xd4ef3085, 0x04881d05, 0xd9d4d039, 0xe6db99e5, 0x1fa27cf8, 0xc4ac5665, 0xf4292244,
        0x432aff97, 0xab9423a7, 0xfc93a039, 0x655b59c3, 0x8f0ccc92, 0xffeff47d, 0x85845dd1,
        0x6fa87e4f, 0xfe2ce6e0, 0xa3014314, 0x4e0811a1, 0xf7537e82, 0xbd3af235, 0x2ad7d2bb,
        0xeb86d391,
    ];

    let mut m = [0; 16];
    for (i, c) in chunk.chunks(4).enumerate() {
        m[i] = u32::from_le_bytes([c[0], c[1], c[2], c[3]]);
    }

    let mut a = state.a;
    let mut b = state.b;
    let mut c = state.c;
    let mut d = state.d;

    for i in 0..=63 {
        let (f, g) = match i {
            0..=15 => ((b & c) | (!b & d), i),
            16..=31 => ((d & b) | (!d & c), (5 * i + 1) % 16),
            32..=47 => (b ^ c ^ d, (3 * i + 5) % 16),
            48..=63 => (c ^ (b | !d), 7 * i % 16),
            _ => panic!(),
        };

        let f = f.wrapping_add(a);
        let f = f.wrapping_add(K[i]);
        let f = f.wrapping_add(m[g]);
        a = d;
        d = c;
        c = b;
        b = b.wrapping_add(f.rotate_left(S[i]));
    }

    State {
        a: state.a.wrapping_add(a),
        b: state.b.wrapping_add(b),
        c: state.c.wrapping_add(c),
        d: state.d.wrapping_add(d),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn rfc1321() {
        let test_cases = [
            ("", "d41d8cd98f00b204e9800998ecf8427e"),
            ("a", "0cc175b9c0f1b6a831c399e269772661"),
            ("abc", "900150983cd24fb0d6963f7d28e17f72"),
            ("message digest", "f96b697d7cb7938d525a2f31aaf161d0"),
            (
                "abcdefghijklmnopqrstuvwxyz",
                "c3fcd3d76192e4007dfb496cca67e13b",
            ),
            (
                "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789",
                "d174ab98d277d9f5a5611c2c9f419d9f",
            ),
            (
                "12345678901234567890123456789012345678901234567890123456789012345678901234567890",
                "57edf4a22be3c955ac49da2e2107b67a",
            ),
        ];
        for (l, r) in test_cases.iter() {
            assert_eq!(md5(&mut Cursor::new(l), l.len()), r.to_string());
        }
    }
}
