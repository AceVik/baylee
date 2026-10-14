//! Bounded, non-executing reader for the save's nested Java string streams.
use std::collections::BTreeMap;

use anyhow::{Context, Result, bail, ensure};

const MAGIC: &[u8] = b"\xac\xed\x00\x05";
const BASE_HANDLE: u32 = 0x7e_0000;
const MAX_ELEMENTS: usize = 1_000_000;

#[derive(Clone)]
enum Handle {
    Descriptor,
    Array,
    Text(String),
}

struct Stream<'a> {
    data: &'a [u8],
    pos: usize,
    handles: Vec<Handle>,
}

impl<'a> Stream<'a> {
    fn new(data: &'a [u8]) -> Result<Self> {
        ensure!(
            data.starts_with(MAGIC),
            "value is not a Java serialization stream"
        );
        Ok(Self {
            data,
            pos: 4,
            handles: Vec::new(),
        })
    }

    fn take(&mut self, count: usize) -> Result<&'a [u8]> {
        let end = self
            .pos
            .checked_add(count)
            .context("Java length overflow")?;
        let bytes = self
            .data
            .get(self.pos..end)
            .context("truncated Java stream")?;
        self.pos = end;
        Ok(bytes)
    }

    fn byte(&mut self) -> Result<u8> {
        Ok(self.take(1)?[0])
    }
    fn u16(&mut self) -> Result<u16> {
        Ok(u16::from_be_bytes(self.take(2)?.try_into()?))
    }
    fn u32(&mut self) -> Result<u32> {
        Ok(u32::from_be_bytes(self.take(4)?.try_into()?))
    }
    fn u64(&mut self) -> Result<u64> {
        Ok(u64::from_be_bytes(self.take(8)?.try_into()?))
    }

    fn utf(&mut self, long: bool) -> Result<String> {
        let count = if long {
            self.u64()?
        } else {
            u64::from(self.u16()?)
        };
        modified_utf8(self.take(usize::try_from(count)?)?)
    }

    fn reference(&mut self) -> Result<Handle> {
        let id = self
            .u32()?
            .checked_sub(BASE_HANDLE)
            .context("invalid Java handle")?;
        self.handles
            .get(id as usize)
            .cloned()
            .context("unknown Java back-reference")
    }

    fn element(&mut self) -> Result<Option<String>> {
        match self.byte()? {
            0x70 => Ok(None),
            tag @ (0x74 | 0x7c) => {
                let text = self.utf(tag == 0x7c)?;
                self.handles.push(Handle::Text(text.clone()));
                Ok(Some(text))
            }
            0x71 => match self.reference()? {
                Handle::Text(text) => Ok(Some(text)),
                _ => bail!("back-reference is not a string"),
            },
            _ => bail!("unexpected Java array element tag"),
        }
    }
}

// Java encodes UTF-16 code units (including surrogate pairs), not UTF-8.
fn modified_utf8(bytes: &[u8]) -> Result<String> {
    let mut units = Vec::with_capacity(bytes.len());
    let mut pos = 0;
    while pos < bytes.len() {
        let first = bytes[pos];
        let (unit, count) = match first {
            1..=0x7f => (u16::from(first), 1),
            0xc0..=0xdf => {
                let second = *bytes.get(pos + 1).context("truncated modified UTF-8")?;
                ensure!(second & 0xc0 == 0x80, "invalid modified UTF-8 continuation");
                let value = (u16::from(first & 0x1f) << 6) | u16::from(second & 0x3f);
                ensure!(
                    value >= 0x80 || (first == 0xc0 && second == 0x80),
                    "overlong modified UTF-8"
                );
                (value, 2)
            }
            0xe0..=0xef => {
                let tail = bytes
                    .get(pos + 1..pos + 3)
                    .context("truncated modified UTF-8")?;
                ensure!(
                    tail.iter().all(|b| b & 0xc0 == 0x80),
                    "invalid modified UTF-8 continuation"
                );
                let value = (u16::from(first & 0x0f) << 12)
                    | (u16::from(tail[0] & 0x3f) << 6)
                    | u16::from(tail[1] & 0x3f);
                ensure!(value >= 0x800, "overlong modified UTF-8");
                (value, 3)
            }
            _ => bail!("invalid modified UTF-8 lead byte"),
        };
        units.push(unit);
        pos += count;
    }
    String::from_utf16(&units).context("unpaired surrogate in Java string")
}

pub(super) fn string_array(blob: &[u8]) -> Result<Vec<String>> {
    let mut stream = Stream::new(blob)?;
    ensure!(stream.byte()? == 0x75, "value is not an array");
    ensure!(
        stream.byte()? == 0x72,
        "array has no independent class descriptor"
    );
    ensure!(
        stream.utf(false)? == "[Ljava.lang.String;",
        "array is not String[]"
    );
    stream.take(8)?;
    ensure!(stream.byte()? == 2, "unsupported array class flags");
    ensure!(stream.u16()? == 0, "array class declares fields");
    stream.handles.push(Handle::Descriptor);
    ensure!(
        stream.byte()? == 0x78,
        "unterminated array class descriptor"
    );
    ensure!(stream.byte()? == 0x70, "array class has a superclass");
    stream.handles.push(Handle::Array);
    let count = stream.u32()? as usize;
    ensure!(
        count <= MAX_ELEMENTS && count <= blob.len() - stream.pos,
        "array count exceeds input limit"
    );
    let mut out = Vec::new();
    let mut decoded_bytes = 0_usize;
    for _ in 0..count {
        if let Some(text) = stream.element()?
            && !text.is_empty()
        {
            decoded_bytes = decoded_bytes
                .checked_add(text.len())
                .context("decoded card strings exceed limit")?;
            ensure!(
                decoded_bytes <= 64 * 1024 * 1024,
                "decoded card strings exceed 64 MiB"
            );
            out.push(text);
        }
    }
    ensure!(stream.pos == blob.len(), "trailing bytes after Java array");
    Ok(out)
}

pub(super) fn utf_value(blob: &[u8]) -> Result<String> {
    let mut stream = Stream::new(blob)?;
    let length = match stream.byte()? {
        0x77 => usize::from(stream.byte()?),
        0x7a => stream.u32()? as usize,
        _ => bail!("deck name is not Java block data"),
    };
    let data = stream.take(length)?;
    ensure!(
        stream.pos == blob.len(),
        "trailing bytes after deck name block"
    );
    ensure!(data.len() >= 2, "truncated deck name");
    let size = usize::from(u16::from_be_bytes(data[..2].try_into()?));
    ensure!(size == data.len() - 2, "deck name length mismatch");
    modified_utf8(&data[2..])
}

/// Locate flat map entries without instantiating any untrusted Java class.
pub(super) fn entries(raw: &[u8]) -> Result<BTreeMap<String, Vec<u8>>> {
    let mut found = BTreeMap::new();
    for start in 0..raw.len().saturating_sub(3) {
        if raw[start] != 0x74 {
            continue;
        }
        let size = usize::from(u16::from_be_bytes([raw[start + 1], raw[start + 2]]));
        if size > 64 {
            continue;
        }
        let key_end = start + 3 + size;
        let Some(key_bytes) = raw.get(start + 3..key_end) else {
            continue;
        };
        let Ok(key) = modified_utf8(key_bytes) else {
            continue;
        };
        if !["deck_name_", "deck_", "sideBoardCards_", "commanderCards_"]
            .iter()
            .any(|prefix| key.starts_with(prefix))
        {
            continue;
        }
        let mut stream = Stream {
            data: raw,
            pos: key_end,
            handles: Vec::new(),
        };
        if stream.byte()? != 0x75 {
            continue;
        }
        match stream.byte()? {
            0x71 => {
                stream.take(4)?;
            }
            0x72 => {
                ensure!(stream.utf(false)? == "[B", "save value is not byte[]");
                stream.take(8)?;
                ensure!(
                    stream.byte()? == 2 && stream.u16()? == 0,
                    "unsupported byte[] descriptor"
                );
                ensure!(
                    stream.byte()? == 0x78 && stream.byte()? == 0x70,
                    "invalid byte[] descriptor terminator"
                );
            }
            _ => continue,
        }
        let count = stream.u32()? as usize;
        let value = stream.take(count)?;
        ensure!(
            value.starts_with(MAGIC),
            "deck map value is not a nested Java stream"
        );
        ensure!(!found.contains_key(&key), "duplicate deck save key");
        found.insert(key, value.to_vec());
    }
    Ok(found)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn array(elements: &[u8], count: u32) -> Vec<u8> {
        let mut bytes = MAGIC.to_vec();
        bytes.extend_from_slice(b"ur\x00\x13[Ljava.lang.String;");
        bytes.extend_from_slice(&[0; 8]);
        bytes.extend_from_slice(b"\x02\x00\x00\x78\x70");
        bytes.extend_from_slice(&count.to_be_bytes());
        bytes.extend_from_slice(elements);
        bytes
    }

    #[test]
    fn references_nulls_and_long_strings() {
        let data = array(
            b"t\x00\x03Oakq\x00\x7e\x00\x02p|\x00\x00\x00\x00\x00\x00\x00\x03Elm",
            4,
        );
        assert_eq!(string_array(&data).unwrap(), ["Oak", "Oak", "Elm"]);
        for end in 0..data.len() {
            assert!(string_array(&data[..end]).is_err());
        }
    }

    #[test]
    fn rejects_wrong_handles_counts_and_trailing_bytes() {
        assert!(string_array(&array(b"q\x00\x7e\x00\x00", 1)).is_err());
        assert!(string_array(&array(b"", u32::MAX)).is_err());
        assert!(string_array(&array(b"pp", 1)).is_err());
        assert!(utf_value(b"\xac\xed\x00\x05w\x04\x00\x03Oak").is_err());
        assert_eq!(
            utf_value(b"\xac\xed\x00\x05w\x05\x00\x03Oak").unwrap(),
            "Oak"
        );
    }

    #[test]
    fn modified_unicode() {
        assert_eq!(
            modified_utf8(b"A\xc0\x80\xed\xa0\xbd\xed\xb8\x80").unwrap(),
            "A\0😀"
        );
        assert!(modified_utf8(b"\xed\xa0\xbd").is_err());
        assert!(modified_utf8(b"\xc0\xaf").is_err());
    }

    #[test]
    fn scanner_accepts_only_checked_nested_values() {
        let mut bytes = b"t\x00\x0bdeck_name_0uq\x00\x7e\x00\x00".to_vec();
        let value = b"\xac\xed\x00\x05w\x05\x00\x03Oak";
        bytes.extend_from_slice(&(value.len() as u32).to_be_bytes());
        bytes.extend_from_slice(value);
        assert_eq!(entries(&bytes).unwrap()["deck_name_0"], value);
        bytes.pop();
        assert!(entries(&bytes).is_err());
    }
}
