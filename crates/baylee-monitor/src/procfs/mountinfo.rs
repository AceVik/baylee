//! `/proc/self/mountinfo`: the same table as `/proc/mounts`, plus each mount's
//! device number, which identifies a partition whatever `/dev` name was used
//! to mount it (`/dev/root`, `/dev/mapper/…`, `/dev/disk/by-uuid/…`).

use super::unescape_octal;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Mount {
    pub device: (u32, u32),
    /// The directory of the filesystem that is mounted; `/` unless a bind mount.
    pub root: String,
    pub path: String,
    pub read_only: bool,
    pub fs_type: String,
    pub source: String,
}

pub(crate) fn parse(text: &str) -> Vec<Mount> {
    text.lines().filter_map(parse_line).collect()
}

fn parse_line(line: &str) -> Option<Mount> {
    // `36 35 98:0 /root /mount rw,noatime master:1 - ext3 /dev/sda1 rw`: the
    // optional fields end at a lone dash.
    let (front, back) = line.split_once(" - ")?;
    let mut front = front.split(' ');
    let _mount_id = front.next()?;
    let _parent_id = front.next()?;
    let (major, minor) = front.next()?.split_once(':')?;
    let root = unescape_octal(front.next()?);
    let path = unescape_octal(front.next()?);
    let read_only = front.next()?.split(',').any(|option| option == "ro");
    let mut back = back.split(' ');
    Some(Mount {
        device: (major.parse().ok()?, minor.parse().ok()?),
        root,
        path,
        read_only,
        fs_type: back.next()?.to_owned(),
        source: unescape_octal(back.next()?),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_device_numbers_flags_and_escapes() {
        let text = "29 1 259:7 / / rw,relatime shared:1 - ext4 /dev/nvme1n1p2 rw\n\
                    90 29 259:2 / /mnt/my\\040c ro,relatime shared:5 master:2 - ntfs3 /dev/nvme0n1p2 ro,uid=1000\n\
                    91 29 0:25 / /sys rw,nosuid - sysfs sysfs rw\n";
        let mounts = parse(text);
        assert_eq!(mounts.len(), 3);
        assert_eq!(
            mounts[0],
            Mount {
                device: (259, 7),
                root: "/".into(),
                path: "/".into(),
                read_only: false,
                fs_type: "ext4".into(),
                source: "/dev/nvme1n1p2".into(),
            }
        );
        assert_eq!(mounts[1].path, "/mnt/my c");
        assert!(mounts[1].read_only);
        assert_eq!(mounts[1].fs_type, "ntfs3");
        assert_eq!(mounts[2].device, (0, 25));
    }

    #[test]
    fn a_read_only_word_inside_an_option_is_no_flag() {
        let mounts = parse("1 1 8:1 / /a rw,errors=remount-ro - ext4 /dev/sda1 rw\n");
        assert!(!mounts[0].read_only);
    }

    #[test]
    fn broken_lines_are_skipped() {
        assert!(parse("").is_empty());
        assert!(parse("no separator here\n1 2 x / / rw - ext4 /dev/a rw\n").is_empty());
    }
}
