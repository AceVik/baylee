//! A fingerprint of a view that both ends of a socket compute alike.
//!
//! What it is for: a wrong patch, or a right one on the wrong view, shows as a
//! digest that does not match, and the seat asks for a snapshot where it would
//! otherwise have gone on drawing a wrong table. It is no defence against the
//! host, which writes the view anyway, and carries nothing of a view's content
//! to anyone but the seat the view belongs to.
//!
//! It walks a view through `Serialize`, so it covers what the wire carries
//! whatever a field's own encoding is, and allocates nothing. Numbers go in as
//! little-endian words and lengths as `u64`, so a build for another word size
//! or byte order reads the same digest. It is not `DefaultHasher`, which is
//! free to change between compiler releases, and not `derive(Hash)`, whose
//! lengths are `usize`.
//!
//! Both ends of a socket have to agree on the encoding, and a golden test pins
//! it (`patch/tests.rs`).

use serde::ser;
use serde::{Deserialize, Serialize, Serializer};

use crate::player_view::PlayerView;

/// The fingerprint of a view ([`digest`]).
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ViewDigest(pub u64);

/// The fingerprint of `view`.
#[must_use]
pub fn digest(view: &PlayerView) -> ViewDigest {
    ViewDigest(fingerprint(view))
}

/// The fingerprint of anything that serializes. A value that refuses to is
/// fingerprinted as far as it went, the same on both ends.
pub(super) fn fingerprint<T: Serialize + ?Sized>(value: &T) -> u64 {
    let mut mix = Mix(START);
    let refused = value.serialize(&mut mix).is_err();
    mix.word(u64::from(refused));
    mix.finish()
}

/// The fractional bits of pi: where every fingerprint starts.
const START: u64 = 0x243F_6A88_85A3_08D3;

/// Odd, and with its bits evenly spread: the golden ratio, in 64 bits.
const SPREAD: u64 = 0x9E37_79B9_7F4A_7C15;

/// The running state: each word is folded in by one wide multiplication, whose
/// two halves are combined so a word reaches every bit.
struct Mix(u64);

impl Mix {
    fn word(&mut self, word: u64) {
        let wide = u128::from(self.0 ^ word) * u128::from(SPREAD);
        // Both halves on purpose: the low one alone never carries a high bit
        // of the word down.
        self.0 = ((wide >> 64) as u64 ^ wide as u64).wrapping_add(SPREAD);
    }

    fn bytes(&mut self, bytes: &[u8]) {
        self.word(bytes.len() as u64);
        let (words, rest) = bytes.as_chunks::<8>();
        for word in words {
            self.word(u64::from_le_bytes(*word));
        }
        if !rest.is_empty() {
            let mut last = [0; 8];
            last[..rest.len()].copy_from_slice(rest);
            self.word(u64::from_le_bytes(last));
        }
    }

    fn finish(mut self) -> u64 {
        self.word(SPREAD);
        self.0
    }
}

/// A value that would not serialize. Nothing here raises one; a type's own
/// `Serialize` may.
#[derive(Debug)]
struct Refused;

impl std::fmt::Display for Refused {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("a value refused to serialize")
    }
}

impl std::error::Error for Refused {}

impl ser::Error for Refused {
    fn custom<T: std::fmt::Display>(_: T) -> Self {
        Self
    }
}

/// A sequence, map or struct on its way through. It counts what it holds, so
/// two lists cut differently never read alike, and for a struct it also keeps
/// which fields were written: a field left out at its default and the next one
/// written is not the same as the first written and the next left out.
struct Compound<'a> {
    mix: &'a mut Mix,
    items: u64,
    written: u64,
}

impl Compound<'_> {
    fn item<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), Refused> {
        self.items += 1;
        self.written = self.written.rotate_left(1) | 1;
        value.serialize(&mut *self.mix)
    }

    fn skipped(&mut self) {
        self.written = self.written.rotate_left(1);
    }

    #[allow(clippy::unnecessary_wraps)] // what every `end` of a serializer returns
    fn finish(self) -> Result<(), Refused> {
        self.mix.word(self.items);
        self.mix.word(self.written);
        Ok(())
    }
}

impl<'a> Serializer for &'a mut Mix {
    type Ok = ();
    type Error = Refused;
    type SerializeSeq = Compound<'a>;
    type SerializeTuple = Compound<'a>;
    type SerializeTupleStruct = Compound<'a>;
    type SerializeTupleVariant = Compound<'a>;
    type SerializeMap = Compound<'a>;
    type SerializeStruct = Compound<'a>;
    type SerializeStructVariant = Compound<'a>;

    fn serialize_bool(self, value: bool) -> Result<(), Refused> {
        self.word(u64::from(value));
        Ok(())
    }

    fn serialize_i8(self, value: i8) -> Result<(), Refused> {
        self.serialize_i64(i64::from(value))
    }

    fn serialize_i16(self, value: i16) -> Result<(), Refused> {
        self.serialize_i64(i64::from(value))
    }

    fn serialize_i32(self, value: i32) -> Result<(), Refused> {
        self.serialize_i64(i64::from(value))
    }

    fn serialize_i64(self, value: i64) -> Result<(), Refused> {
        self.word(value.cast_unsigned());
        Ok(())
    }

    fn serialize_i128(self, value: i128) -> Result<(), Refused> {
        self.serialize_u128(value.cast_unsigned())
    }

    fn serialize_u8(self, value: u8) -> Result<(), Refused> {
        self.serialize_u64(u64::from(value))
    }

    fn serialize_u16(self, value: u16) -> Result<(), Refused> {
        self.serialize_u64(u64::from(value))
    }

    fn serialize_u32(self, value: u32) -> Result<(), Refused> {
        self.serialize_u64(u64::from(value))
    }

    fn serialize_u64(self, value: u64) -> Result<(), Refused> {
        self.word(value);
        Ok(())
    }

    #[allow(clippy::cast_possible_truncation)] // the low word, on purpose
    fn serialize_u128(self, value: u128) -> Result<(), Refused> {
        self.word(value as u64);
        self.word((value >> 64) as u64);
        Ok(())
    }

    fn serialize_f32(self, value: f32) -> Result<(), Refused> {
        self.serialize_u64(u64::from(value.to_bits()))
    }

    fn serialize_f64(self, value: f64) -> Result<(), Refused> {
        self.serialize_u64(value.to_bits())
    }

    fn serialize_char(self, value: char) -> Result<(), Refused> {
        self.serialize_u64(u64::from(u32::from(value)))
    }

    fn serialize_str(self, value: &str) -> Result<(), Refused> {
        self.bytes(value.as_bytes());
        Ok(())
    }

    fn serialize_bytes(self, value: &[u8]) -> Result<(), Refused> {
        self.bytes(value);
        Ok(())
    }

    fn serialize_none(self) -> Result<(), Refused> {
        self.word(0);
        Ok(())
    }

    fn serialize_some<T: Serialize + ?Sized>(self, value: &T) -> Result<(), Refused> {
        self.word(1);
        value.serialize(self)
    }

    fn serialize_unit(self) -> Result<(), Refused> {
        self.word(0);
        Ok(())
    }

    fn serialize_unit_struct(self, _: &'static str) -> Result<(), Refused> {
        self.serialize_unit()
    }

    fn serialize_unit_variant(
        self,
        _: &'static str,
        variant: u32,
        _: &'static str,
    ) -> Result<(), Refused> {
        self.serialize_u32(variant)
    }

    fn serialize_newtype_struct<T: Serialize + ?Sized>(
        self,
        _: &'static str,
        value: &T,
    ) -> Result<(), Refused> {
        value.serialize(self)
    }

    fn serialize_newtype_variant<T: Serialize + ?Sized>(
        self,
        _: &'static str,
        variant: u32,
        _: &'static str,
        value: &T,
    ) -> Result<(), Refused> {
        self.word(u64::from(variant));
        value.serialize(self)
    }

    fn serialize_seq(self, _: Option<usize>) -> Result<Compound<'a>, Refused> {
        Ok(Compound {
            mix: self,
            items: 0,
            written: 0,
        })
    }

    fn serialize_tuple(self, len: usize) -> Result<Compound<'a>, Refused> {
        self.serialize_seq(Some(len))
    }

    fn serialize_tuple_struct(self, _: &'static str, len: usize) -> Result<Compound<'a>, Refused> {
        self.serialize_seq(Some(len))
    }

    fn serialize_tuple_variant(
        self,
        _: &'static str,
        variant: u32,
        _: &'static str,
        len: usize,
    ) -> Result<Compound<'a>, Refused> {
        self.word(u64::from(variant));
        self.serialize_seq(Some(len))
    }

    fn serialize_map(self, len: Option<usize>) -> Result<Compound<'a>, Refused> {
        self.serialize_seq(len)
    }

    fn serialize_struct(self, _: &'static str, len: usize) -> Result<Compound<'a>, Refused> {
        self.serialize_seq(Some(len))
    }

    fn serialize_struct_variant(
        self,
        _: &'static str,
        variant: u32,
        _: &'static str,
        len: usize,
    ) -> Result<Compound<'a>, Refused> {
        self.word(u64::from(variant));
        self.serialize_seq(Some(len))
    }
}

impl ser::SerializeSeq for Compound<'_> {
    type Ok = ();
    type Error = Refused;

    fn serialize_element<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), Refused> {
        self.item(value)
    }

    fn end(self) -> Result<(), Refused> {
        self.finish()
    }
}

impl ser::SerializeTuple for Compound<'_> {
    type Ok = ();
    type Error = Refused;

    fn serialize_element<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), Refused> {
        self.item(value)
    }

    fn end(self) -> Result<(), Refused> {
        self.finish()
    }
}

impl ser::SerializeTupleStruct for Compound<'_> {
    type Ok = ();
    type Error = Refused;

    fn serialize_field<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), Refused> {
        self.item(value)
    }

    fn end(self) -> Result<(), Refused> {
        self.finish()
    }
}

impl ser::SerializeTupleVariant for Compound<'_> {
    type Ok = ();
    type Error = Refused;

    fn serialize_field<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), Refused> {
        self.item(value)
    }

    fn end(self) -> Result<(), Refused> {
        self.finish()
    }
}

impl ser::SerializeMap for Compound<'_> {
    type Ok = ();
    type Error = Refused;

    fn serialize_key<T: Serialize + ?Sized>(&mut self, key: &T) -> Result<(), Refused> {
        self.item(key)
    }

    fn serialize_value<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), Refused> {
        value.serialize(&mut *self.mix)
    }

    fn end(self) -> Result<(), Refused> {
        self.finish()
    }
}

impl ser::SerializeStruct for Compound<'_> {
    type Ok = ();
    type Error = Refused;

    fn serialize_field<T: Serialize + ?Sized>(
        &mut self,
        _: &'static str,
        value: &T,
    ) -> Result<(), Refused> {
        self.item(value)
    }

    fn skip_field(&mut self, _: &'static str) -> Result<(), Refused> {
        self.skipped();
        Ok(())
    }

    fn end(self) -> Result<(), Refused> {
        self.finish()
    }
}

impl ser::SerializeStructVariant for Compound<'_> {
    type Ok = ();
    type Error = Refused;

    fn serialize_field<T: Serialize + ?Sized>(
        &mut self,
        _: &'static str,
        value: &T,
    ) -> Result<(), Refused> {
        self.item(value)
    }

    fn skip_field(&mut self, _: &'static str) -> Result<(), Refused> {
        self.skipped();
        Ok(())
    }

    fn end(self) -> Result<(), Refused> {
        self.finish()
    }
}
