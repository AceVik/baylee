//! Optional native libsoxr 0.1 ABI, loaded only for offline bank reconstruction.
//! No library or resampler is linked into the client or required by CI.
#![allow(clippy::cast_precision_loss)]

use anyhow::{Context, Result, ensure};
use libloading::Library;
use std::{
    ffi::{CStr, c_char, c_int, c_uint, c_ulong, c_void},
    path::Path,
    ptr,
};

// Layouts and signatures follow libsoxr's public 0.1 C header, soxr.h.
#[repr(C)]
struct Io {
    input: c_int,
    output: c_int,
    scale: f64,
    reserved: *mut c_void,
    flags: c_ulong,
}
#[repr(C)]
struct Quality {
    precision: f64,
    phase: f64,
    passband: f64,
    stopband: f64,
    reserved: *mut c_void,
    flags: c_ulong,
}
type Error = *const c_char;
type Create = unsafe extern "C" fn(
    f64,
    f64,
    c_uint,
    *mut Error,
    *const Io,
    *const Quality,
    *const c_void,
) -> *mut c_void;
type Process = unsafe extern "C" fn(
    *mut c_void,
    *const c_void,
    usize,
    *mut usize,
    *mut c_void,
    usize,
    *mut usize,
) -> Error;
type Delete = unsafe extern "C" fn(*mut c_void);

pub(super) struct Resampler(Library);
struct Stream {
    handle: *mut c_void,
    delete: Delete,
}
impl Drop for Stream {
    fn drop(&mut self) {
        // SAFETY: handle is the live, uniquely owned soxr_create result; the
        // library outlives this local guard, including every error return.
        unsafe {
            (self.delete)(self.handle);
        }
    }
}

fn check(error: Error) -> Result<()> {
    if !error.is_null() {
        // SAFETY: libsoxr returns null or a library-owned NUL-terminated error.
        anyhow::bail!(
            "libsoxr: {}",
            unsafe { CStr::from_ptr(error) }.to_string_lossy()
        );
    }
    Ok(())
}

impl Resampler {
    pub(super) fn open(path: &Path) -> Result<Self> {
        // SAFETY: explicitly selected native tool library, same trust boundary
        // as invoking ffmpeg. Symbol signatures below match the stable C ABI.
        let library = unsafe { Library::new(path) }.context("load native libsoxr")?;
        // SAFETY: soxr_version has this signature and returns static C text.
        let version = unsafe {
            let version =
                library.get::<unsafe extern "C" fn() -> *const c_char>(b"soxr_version\0")?;
            let value = version();
            ensure!(!value.is_null(), "missing libsoxr version");
            CStr::from_ptr(value).to_str()?.to_owned()
        };
        ensure!(version.starts_with("libsoxr-0.1."), "unsupported {version}");
        eprintln!("offline resampler: {version}");
        Ok(Self(library))
    }

    pub(super) fn stereo(&self, input: &[[f64; 2]], rate: f64) -> Result<Vec<[f32; 2]>> {
        ensure!(
            !input.is_empty() && input.len() <= 48_000 * 120,
            "invalid source length"
        );
        ensure!(
            rate.is_finite() && (8000.0..=192_000.0).contains(&rate),
            "invalid source rate"
        );
        ensure!(
            input.iter().flatten().all(|v| v.is_finite()),
            "non-finite source"
        );
        let capacity = (input.len() as f64 * 48_000.0 / rate) as usize + 1;
        let mut output = vec![[0.0_f64; 2]; capacity];
        // SAFETY: the signatures/layouts match soxr.h. Arrays are contiguous
        // interleaved f64 stereo; lengths are FRAME counts, buffers disjoint.
        // Every offset/returned count is bounded before advancing pointers.
        // All symbols and the stream guard die before self's Library can drop.
        unsafe {
            let io = self
                .0
                .get::<unsafe extern "C" fn(c_int, c_int) -> Io>(b"soxr_io_spec\0")?(
                1, 1
            );
            let quality = self
                .0
                .get::<unsafe extern "C" fn(c_ulong, c_ulong) -> Quality>(b"soxr_quality_spec\0")?(
                6, 0,
            );
            let create = self.0.get::<Create>(b"soxr_create\0")?;
            let process = self.0.get::<Process>(b"soxr_process\0")?;
            let delete = *self.0.get::<Delete>(b"soxr_delete\0")?;
            let mut error = ptr::null();
            let handle = create(
                rate,
                48_000.0,
                2,
                &raw mut error,
                &raw const io,
                &raw const quality,
                ptr::null(),
            );
            let stream = (!handle.is_null()).then(|| Stream { handle, delete });
            check(error)?;
            let stream = stream.context("null libsoxr stream")?;
            let chunk = rate.max(1000.0) as usize;
            let mut written = 0;
            for source in input.chunks(chunk) {
                let (mut consumed, mut produced) = (0, 0);
                check(process(
                    stream.handle,
                    source.as_ptr().cast(),
                    source.len(),
                    &raw mut consumed,
                    output.as_mut_ptr().add(written).cast(),
                    capacity - written,
                    &raw mut produced,
                ))?;
                ensure!(
                    consumed == source.len() && produced <= capacity - written,
                    "libsoxr frame accounting"
                );
                written += produced;
            }
            let mut produced = 0;
            check(process(
                stream.handle,
                ptr::null(),
                0,
                ptr::null_mut(),
                output.as_mut_ptr().add(written).cast(),
                capacity - written,
                &raw mut produced,
            ))?;
            ensure!(produced <= capacity - written, "libsoxr flush overflow");
            output.truncate(written + produced);
        }
        ensure!(
            output.iter().flatten().all(|x| x.is_finite()),
            "non-finite resampling output"
        );
        Ok(output
            .into_iter()
            .map(|[l, r]| [l as f32, r as f32])
            .collect())
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn explicit_native_soxr_stereo_and_guards() {
        let Some(path) = std::env::var_os("BAYLEE_TEST_SOXR") else {
            return;
        };
        let resampler = super::Resampler::open(std::path::Path::new(&path)).unwrap();
        assert!(resampler.stereo(&[], 48000.0).is_err());
        assert!(resampler.stereo(&[[0.0; 2]], f64::NAN).is_err());
        assert!(resampler.stereo(&[[f64::INFINITY; 2]], 48000.0).is_err());
        for rate in [8000.0, 44100.0, 48000.0, 96000.0, 192_000.0] {
            let mut input = vec![[0.0; 2]; 4096];
            input[100][0] = 0.5;
            let output = resampler.stereo(&input, rate).unwrap();
            assert!(output.iter().all(|x| x[1] == 0.0));
            assert!(output.iter().any(|x| x[0].abs() > 0.01));
            assert!((output.len() as f64 - 4096.0 * 48000.0 / rate).abs() <= 1.0);
        }
    }
}
