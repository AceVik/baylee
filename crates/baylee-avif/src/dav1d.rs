//! The one door to rav1d, whose only interface is dav1d's C ABI. Everything
//! `unsafe` in this crate is here, and every call keeps the same three
//! promises dav1d asks for: the context is one `dav1d_open` made and nobody
//! has closed, every pointer handed in points at a value of its type that
//! lives for the call, and a picture or data buffer is unreferenced exactly
//! once. The context is used from one thread at a time (`&mut self`), which
//! rav1d's own `try_lock().unwrap()` on its state also requires.
//!
//! What crosses back is copied out row by row into owned planes before the
//! picture is released, so no pointer of rav1d's outlives the call that got
//! it.

#![allow(unsafe_code)]

use std::ptr::NonNull;

use rav1d::include::dav1d::data::Dav1dData;
use rav1d::include::dav1d::dav1d::{Dav1dContext, Dav1dLogger, Dav1dSettings};
use rav1d::include::dav1d::headers::{DAV1D_PIXEL_LAYOUT_I400, DAV1D_PIXEL_LAYOUT_I420};
use rav1d::include::dav1d::picture::Dav1dPicture;
use rav1d::src::lib::{
    dav1d_close, dav1d_data_create, dav1d_data_unref, dav1d_default_settings, dav1d_flush,
    dav1d_get_picture, dav1d_open, dav1d_picture_unref, dav1d_send_data,
};

use crate::Error;
use crate::frame::{AlignedPlanes, Frame, Matrix};

/// dav1d's "try again": the decoder wants its output taken first (on
/// sending) or more input (on getting).
const AGAIN: i32 = -libc::EAGAIN;

/// Owns a referenced `Dav1dPicture` so `dav1d_picture_unref` runs once on drop.
struct PictureGuard(Dav1dPicture);

impl Drop for PictureGuard {
    fn drop(&mut self) {
        // SAFETY: `PictureGuard` is constructed only after a successful
        // `dav1d_get_picture` call and is dropped once, releasing its planes.
        unsafe { dav1d_picture_unref(Some(NonNull::from(&mut self.0))) };
    }
}

/// One AV1 decoder.
pub(crate) struct Decoder {
    ctx: Option<Dav1dContext>,
}

// SAFETY: a `Dav1dContext` is an `Arc` of rav1d's context behind a raw
// pointer; rav1d's context is `Send` + `Sync` (its own threads share it), and
// this wrapper only ever touches it through `&mut self`.
unsafe impl Send for Decoder {}

impl Decoder {
    /// A decoder on `threads` threads (at least one) that refuses any frame
    /// of more than `max_pixels` pixels before it allocates it.
    pub(crate) fn open(threads: u32, max_pixels: u32) -> Result<Self, Error> {
        let mut settings = std::mem::MaybeUninit::<Dav1dSettings>::uninit();
        // SAFETY: `dav1d_default_settings` writes a whole `Dav1dSettings`
        // into the pointer it is given and reads nothing from it; the
        // pointer is to a local of exactly that type.
        unsafe { dav1d_default_settings(NonNull::new_unchecked(settings.as_mut_ptr())) };
        // SAFETY: initialised by the call above.
        let mut settings = unsafe { settings.assume_init() };
        settings.n_threads = threads.clamp(1, 8) as i32;
        // One frame in flight: a sample goes in and its picture comes out,
        // which keeps memory to the reference frames and the clock simple.
        settings.max_frame_delay = 1;
        settings.frame_size_limit = max_pixels;
        settings.all_layers = 0;
        settings.apply_grain = 1;
        // SAFETY: a logger with no callback never calls anything.
        settings.logger = unsafe { Dav1dLogger::new(None, None) };
        let mut ctx: Option<Dav1dContext> = None;
        // SAFETY: both pointers are to live locals of the types dav1d_open
        // reads (`Dav1dSettings`) and writes (`Option<Dav1dContext>`).
        let result = unsafe {
            dav1d_open(
                Some(NonNull::from(&mut ctx)),
                Some(NonNull::from(&mut settings)),
            )
        };
        if result.0 != 0 || ctx.is_none() {
            return Err(Error::Decode("the decoder would not open"));
        }
        Ok(Self { ctx })
    }

    /// Submits one temporal unit (`sample`) and returns its referenced picture.
    fn decode_picture(&mut self, sample: &[u8]) -> Result<PictureGuard, Error> {
        let ctx = self.ctx.ok_or(Error::Decode("the decoder is closed"))?;
        if sample.is_empty() {
            return Err(Error::Decode("an empty sample"));
        }
        let mut data = Dav1dData::default();
        // SAFETY: `data` is a live `Dav1dData`; on success the returned
        // pointer is the start of a fresh buffer of `sample.len()` bytes that
        // `data` owns.
        let buffer = unsafe { dav1d_data_create(Some(NonNull::from(&mut data)), sample.len()) };
        if buffer.is_null() {
            return Err(Error::Decode("no memory for a sample"));
        }
        // SAFETY: `buffer` is valid for `sample.len()` bytes (above) and
        // cannot overlap `sample`, which the caller owns.
        unsafe { std::ptr::copy_nonoverlapping(sample.as_ptr(), buffer, sample.len()) };
        let mut picture = Dav1dPicture::default();
        let mut got = false;
        // With one frame in flight a sample gives its picture at once; the
        // loop only takes the picture first when the decoder asks for that,
        // and is bounded either way.
        for _ in 0..4 {
            if data.sz > 0 {
                // SAFETY: `ctx` is open (`Some`, never closed except in `Drop`)
                // and `data` is the live buffer made above.
                let sent = unsafe { dav1d_send_data(Some(ctx), Some(NonNull::from(&mut data))) };
                if sent.0 != 0 && sent.0 != AGAIN {
                    break;
                }
            }
            // SAFETY: `ctx` is open, `picture` is a live `Dav1dPicture` that
            // dav1d overwrites on `0`.
            let res = unsafe { dav1d_get_picture(Some(ctx), Some(NonNull::from(&mut picture))) };
            if res.0 == 0 {
                got = true;
                break;
            }
            if res.0 != AGAIN {
                break;
            }
        }
        if data.sz > 0 {
            // SAFETY: `data` still owns its buffer, which is released once.
            unsafe { dav1d_data_unref(Some(NonNull::from(&mut data))) };
        }
        if !got {
            return Err(Error::Decode("a sample gave no picture"));
        }
        Ok(PictureGuard(picture))
    }

    /// Decodes one sample (a temporal unit) into `frame`, writing its planes
    /// in place so a frame kept between calls is reused. An alpha track's
    /// sample is decoded with `luma_only`: its luma is the opacity, and it may
    /// be monochrome.
    pub(crate) fn decode(
        &mut self,
        sample: &[u8],
        width: u32,
        height: u32,
        frame: &mut Frame,
        luma_only: bool,
    ) -> Result<(), Error> {
        let guard = self.decode_picture(sample)?;
        copy_out(&guard.0, width, height, frame, luma_only)
    }

    /// Decodes one 8-bit 4:2:0 colour sample directly into 256-byte row-aligned
    /// `planes` with zero heap allocations after initial buffer sizing.
    pub(crate) fn decode_aligned(
        &mut self,
        sample: &[u8],
        width: u32,
        height: u32,
        planes: &mut AlignedPlanes,
    ) -> Result<(), Error> {
        let guard = self.decode_picture(sample)?;
        copy_out_aligned(&guard.0, width, height, planes)
    }

    /// Forgets every reference frame, for a sequence starting over.
    pub(crate) fn flush(&mut self) {
        if let Some(ctx) = self.ctx {
            // SAFETY: `ctx` is open.
            unsafe { dav1d_flush(ctx) };
        }
    }
}

impl Drop for Decoder {
    fn drop(&mut self) {
        if self.ctx.is_some() {
            // SAFETY: `ctx` was made by `dav1d_open` and is closed only here;
            // `dav1d_close` sets it to `None`.
            unsafe { dav1d_close(Some(NonNull::from(&mut self.ctx))) };
        }
    }
}

/// Copies a picture's planes into `frame`, refusing a picture that is not
/// the 8-bit 4:2:0 frame of the size the track declared.
fn copy_out(
    picture: &Dav1dPicture,
    width: u32,
    height: u32,
    frame: &mut Frame,
    luma_only: bool,
) -> Result<(), Error> {
    let shape = &picture.p;
    if shape.w < 0 || shape.h < 0 || shape.w as u32 != width || shape.h as u32 != height {
        return Err(Error::Decode("a frame of another size than its track's"));
    }
    let monochrome = shape.layout == DAV1D_PIXEL_LAYOUT_I400;
    if shape.bpc != 8 || !(shape.layout == DAV1D_PIXEL_LAYOUT_I420 || (luma_only && monochrome)) {
        return Err(Error::Decode("not an 8-bit 4:2:0 frame"));
    }
    let (w, h) = (width as usize, height as usize);
    let (cw, ch) = (w.div_ceil(2), h.div_ceil(2));
    let (luma_stride, chroma_stride) = (picture.stride[0], picture.stride[1]);
    if luma_stride < w as isize || (!luma_only && chroma_stride < cw as isize) {
        return Err(Error::Decode("a stride narrower than its row"));
    }
    let Some(luma) = picture.data[0] else {
        return Err(Error::Decode("a plane is missing"));
    };
    frame.width = width;
    frame.height = height;
    // SAFETY: dav1d's picture holds `h` rows of at least `w` bytes of luma,
    // each `luma_stride` bytes after the last (checked `>= w` above), for as
    // long as `PictureGuard` keeps `picture` referenced.
    unsafe { copy_plane(luma.cast(), luma_stride, w, h, &mut frame.y) };
    if luma_only {
        return Ok(());
    }
    let [_, Some(cb), Some(cr)] = picture.data else {
        return Err(Error::Decode("a plane is missing"));
    };
    // SAFETY: a 4:2:0 picture (checked above) holds `ch` rows of at least
    // `cw` bytes for each chroma plane, `chroma_stride` apart (checked `>= cw`),
    // for as long as `PictureGuard` keeps `picture` referenced.
    unsafe {
        copy_plane(cb.cast(), chroma_stride, cw, ch, &mut frame.u);
        copy_plane(cr.cast(), chroma_stride, cw, ch, &mut frame.v);
    }
    if let Some(seq) = picture.seq_hdr {
        // SAFETY: a picture's sequence header lives as long as the picture.
        let seq = unsafe { seq.as_ref() };
        frame.matrix = Matrix::from_h273(seq.mtrx);
        frame.full_range = seq.color_range != 0;
    }
    Ok(())
}

/// Copies an 8-bit 4:2:0 picture directly into 256-byte row-aligned `planes`.
fn copy_out_aligned(
    picture: &Dav1dPicture,
    width: u32,
    height: u32,
    planes: &mut AlignedPlanes,
) -> Result<(), Error> {
    let shape = &picture.p;
    if shape.w < 0 || shape.h < 0 || shape.w as u32 != width || shape.h as u32 != height {
        return Err(Error::Decode("a frame of another size than its track's"));
    }
    if shape.bpc != 8 || shape.layout != DAV1D_PIXEL_LAYOUT_I420 {
        return Err(Error::Decode("not an 8-bit 4:2:0 frame"));
    }
    let (w, h) = (width as usize, height as usize);
    let (cw, ch) = (w.div_ceil(2), h.div_ceil(2));
    let (luma_stride, chroma_stride) = (picture.stride[0], picture.stride[1]);
    if luma_stride < w as isize || chroma_stride < cw as isize {
        return Err(Error::Decode("a stride narrower than its row"));
    }
    let [Some(luma), Some(cb), Some(cr)] = picture.data else {
        return Err(Error::Decode("a plane is missing"));
    };
    planes.prepare(width, height);
    let y_dst_stride = planes.y_stride as usize;
    let c_dst_stride = planes.c_stride as usize;
    let (y_dst, u_dst, v_dst) = planes.planes_mut();
    // SAFETY: `picture` is referenced by `PictureGuard`; `luma` has `h` rows of
    // at least `w` readable bytes spaced `luma_stride >= w` apart, and `y_dst`
    // has `h * y_dst_stride` writable bytes (`y_dst_stride >= w`), non-overlapping.
    unsafe {
        copy_plane_strided(luma.cast(), luma_stride, w, h, y_dst, y_dst_stride);
    }
    // SAFETY: `cb` and `cr` each have `ch` rows of at least `cw` readable bytes
    // spaced `chroma_stride >= cw` apart, and `u_dst` / `v_dst` each have
    // `ch * c_dst_stride` writable bytes (`c_dst_stride >= cw`), non-overlapping.
    unsafe {
        copy_plane_strided(cb.cast(), chroma_stride, cw, ch, u_dst, c_dst_stride);
        copy_plane_strided(cr.cast(), chroma_stride, cw, ch, v_dst, c_dst_stride);
    }
    if let Some(seq) = picture.seq_hdr {
        // SAFETY: a picture's sequence header lives as long as the picture.
        let seq = unsafe { seq.as_ref() };
        planes.matrix = Matrix::from_h273(seq.mtrx);
        planes.full_range = seq.color_range != 0;
    }
    Ok(())
}

/// # Safety
///
/// `base` points at `rows` rows of at least `width` readable bytes, each
/// `stride >= width` bytes after the one before.
unsafe fn copy_plane(
    base: NonNull<u8>,
    stride: isize,
    width: usize,
    rows: usize,
    out: &mut Vec<u8>,
) {
    let total = width * rows;
    if out.len() != total {
        out.resize(total, 0);
    }
    // SAFETY: `out` has `rows * width` writable bytes (`dst_stride == width`),
    // and `base` satisfies `copy_plane_strided`'s preconditions.
    unsafe { copy_plane_strided(base, stride, width, rows, out, width) };
}

/// # Safety
///
/// - `base` points at `rows` rows of at least `width` readable bytes, each
///   `src_stride` (`>= width as isize`) bytes after the previous row.
/// - `dst` has length `>= rows * dst_stride`, with `dst_stride >= width`, and
///   does not overlap the source plane.
unsafe fn copy_plane_strided(
    base: NonNull<u8>,
    src_stride: isize,
    width: usize,
    rows: usize,
    dst: &mut [u8],
    dst_stride: usize,
) {
    if rows == 0 || width == 0 {
        return;
    }
    let src_stride_usize = src_stride as usize;
    if src_stride_usize == width && dst_stride == width {
        let total = width * rows;
        // SAFETY: when `src_stride == width == dst_stride`, all `rows * width`
        // source and destination bytes are contiguous and non-overlapping.
        unsafe { std::ptr::copy_nonoverlapping(base.as_ptr(), dst.as_mut_ptr(), total) };
        return;
    }
    for row in 0..rows {
        let dst_at = row * dst_stride;
        // SAFETY: by the caller's contract, row `row` starts `row * src_stride`
        // bytes from `base` with `width` readable bytes, and `dst[dst_at..dst_at + width]`
        // is in bounds (`row < rows`, `width <= dst_stride`) and non-overlapping.
        unsafe {
            std::ptr::copy_nonoverlapping(
                base.as_ptr().add(row * src_stride_usize),
                dst.as_mut_ptr().add(dst_at),
                width,
            );
        }
    }
}
