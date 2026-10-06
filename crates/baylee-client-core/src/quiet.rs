//! Lines for a terminal that may have gone away.
//!
//! `println!` and `eprintln!` panic when their stream cannot be written:
//! "failed printing to stderr: Broken pipe (os error 32)", a beta.5 crash
//! report. Rust ignores `SIGPIPE`, so a write to a pipe nobody reads any
//! more fails instead of ending the process, and the macros turn that
//! failure into a panic. A seat bridge whose client quit, a client started
//! by a launcher that was killed, a program piped into `head`: each writes
//! to a pipe that outlived its reader.
//!
//! Every line a program of ours writes on a path that can outlive its
//! reader goes through [`say!`](crate::say) (stdout) or
//! [`say_err!`](crate::say_err) (stderr), which write the line and drop a
//! failure: a line nobody can read any more is not worth a crash.

use std::io::Write;

/// Writes `text` and a newline to `out`, and flushes it, dropping any
/// failure: a reader that went away is not this program's to answer for.
pub fn line(out: &mut dyn Write, text: std::fmt::Arguments<'_>) {
    let _ = out
        .write_fmt(text)
        .and_then(|()| out.write_all(b"\n"))
        .and_then(|()| out.flush());
}

/// `println!` that never panics: a closed stdout drops the line.
#[macro_export]
macro_rules! say {
    ($($arg:tt)*) => {
        $crate::quiet::line(&mut ::std::io::stdout().lock(), format_args!($($arg)*))
    };
}

/// `eprintln!` that never panics: a closed stderr drops the line.
#[macro_export]
macro_rules! say_err {
    ($($arg:tt)*) => {
        $crate::quiet::line(&mut ::std::io::stderr().lock(), format_args!($($arg)*))
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A stream whose reader is gone: every write fails as a closed pipe's.
    struct Closed;

    impl Write for Closed {
        fn write(&mut self, _: &[u8]) -> std::io::Result<usize> {
            Err(std::io::ErrorKind::BrokenPipe.into())
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Err(std::io::ErrorKind::BrokenPipe.into())
        }
    }

    #[test]
    fn a_closed_pipe_drops_the_line_and_does_not_panic() {
        line(&mut Closed, format_args!("nobody reads {}", "this"));
        let mut kept = Vec::new();
        line(&mut kept, format_args!("somebody reads {}", 1));
        assert_eq!(kept, b"somebody reads 1\n");
    }

    /// The real streams, closed under the process, are the bridge's test
    /// (`baylee-seat`'s `tests/closed_pipes.rs`): this one holds the macros
    /// to the same writer.
    #[test]
    fn the_macros_write_a_line() {
        crate::say!("{}", "a line on stdout");
        crate::say_err!("{}", "a line on stderr");
    }
}
