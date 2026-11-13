//! ISO base media file boxes (ISO/IEC 14496-12 §4.2), read without trusting
//! one length: a box's stated size is checked against what its parent holds,
//! and a container yields at most [`MAX_CHILDREN`] of them, so a file of
//! nothing but tiny boxes cannot make a loop of its own length.

use crate::Error;

/// The most boxes one container may hold. An AVIF sequence's busiest
/// container (`stbl`) has seven or eight; a thousand is generous and still
/// bounds the walk.
pub(crate) const MAX_CHILDREN: usize = 1024;

/// One box: its four-character type and its body, the header taken off.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Atom<'a> {
    pub(crate) kind: [u8; 4],
    pub(crate) body: &'a [u8],
}

/// The boxes laid end to end in `bytes`, in order.
pub(crate) fn children(bytes: &[u8]) -> Result<Vec<Atom<'_>>, Error> {
    let mut rest = bytes;
    let mut out = Vec::new();
    while !rest.is_empty() {
        if out.len() == MAX_CHILDREN {
            return Err(Error::OutOfBounds("too many boxes in one container"));
        }
        let mut at = Cursor::new(rest);
        let size32 = at.u32("box size")?;
        let mut kind = [0; 4];
        kind.copy_from_slice(at.take(4, "box type")?);
        let (header, size) = match size32 {
            // A 64-bit size follows the type.
            1 => (16, at.u64("large box size")?),
            // The box runs to the end of its parent.
            0 => (8, rest.len() as u64),
            n => (8, u64::from(n)),
        };
        if size < header || size > rest.len() as u64 {
            return Err(Error::Truncated("a box's size runs past its parent"));
        }
        let size = size as usize;
        out.push(Atom {
            kind,
            body: &rest[header as usize..size],
        });
        rest = &rest[size..];
    }
    Ok(out)
}

/// The one child of type `kind`, if there is one. A second of the same type
/// is refused: a reader that took the first and a decoder that took the last
/// would disagree about the file.
#[allow(clippy::trivially_copy_pass_by_ref)] // a box's type is written `b"moov"`
pub(crate) fn only<'a>(atoms: &[Atom<'a>], kind: &[u8; 4]) -> Result<Option<Atom<'a>>, Error> {
    let mut found = None;
    for atom in atoms {
        if &atom.kind == kind {
            if found.is_some() {
                return Err(Error::Unsupported(
                    "a box that may appear once appears twice",
                ));
            }
            found = Some(*atom);
        }
    }
    Ok(found)
}

/// The one child of type `kind`, which must be there.
#[allow(clippy::trivially_copy_pass_by_ref)] // a box's type is written `b"moov"`
pub(crate) fn needed<'a>(
    atoms: &[Atom<'a>],
    kind: &[u8; 4],
    what: &'static str,
) -> Result<Atom<'a>, Error> {
    only(atoms, kind)?.ok_or(Error::Unsupported(what))
}

/// A full box's version and flags (§4.2.2), and the cursor after them.
pub(crate) fn full(body: &[u8]) -> Result<(u8, u32, Cursor<'_>), Error> {
    let mut at = Cursor::new(body);
    let word = at.u32("full box header")?;
    Ok(((word >> 24) as u8, word & 0x00ff_ffff, at))
}

/// Big-endian fields read off the front of a body, each refused rather than
/// read past its end.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Cursor<'a> {
    bytes: &'a [u8],
}

impl<'a> Cursor<'a> {
    pub(crate) fn new(bytes: &'a [u8]) -> Self {
        Self { bytes }
    }

    pub(crate) fn rest(&self) -> &'a [u8] {
        self.bytes
    }

    pub(crate) fn take(&mut self, n: usize, what: &'static str) -> Result<&'a [u8], Error> {
        if n > self.bytes.len() {
            return Err(Error::Truncated(what));
        }
        let (head, tail) = self.bytes.split_at(n);
        self.bytes = tail;
        Ok(head)
    }

    pub(crate) fn skip(&mut self, n: usize, what: &'static str) -> Result<(), Error> {
        self.take(n, what).map(|_| ())
    }

    pub(crate) fn u16(&mut self, what: &'static str) -> Result<u16, Error> {
        let b = self.take(2, what)?;
        Ok(u16::from_be_bytes([b[0], b[1]]))
    }

    pub(crate) fn u32(&mut self, what: &'static str) -> Result<u32, Error> {
        let b = self.take(4, what)?;
        Ok(u32::from_be_bytes([b[0], b[1], b[2], b[3]]))
    }

    pub(crate) fn u64(&mut self, what: &'static str) -> Result<u64, Error> {
        let b = self.take(8, what)?;
        let mut word = [0; 8];
        word.copy_from_slice(b);
        Ok(u64::from_be_bytes(word))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[allow(clippy::trivially_copy_pass_by_ref)] // a box's type is written `b"moov"`
    fn boxed(kind: &[u8; 4], body: &[u8]) -> Vec<u8> {
        let mut out = ((body.len() + 8) as u32).to_be_bytes().to_vec();
        out.extend_from_slice(kind);
        out.extend_from_slice(body);
        out
    }

    #[test]
    fn boxes_are_read_in_order_and_a_size_past_the_parent_is_refused() {
        let mut file = boxed(b"ftyp", b"avis");
        file.extend(boxed(b"free", &[1, 2, 3]));
        let atoms = children(&file).unwrap();
        assert_eq!(atoms.len(), 2);
        assert_eq!(&atoms[0].kind, b"ftyp");
        assert_eq!(atoms[0].body, b"avis");
        assert_eq!(atoms[1].body, &[1, 2, 3]);

        let mut long = boxed(b"ftyp", b"avis");
        long[3] += 1;
        assert_eq!(
            children(&long).unwrap_err(),
            Error::Truncated("a box's size runs past its parent")
        );
        // A size smaller than its own header would loop for ever on zero.
        let mut short = boxed(b"ftyp", b"");
        short[3] = 4;
        assert!(children(&short).is_err());
        // A size of zero runs to the end of the parent.
        let mut open = boxed(b"mdat", b"abc");
        open[..4].copy_from_slice(&0u32.to_be_bytes());
        assert_eq!(children(&open).unwrap()[0].body, b"abc");
    }

    #[test]
    fn a_large_size_is_read_and_held_to_the_parent() {
        let mut file = 1u32.to_be_bytes().to_vec();
        file.extend_from_slice(b"mdat");
        file.extend_from_slice(&(16u64 + 2).to_be_bytes());
        file.extend_from_slice(&[7, 8]);
        assert_eq!(children(&file).unwrap()[0].body, &[7, 8]);
        file[15] = 200;
        assert!(children(&file).is_err());
    }

    #[test]
    fn a_container_of_many_tiny_boxes_is_refused_at_the_bound() {
        let file: Vec<u8> = (0..=MAX_CHILDREN)
            .flat_map(|_| boxed(b"free", &[]))
            .collect();
        assert_eq!(
            children(&file).unwrap_err(),
            Error::OutOfBounds("too many boxes in one container")
        );
        let fits: Vec<u8> = (0..MAX_CHILDREN)
            .flat_map(|_| boxed(b"free", &[]))
            .collect();
        assert_eq!(children(&fits).unwrap().len(), MAX_CHILDREN);
    }

    #[test]
    fn a_box_that_may_appear_once_is_refused_twice() {
        let mut file = boxed(b"moov", &[]);
        file.extend(boxed(b"moov", &[]));
        let atoms = children(&file).unwrap();
        assert!(only(&atoms, b"moov").is_err());
        assert!(only(&atoms, b"meta").unwrap().is_none());
    }

    #[test]
    fn a_cursor_never_reads_past_its_end() {
        let mut at = Cursor::new(&[0, 1, 2]);
        assert_eq!(at.u16("a").unwrap(), 1);
        assert_eq!(at.u16("b").unwrap_err(), Error::Truncated("b"));
        assert_eq!(at.rest(), &[2]);
    }
}
