//! Read only bounded routing metadata; all replay/rules evaluation stays in the engine.
use baylee_core::preset::LoopPolicy;
use baylee_protocol::execution::{Identity, Variant};
use serde::Deserialize;
use std::io::{BufRead as _, Read as _};

const MAX_HEADER_BYTES: usize = 2 * 1024 * 1024;

#[derive(Deserialize)]
struct Header {
    kind: String,
    preset: Preset,
    #[serde(default)]
    execution: Option<Identity>,
}

#[derive(Deserialize)]
struct Preset {
    #[serde(default)]
    house_rules: Rules,
}

#[derive(Default, Deserialize)]
struct Rules {
    #[serde(default)]
    loop_policy: LoopPolicy,
}

pub(super) fn variant(record: &[u8]) -> Option<Variant> {
    let reader = flate2::read::MultiGzDecoder::new(record).take((MAX_HEADER_BYTES + 1) as u64);
    let mut bytes = Vec::new();
    std::io::BufReader::new(reader)
        .read_until(b'\n', &mut bytes)
        .ok()?;
    if bytes.len() > MAX_HEADER_BYTES || bytes.last() != Some(&b'\n') {
        return None;
    }
    let header: Header = serde_json::from_slice(&bytes).ok()?;
    if header.kind != "header" {
        return None;
    }
    let variant = crate::engine::execution::standard_variant(header.preset.house_rules.loop_policy);
    if let Some(identity) = header.execution {
        if !identity.valid() {
            return None;
        }
        if identity.variant == Variant::Tower {
            return (variant == Variant::Standard).then_some(Variant::Tower);
        }
        if identity.variant != variant {
            return None;
        }
    }
    Some(variant)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write as _;
    fn compressed(bytes: &[u8]) -> Vec<u8> {
        let mut writer = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::fast());
        writer.write_all(bytes).unwrap();
        writer.finish().unwrap()
    }
    #[test]
    fn old_headers_keep_the_detector_and_off_headers_never_select_it() {
        assert_eq!(
            variant(&compressed(b"{\"kind\":\"header\",\"preset\":{}}\n")),
            Some(Variant::StandardLoops)
        );
        let header = serde_json::json!({"kind":"header","preset":{"house_rules":{"loop_policy":LoopPolicy::Off}}});
        assert_eq!(
            variant(&compressed(format!("{header}\n").as_bytes())),
            Some(Variant::Standard)
        );
        assert_eq!(variant(&compressed(format!("{header}").as_bytes())), None);
        assert_eq!(
            variant(&compressed(&vec![b' '; MAX_HEADER_BYTES + 1])),
            None
        );
        assert_eq!(variant(b"invalid gzip"), None);
    }
}
