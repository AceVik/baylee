//! Safe wrappers over the C shim in `shim/xrt_shim.cpp`.
//!
//! One [`Npu`] is one hardware context for one xclbin. Buffer objects and
//! runs borrow it, so they cannot outlive the context they were made for.

use std::ffi::{CStr, CString, c_char, c_int, c_void};
use std::marker::PhantomData;
use std::path::Path;
use std::ptr::NonNull;

#[repr(C)]
struct RawCtx {
    _p: [u8; 0],
}
#[repr(C)]
struct RawBo {
    _p: [u8; 0],
}
#[repr(C)]
struct RawRun {
    _p: [u8; 0],
}

unsafe extern "C" {
    fn npu_open(
        xclbin: *const c_char,
        kernel_prefix: *const c_char,
        err: *mut c_char,
        cap: usize,
    ) -> *mut RawCtx;
    fn npu_close(c: *mut RawCtx);
    fn npu_bo_alloc(
        c: *mut RawCtx,
        bytes: usize,
        instr: c_int,
        arg: c_int,
        err: *mut c_char,
        cap: usize,
    ) -> *mut RawBo;
    fn npu_bo_map(b: *mut RawBo) -> *mut c_void;
    fn npu_bo_sync(b: *mut RawBo, to_device: c_int, err: *mut c_char, cap: usize) -> c_int;
    fn npu_bo_free(b: *mut RawBo);
    fn npu_run_new(
        c: *mut RawCtx,
        instr: *mut RawBo,
        ninstr: u32,
        a: *mut RawBo,
        b: *mut RawBo,
        out: *mut RawBo,
        err: *mut c_char,
        cap: usize,
    ) -> *mut RawRun;
    fn npu_run_exec(r: *mut RawRun, err: *mut c_char, cap: usize) -> c_int;
    fn npu_run_free(r: *mut RawRun);
    fn npu_cmd_state_completed() -> c_int;
}

const ERR_CAP: usize = 1024;

/// A buffer the shim writes an error message into.
struct ErrBuf([c_char; ERR_CAP]);

impl ErrBuf {
    fn new() -> Self {
        Self([0; ERR_CAP])
    }
    fn ptr(&mut self) -> *mut c_char {
        self.0.as_mut_ptr()
    }
    fn message(&self, what: &str) -> String {
        // SAFETY: the buffer starts zeroed and the shim always writes a
        // NUL-terminated string shorter than ERR_CAP, so a NUL is in bounds.
        let msg = unsafe { CStr::from_ptr(self.0.as_ptr()) };
        format!("{what}: {}", msg.to_string_lossy())
    }
}

/// A hardware context on NPU 0 for one xclbin, with its kernel.
pub struct Npu {
    raw: NonNull<RawCtx>,
}

impl Npu {
    /// Opens device 0, registers `xclbin` and makes a hardware context for
    /// the first kernel whose name starts with `kernel_prefix`.
    pub fn open(xclbin: &Path, kernel_prefix: &str) -> Result<Self, String> {
        let path = CString::new(xclbin.as_os_str().as_encoded_bytes())
            .map_err(|e| format!("xclbin path: {e}"))?;
        let prefix = CString::new(kernel_prefix).map_err(|e| format!("kernel prefix: {e}"))?;
        let mut err = ErrBuf::new();
        // SAFETY: both strings are NUL-terminated and live across the call;
        // `err` holds ERR_CAP bytes.
        let raw = unsafe { npu_open(path.as_ptr(), prefix.as_ptr(), err.ptr(), ERR_CAP) };
        NonNull::new(raw)
            .map(|raw| Self { raw })
            .ok_or_else(|| err.message("open NPU context"))
    }

    /// A host-only data BO in the memory group of kernel argument `arg`.
    pub fn data_bo(&self, bytes: usize, arg: i32) -> Result<Bo<'_>, String> {
        self.bo(bytes, false, arg)
    }

    /// The cacheable instruction BO (kernel argument 1).
    pub fn instr_bo(&self, bytes: usize) -> Result<Bo<'_>, String> {
        self.bo(bytes, true, 1)
    }

    fn bo(&self, bytes: usize, instr: bool, arg: i32) -> Result<Bo<'_>, String> {
        let mut err = ErrBuf::new();
        // SAFETY: `self.raw` is a live context; `err` holds ERR_CAP bytes.
        let raw = unsafe {
            npu_bo_alloc(
                self.raw.as_ptr(),
                bytes,
                c_int::from(instr),
                arg,
                err.ptr(),
                ERR_CAP,
            )
        };
        let raw = NonNull::new(raw).ok_or_else(|| err.message("allocate BO"))?;
        // SAFETY: `raw` is a BO the shim just allocated.
        let map = unsafe { npu_bo_map(raw.as_ptr()) };
        let Some(map) = NonNull::new(map.cast::<u8>()) else {
            // SAFETY: `raw` is live and not used after this.
            unsafe { npu_bo_free(raw.as_ptr()) };
            return Err("map BO: XRT returned no mapping".to_owned());
        };
        Ok(Bo {
            raw,
            map,
            bytes,
            _ctx: PhantomData,
        })
    }

    /// A reusable run of the kernel with
    /// `(opcode 3, instr, ninstr, a, b, out)` as its arguments.
    pub fn run<'a>(
        &'a self,
        instr: &'a Bo<'a>,
        ninstr: u32,
        a: &'a Bo<'a>,
        b: &'a Bo<'a>,
        out: &'a Bo<'a>,
    ) -> Result<Run<'a>, String> {
        let mut err = ErrBuf::new();
        // SAFETY: the context and all four BOs are live for 'a, which the
        // returned run borrows; `err` holds ERR_CAP bytes.
        let raw = unsafe {
            npu_run_new(
                self.raw.as_ptr(),
                instr.raw.as_ptr(),
                ninstr,
                a.raw.as_ptr(),
                b.raw.as_ptr(),
                out.raw.as_ptr(),
                err.ptr(),
                ERR_CAP,
            )
        };
        NonNull::new(raw)
            .map(|raw| Run {
                raw,
                _ctx: PhantomData,
            })
            .ok_or_else(|| err.message("create run"))
    }
}

impl Drop for Npu {
    fn drop(&mut self) {
        // SAFETY: `raw` came from npu_open and every borrower is gone.
        unsafe { npu_close(self.raw.as_ptr()) }
    }
}

/// A buffer object with its host mapping.
pub struct Bo<'a> {
    raw: NonNull<RawBo>,
    map: NonNull<u8>,
    bytes: usize,
    _ctx: PhantomData<&'a Npu>,
}

/// Which way a BO sync goes.
#[derive(Clone, Copy)]
pub enum Sync {
    ToDevice,
    FromDevice,
}

// The mapping is memory the NPU also reads and writes, so a `Bo` never
// lends out a Rust reference into it: every access below is a copy through
// the raw pointer, which keeps writing through `&self` sound while a `Run`
// holds the BO. `Bo` is neither `Send` nor `Sync` (it holds raw pointers),
// so no two threads touch one mapping. The host is little-endian (x86_64),
// so in-memory integers and floats are already the NPU's byte order.
impl Bo<'_> {
    fn copy_in<T: Copy>(&self, vals: &[T]) {
        let len = std::mem::size_of_val(vals);
        assert!(len <= self.bytes, "BO holds {} bytes, asked to write {len}", self.bytes);
        // SAFETY: XRT maps `self.bytes` bytes at `map` for the BO's lifetime,
        // `len` fits in them, and the source is a Rust slice, so the two
        // regions cannot overlap.
        unsafe { std::ptr::copy_nonoverlapping(vals.as_ptr().cast::<u8>(), self.map.as_ptr(), len) }
    }

    /// Copies `words` into the start of the mapping.
    pub fn write_u32(&self, words: &[u32]) {
        self.copy_in(words);
    }

    /// Copies `vals` into the start of the mapping.
    pub fn write_u16(&self, vals: &[u16]) {
        self.copy_in(vals);
    }

    /// Sets every byte of the mapping to `byte`.
    pub fn fill(&self, byte: u8) {
        // SAFETY: the mapping holds `self.bytes` bytes (see `copy_in`).
        unsafe { std::ptr::write_bytes(self.map.as_ptr(), byte, self.bytes) }
    }

    /// Reads the first `n` f32 values of the mapping.
    pub fn read_f32(&self, n: usize) -> Vec<f32> {
        assert!(n * 4 <= self.bytes, "BO holds {} bytes, asked to read {}", self.bytes, n * 4);
        let mut out = vec![0.0_f32; n];
        // SAFETY: `n * 4` bytes are mapped and `out` holds exactly that many
        // bytes; the copy is bytewise, so alignment does not matter, and
        // every bit pattern is a valid f32.
        unsafe { std::ptr::copy_nonoverlapping(self.map.as_ptr(), out.as_mut_ptr().cast::<u8>(), n * 4) }
        out
    }

    /// Syncs the BO's host and device views.
    pub fn sync(&self, dir: Sync) -> Result<(), String> {
        let mut err = ErrBuf::new();
        let to_device = c_int::from(matches!(dir, Sync::ToDevice));
        // SAFETY: `raw` is live; `err` holds ERR_CAP bytes.
        let rc = unsafe { npu_bo_sync(self.raw.as_ptr(), to_device, err.ptr(), ERR_CAP) };
        if rc == 0 { Ok(()) } else { Err(err.message("sync BO")) }
    }
}

impl Drop for Bo<'_> {
    fn drop(&mut self) {
        // SAFETY: `raw` came from npu_bo_alloc; runs that borrow it are gone.
        unsafe { npu_bo_free(self.raw.as_ptr()) }
    }
}

/// A kernel run whose arguments are bound; each [`Run::exec`] starts it and
/// waits for completion.
pub struct Run<'a> {
    raw: NonNull<RawRun>,
    _ctx: PhantomData<&'a Npu>,
}

impl Run<'_> {
    /// Starts the run and blocks until the NPU reports it done.
    pub fn exec(&self) -> Result<(), String> {
        let mut err = ErrBuf::new();
        // SAFETY: `raw` is live; `err` holds ERR_CAP bytes.
        let state = unsafe { npu_run_exec(self.raw.as_ptr(), err.ptr(), ERR_CAP) };
        // SAFETY: a constant query with no arguments.
        let done = unsafe { npu_cmd_state_completed() };
        match state {
            -1 => Err(err.message("run kernel")),
            s if s == done => Ok(()),
            s => Err(format!("run kernel: ended in ert_cmd_state {s}")),
        }
    }
}

impl Drop for Run<'_> {
    fn drop(&mut self) {
        // SAFETY: `raw` came from npu_run_new and is not running.
        unsafe { npu_run_free(self.raw.as_ptr()) }
    }
}
