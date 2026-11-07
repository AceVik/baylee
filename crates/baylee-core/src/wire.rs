//! What a game's frames are written in: `postcard`, in one place.
//!
//! The gateway forwards frames unread, so a guest's bytes reach an engine and a
//! host's reach every client: [`decode`] reads foreign bytes on both sides. What
//! keeps that safe is written down here once:
//!
//! - **No panic, no `unsafe`.** A frame that does not read is an error.
//! - **Nothing allocates on a length alone.** A claimed length is checked against
//!   the bytes that remain before a string is read, and a list grows as its
//!   elements arrive rather than from what it claims (`postcard` hides the claim
//!   from `serde` once it exceeds what is left, and `serde` caps the rest at 1 MiB
//!   of preallocation). `crates/baylee-view/tests/hostile_frames.rs` holds this
//!   to a counting allocator.
//! - **Nothing recurses.** `postcard` keeps no depth count of its own, unlike
//!   `serde_json`, so a type a client can send must be one that cannot nest. A
//!   payment's steps are a `PaymentStep` and not a `PlayerAction` for this reason:
//!   a frame of 32 768 nested payments, 64 KiB, aborts a process on a thread's
//!   default stack.
//! - **Nothing is left over.** Bytes after the value are refused.
//!
//! JSON stays what it was for everything that is not a frame: tapes, records,
//! tools, and the hashes kept beside stored data
//! (`baylee-train`'s `view_hash_fnv1a64`, `pending_sha256`), which are of JSON
//! bytes. A format that names no fields cannot leave one out, so the types on the
//! wire leave a field out of JSON only through [`left_out`], which holds while no
//! frame is being written.
//!

use std::cell::Cell;

use serde::{Deserialize, Serialize};

thread_local! {
    /// Whether [`encode`] is running on this thread.
    static WRITING_A_FRAME: Cell<bool> = const { Cell::new(false) };
}

/// Puts [`WRITING_A_FRAME`] back, also when a value's `Serialize` unwinds.
struct Writing;

impl Drop for Writing {
    fn drop(&mut self) {
        WRITING_A_FRAME.set(false);
    }
}

/// For `skip_serializing_if`: a field at its default is not written in JSON, and
/// always is in a frame, whose reader takes fields by position.
#[must_use]
pub fn left_out<T: Default + PartialEq>(value: &T) -> bool {
    !WRITING_A_FRAME.get() && *value == T::default()
}

/// Runs `write` with every field written, as a frame needs: what [`encode`] does,
/// for measuring another format against it.
pub fn every_field<R>(write: impl FnOnce() -> R) -> R {
    WRITING_A_FRAME.set(true);
    let _writing = Writing;
    write()
}

/// `value` as a frame carries it. Empty if it does not encode, which nothing on
/// the wire can make happen: every type here has a known length.
pub fn encode<T: Serialize + ?Sized>(value: &T) -> Vec<u8> {
    every_field(|| postcard::to_allocvec(value).unwrap_or_default())
}

/// Bytes that are not the value they were read as.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Unreadable(String);

impl std::fmt::Display for Unreadable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for Unreadable {}

/// Reads a value from foreign bytes, all of them.
///
/// # Errors
/// When the bytes are not one whole `T`.
pub fn decode<'de, T: Deserialize<'de>>(bytes: &'de [u8]) -> Result<T, Unreadable> {
    let (value, rest) =
        postcard::take_from_bytes(bytes).map_err(|why| Unreadable(why.to_string()))?;
    if rest.is_empty() {
        Ok(value)
    } else {
        Err(Unreadable(format!("{} bytes after the value", rest.len())))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Serialize, Deserialize, PartialEq, Debug, Default)]
    struct Sparse {
        #[serde(default, skip_serializing_if = "left_out")]
        name: Option<String>,
        #[serde(default, skip_serializing_if = "left_out")]
        count: u16,
        #[serde(default, skip_serializing_if = "left_out")]
        list: Vec<u8>,
        kept: bool,
    }

    /// The reason [`left_out`] exists: a frame's reader goes by position.
    #[test]
    fn a_default_is_written_in_a_frame_and_left_out_of_json() {
        let sparse = Sparse::default();
        assert_eq!(decode::<Sparse>(&encode(&sparse)), Ok(Sparse::default()));
        assert_eq!(serde_json::to_string(&sparse).unwrap(), r#"{"kept":false}"#);
        let full = Sparse {
            name: Some("Llanowar".to_owned()),
            count: 300,
            list: vec![1, 2],
            kept: true,
        };
        assert_eq!(decode::<Sparse>(&encode(&full)), Ok(full));
    }

    /// JSON written after a frame is JSON as it was: the flag does not stick.
    #[test]
    fn a_frame_leaves_json_as_it_found_it() {
        encode(&Sparse::default());
        assert_eq!(
            serde_json::to_string(&Sparse::default()).unwrap(),
            r#"{"kept":false}"#
        );
    }

    #[test]
    fn a_value_that_unwinds_leaves_json_as_it_found_it() {
        struct Fails;
        impl Serialize for Fails {
            fn serialize<S: serde::Serializer>(&self, _: S) -> Result<S::Ok, S::Error> {
                panic!("a value that cannot be written");
            }
        }
        let unwound = std::panic::catch_unwind(|| encode(&Fails));
        assert!(unwound.is_err());
        assert_eq!(
            serde_json::to_string(&Sparse::default()).unwrap(),
            r#"{"kept":false}"#
        );
    }

    #[test]
    fn bytes_after_the_value_are_refused() {
        let mut bytes = encode(&7_u32);
        bytes.push(0);
        assert!(decode::<u32>(&bytes).is_err());
    }

    #[test]
    fn a_length_past_the_end_is_refused() {
        // A string that claims more than a terabyte, with three bytes behind it.
        let claim = [0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0x01, b'a', b'b', b'c'];
        assert!(decode::<String>(&claim).is_err());
        assert!(decode::<Vec<u64>>(&claim).is_err());
    }
}
