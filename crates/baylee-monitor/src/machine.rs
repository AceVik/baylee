//! Which machine a page belongs to: its model from the firmware's tables, and
//! the other Pulse pages a viewer may jump to. `product_serial` and
//! `product_uuid` sit beside these files and are never opened.

use serde::Serialize;
use std::{fs, path::Path};

pub(crate) const DMI: &str = "/sys/class/dmi/id";
pub(crate) const MAX_PEERS: usize = 4;
const MAX_PEER_NAME: usize = 32;

/// What a board vendor leaves in a field nobody filled in.
const PLACEHOLDERS: [&str; 8] = [
    "system product name",
    "system version",
    "default string",
    "to be filled by o.e.m.",
    "not specified",
    "not applicable",
    "none",
    "unknown",
];

fn field(dmi: &Path, file: &str) -> Option<String> {
    let value = fs::read_to_string(dmi.join(file)).ok()?;
    let value = value.trim();
    let filled = !value.is_empty() && !PLACEHOLDERS.contains(&value.to_lowercase().as_str());
    filled.then(|| value.to_owned())
}

/// A model a person recognises. A notebook may keep a part number in
/// `product_name` and its name elsewhere; a self-built PC has only its board.
pub(crate) fn model(dmi: &Path) -> Option<String> {
    field(dmi, "product_family")
        .or_else(|| field(dmi, "product_version"))
        .or_else(|| field(dmi, "product_name"))
        .or_else(|| field(dmi, "board_name"))
}

/// Another Pulse page, as `--peer` named it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub(crate) struct Peer {
    pub name: String,
    pub url: String,
}

/// Parses one `--peer NAME=URL`. The URL becomes a link on a page without a
/// login, so only a plain `http://host:port` is taken: no path that could
/// carry anything, no credentials, and no script.
pub(crate) fn parse_peer(value: &str) -> Result<Peer, String> {
    let (name, url) = value
        .split_once('=')
        .ok_or("a peer is written NAME=http://HOST:PORT")?;
    let plain = |c: char| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.');
    if name.is_empty() || name.len() > MAX_PEER_NAME || !name.chars().all(plain) {
        return Err(format!(
            "a peer's name is 1 to {MAX_PEER_NAME} letters, digits, '-', '_' or '.'"
        ));
    }
    let authority = url
        .strip_prefix("http://")
        .ok_or("a peer's URL starts with http://")?;
    let authority = authority.strip_suffix('/').unwrap_or(authority);
    let (host, port) = authority
        .rsplit_once(':')
        .ok_or("a peer's URL names a port")?;
    let bracketed = host.starts_with('[') && host.ends_with(']');
    if host.is_empty() || !(bracketed || host.chars().all(plain)) || port.parse::<u16>().is_err() {
        return Err("a peer's URL is http://HOST:PORT and nothing more".into());
    }
    Ok(Peer {
        name: name.to_owned(),
        url: format!("http://{authority}"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::Scratch;

    #[test]
    fn a_thinkpad_is_known_by_its_family_and_not_its_part_number() {
        let scratch = Scratch::new("dmi-thinkpad");
        scratch.write("dmi/product_name", "21K50003GE\n");
        scratch.write("dmi/product_version", "ThinkPad P14s Gen 4\n");
        scratch.write("dmi/product_family", "ThinkPad P14s Gen 4\n");
        assert_eq!(
            model(&scratch.path("dmi")).as_deref(),
            Some("ThinkPad P14s Gen 4")
        );
    }

    #[test]
    fn a_self_built_pc_falls_back_to_its_board() {
        let scratch = Scratch::new("dmi-board");
        scratch.write("dmi/product_name", "System Product Name\n");
        scratch.write("dmi/product_version", "System Version\n");
        scratch.write("dmi/product_family", "To be filled by O.E.M.\n");
        scratch.write("dmi/board_name", "ROG CROSSHAIR VIII HERO (WI-FI)\n");
        assert_eq!(
            model(&scratch.path("dmi")).as_deref(),
            Some("ROG CROSSHAIR VIII HERO (WI-FI)")
        );
    }

    #[test]
    fn a_machine_without_tables_has_no_model() {
        let scratch = Scratch::new("dmi-none");
        assert_eq!(model(&scratch.path("dmi")), None);
    }

    #[test]
    fn a_peer_is_a_name_and_a_bare_address() {
        assert_eq!(
            parse_peer("acenb=http://192.168.0.12:8787"),
            Ok(Peer {
                name: "acenb".into(),
                url: "http://192.168.0.12:8787".into(),
            })
        );
        assert_eq!(
            parse_peer("big-ace=http://bigace.local:8787/").unwrap().url,
            "http://bigace.local:8787"
        );
        assert_eq!(
            parse_peer("v6=http://[fd00::1]:8787").unwrap().url,
            "http://[fd00::1]:8787"
        );
    }

    #[test]
    fn anything_but_a_bare_address_is_refused() {
        for bad in [
            "acenb",
            "=http://192.168.0.12:8787",
            "a b=http://192.168.0.12:8787",
            "acenb=192.168.0.12:8787",
            "acenb=https://192.168.0.12:8787",
            "acenb=javascript:alert(1)",
            "acenb=http://192.168.0.12",
            "acenb=http://192.168.0.12:port",
            "acenb=http://192.168.0.12:8787/path",
            "acenb=http://user:secret@192.168.0.12:8787",
            "acenb=http://evil.example/?x=:8787",
        ] {
            assert!(parse_peer(bad).is_err(), "{bad}");
        }
        let long = format!("{}=http://192.168.0.12:8787", "n".repeat(33));
        assert!(parse_peer(&long).is_err());
    }
}
