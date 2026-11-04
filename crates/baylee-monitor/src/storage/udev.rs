//! `/run/udev/data/b<major>:<minor>`, the database behind `/dev/disk/by-label`
//! and `by-partlabel`: world-readable, and it knows a partition's label and
//! filesystem whether or not it is mounted. Serial numbers and ids in the same
//! file stay unparsed.

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct Udev {
    pub fs_type: Option<String>,
    /// The filesystem's label.
    pub label: Option<String>,
    /// The GPT partition's own name.
    pub part_name: Option<String>,
}

pub(super) fn parse(text: &str) -> Udev {
    let mut udev = Udev::default();
    let mut plain_label = None;
    for line in text.lines() {
        let Some((key, value)) = line.strip_prefix("E:").and_then(|v| v.split_once('=')) else {
            continue;
        };
        if value.is_empty() {
            continue;
        }
        match key {
            "ID_FS_TYPE" => udev.fs_type = Some(value.to_owned()),
            // `ID_FS_LABEL` has spaces replaced; the encoded one keeps them.
            "ID_FS_LABEL_ENC" => udev.label = Some(decode(value)),
            "ID_FS_LABEL" => plain_label = Some(decode(value)),
            "ID_PART_ENTRY_NAME" => udev.part_name = Some(decode(value)),
            _ => {}
        }
    }
    udev.label = udev.label.or(plain_label);
    udev
}

/// Undoes udev's `\x20` escapes, also in the names under `/dev/disk/by-label`.
fn decode(value: &str) -> String {
    let bytes = value.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut at = 0;
    while at < bytes.len() {
        let code = bytes
            .get(at + 2..at + 4)
            .filter(|_| bytes[at..].starts_with(br"\x"))
            .and_then(|digits| std::str::from_utf8(digits).ok())
            .and_then(|digits| u8::from_str_radix(digits, 16).ok());
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
    fn reads_label_type_and_partition_name_and_nothing_else() {
        let text = "S:disk/by-label/xdata\nI:123\nE:ID_SERIAL=SECRET123\nE:ID_FS_TYPE=ext4\n\
                    E:ID_FS_LABEL=my_data\nE:ID_FS_LABEL_ENC=my\\x20data\n\
                    E:ID_PART_ENTRY_NAME=Basic\\x20data\\x20partition\nG:systemd\n";
        assert_eq!(
            parse(text),
            Udev {
                fs_type: Some("ext4".into()),
                label: Some("my data".into()),
                part_name: Some("Basic data partition".into()),
            }
        );
    }

    #[test]
    fn an_encoded_label_is_optional() {
        assert_eq!(
            parse("E:ID_FS_LABEL=SYSTEM\n").label.as_deref(),
            Some("SYSTEM")
        );
    }

    #[test]
    fn empty_values_mean_absent_and_multibyte_names_decode() {
        assert_eq!(parse("E:ID_FS_LABEL=\nE:ID_FS_TYPE=\n"), Udev::default());
        assert_eq!(decode(r"caf\xc3\xa9"), "café");
        assert_eq!(decode(r"bad\xzz"), r"bad\xzz");
    }
}
