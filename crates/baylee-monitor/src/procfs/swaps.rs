//! `/proc/swaps`: each swap file or partition.

use super::unescape_octal;
use serde::Serialize;

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub(crate) struct SwapDevice {
    /// The last path component only; a swap file's directory is nobody's
    /// business on a page without a login.
    pub name: String,
    /// `file` or `partition`.
    pub kind: String,
    pub size: u64,
    pub used: u64,
    pub priority: i32,
}

pub(crate) fn parse(text: &str) -> Vec<SwapDevice> {
    text.lines()
        .skip(1)
        .filter_map(|line| {
            let mut fields = line.split_whitespace();
            let path = unescape_octal(fields.next()?);
            let kind = fields.next()?.to_owned();
            let size: u64 = fields.next()?.parse().ok()?;
            let used: u64 = fields.next()?.parse().ok()?;
            Some(SwapDevice {
                name: path.rsplit('/').next().unwrap_or(&path).to_owned(),
                kind,
                size: size.saturating_mul(1024),
                used: used.saturating_mul(1024),
                priority: fields.next()?.parse().ok()?,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_only_the_last_path_component() {
        let text = "Filename\t\t\t\tType\t\tSize\t\tUsed\t\tPriority\n\
                    /home/someone/my\\040swap        file\t\t33554428\t32\t\t-1\n\
                    /dev/nvme1n1p3                  partition\t1024\t\t0\t\t5\n";
        assert_eq!(
            parse(text),
            [
                SwapDevice {
                    name: "my swap".into(),
                    kind: "file".into(),
                    size: 33_554_428 * 1024,
                    used: 32 * 1024,
                    priority: -1,
                },
                SwapDevice {
                    name: "nvme1n1p3".into(),
                    kind: "partition".into(),
                    size: 1024 * 1024,
                    used: 0,
                    priority: 5,
                },
            ]
        );
    }

    #[test]
    fn no_swap_means_only_the_heading() {
        assert!(parse("Filename\tType\tSize\tUsed\tPriority\n").is_empty());
        assert!(parse("").is_empty());
    }
}
