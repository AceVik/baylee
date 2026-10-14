use super::*;

fn put16(data: &mut [u8], at: usize, v: u16) {
    data[at..at + 2].copy_from_slice(&v.to_le_bytes());
}
fn put32(data: &mut [u8], at: usize, v: u32) {
    data[at..at + 4].copy_from_slice(&v.to_le_bytes());
}

// A synthetic PE with independently laid-out DOS/COFF/optional headers,
// one .rsrc section and the standard type/id/language hierarchy.
fn fixture(plus: bool) -> Vec<u8> {
    let mut file = vec![0; 2048];
    file[..2].copy_from_slice(b"MZ");
    put32(&mut file, 0x3c, 128);
    file[128..132].copy_from_slice(b"PE\0\0");
    put16(&mut file, 134, 1);
    let optional_size = if plus { 240 } else { 224 };
    put16(&mut file, 148, optional_size);
    put16(&mut file, 152, if plus { 0x20b } else { 0x10b });
    let directories = 152 + if plus { 112 } else { 96 };
    put32(&mut file, directories - 4, 16);
    put32(&mut file, directories + 16, 0x1000);
    let section = 152 + usize::from(optional_size);
    put32(&mut file, section + 8, 1536);
    put32(&mut file, section + 12, 0x1000);
    put32(&mut file, section + 16, 1536);
    put32(&mut file, section + 20, 512);
    let data = &mut file[512..];
    put16(data, 14, 2);
    put32(data, 16, 3);
    put32(data, 20, 0x8000_0040);
    put32(data, 24, 14);
    put32(data, 28, 0x8000_0090);
    put16(data, 64 + 14, 7);
    put16(data, 144 + 14, 1);
    put32(data, 160, 1);
    put32(data, 164, 0x8000_0178);
    for i in 0..8_u32 {
        let language = 208 + 24 * usize::try_from(i).unwrap();
        if i < 7 {
            put32(data, 80 + 8 * usize::try_from(i).unwrap(), i + 1);
            put32(
                data,
                84 + 8 * usize::try_from(i).unwrap(),
                0x8000_0000 | u32::try_from(language).unwrap(),
            );
        }
        put16(data, language + 14, 1);
        put32(data, language + 16, 1033);
        put32(data, language + 20, 512 + i * 16);
        let leaf = 512 + usize::try_from(i).unwrap() * 16;
        put32(data, leaf, 0x1000 + 640 + i * 8);
        put32(data, leaf + 4, if i == 7 { 104 } else { 8 });
    }
    let group = 640 + 7 * 8;
    put16(data, group + 2, 1);
    put16(data, group + 4, 7);
    for (i, size) in SIZES.into_iter().enumerate() {
        let at = group + 6 + 14 * i;
        data[at] = u8::try_from(size).unwrap_or(0);
        data[at + 1] = data[at];
        put16(data, at + 4, 1);
        put16(data, at + 6, 32);
        put32(data, at + 8, 8);
        put16(data, at + 12, u16::try_from(i + 1).unwrap());
    }
    file
}

#[test]
fn accepts_both_pe_optional_headers() {
    for plus in [false, true] {
        check(&fixture(plus)).unwrap();
    }
}

#[test]
fn truncated_and_malformed_resources_fail_without_panicking() {
    let source = fixture(true);
    for end in 0..1312 {
        assert!(check(&source[..end]).is_err(), "truncation at {end}");
    }
    for (at, bytes) in [
        (0, b"xx".to_vec()),
        (0x3c, u32::MAX.to_le_bytes().to_vec()),
        (512 + 164, 0xffff_ffff_u32.to_le_bytes().to_vec()),
        (512 + 696 + 4, 6_u16.to_le_bytes().to_vec()),
        (512 + 696 + 6 + 12, 99_u16.to_le_bytes().to_vec()),
        (512 + 696 + 6 + 1, vec![24]),
        (512 + 696 + 6 + 8, 9_u32.to_le_bytes().to_vec()),
    ] {
        let mut broken = source.clone();
        broken[at..at + bytes.len()].copy_from_slice(&bytes);
        assert!(check(&broken).is_err(), "mutation at {at}");
    }
}
