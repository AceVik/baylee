//! Parsers for the kernel's text files in `/proc`. Each parser takes the file's
//! text, so the tests feed it fixtures and nothing here needs a real system.
//! Unknown lines and fields are skipped: kernels add them freely.

pub(crate) mod diskstats;
pub(crate) mod meminfo;
pub(crate) mod mountinfo;
pub(crate) mod netdev;
pub(crate) mod pressure;
pub(crate) mod sockstat;
pub(crate) mod stat;
pub(crate) mod swaps;
pub(crate) mod vmstat;

/// The growth of a monotonic counter per second. A counter that went backwards
/// was reset (a re-created interface, a re-attached disk) and counts as zero.
#[allow(clippy::cast_precision_loss)] // A rate does not need exact bytes.
pub(crate) fn per_second(previous: u64, current: u64, elapsed_secs: f64) -> f64 {
    current.saturating_sub(previous) as f64 / elapsed_secs
}

/// Undoes the kernel's octal escapes (`\040` is a space) in a path or name.
pub(crate) fn unescape_octal(field: &str) -> String {
    let bytes = field.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut at = 0;
    while at < bytes.len() {
        let code = bytes
            .get(at + 1..at + 4)
            .filter(|_| bytes[at] == b'\\')
            .and_then(|digits| std::str::from_utf8(digits).ok())
            .and_then(|digits| u8::from_str_radix(digits, 8).ok());
        if let Some(code) = code {
            decoded.push(code);
            at += 4;
        } else {
            decoded.push(bytes[at]);
            at += 1;
        }
    }
    String::from_utf8_lossy(&decoded).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_counter_that_went_backwards_is_no_negative_rate() {
        assert!((per_second(100, 300, 2.0) - 100.0).abs() < f64::EPSILON);
        assert!(per_second(300, 100, 2.0).abs() < f64::EPSILON);
    }

    #[test]
    fn octal_escapes_decode_and_a_lone_backslash_stays() {
        assert_eq!(unescape_octal(r"/mnt/my\040disk"), "/mnt/my disk");
        assert_eq!(unescape_octal(r"/mnt/caf\303\251"), "/mnt/café");
        assert_eq!(unescape_octal(r"a\b"), r"a\b");
        assert_eq!(unescape_octal("plain"), "plain");
    }
}
