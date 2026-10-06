//! What the view leaves out of its JSON: a field at its default is not
//! written, and a reader takes the default for a field that is not there.
//!
//! A permanent's object carried some forty fields, and on a busy board more
//! than half of them were `null`, `[]`, `false` or `0` — 592 bytes for a
//! vanilla token, of which 230 say something. Writing only what is not the
//! default cut a busy four-seat view's JSON, and with it the engine's
//! encoding and the client's decoding (`docs/perf-client.md`).

/// For `skip_serializing_if`: a `false` is not written.
#[allow(clippy::trivially_copy_pass_by_ref)] // serde passes the field by reference
pub(crate) const fn is_false(value: &bool) -> bool {
    !*value
}

/// For `skip_serializing_if`: a zero is not written.
#[allow(clippy::trivially_copy_pass_by_ref)] // serde passes the field by reference
pub(crate) const fn is_zero_u16(value: &u16) -> bool {
    *value == 0
}
