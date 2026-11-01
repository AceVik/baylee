//! A losslessly packed stereo bank, prepared and pitched before the audio thread.
use std::{io::Read, sync::OnceLock};

const PACKED: [&[u8]; 4] = [
    include_bytes!("../../../../assets/chamber/stereo-01.pcm.zlib"),
    include_bytes!("../../../../assets/chamber/stereo-02.pcm.zlib"),
    include_bytes!("../../../../assets/chamber/stereo-03.pcm.zlib"),
    include_bytes!("../../../../assets/chamber/stereo-04.pcm.zlib"),
];
const LIMIT: usize = 256 * 1024 * 1024;

#[derive(Clone, Copy, Debug)]
pub(super) struct Clip {
    offset: usize,
    pub frames: usize,
}
pub(super) struct Bank {
    pcm: Vec<u8>,
    clips: Box<[[Option<Clip>; 256]]>,
}
fn word(bytes: &[u8], at: usize) -> usize {
    u32::from_le_bytes(bytes[at..at + 4].try_into().expect("bank word")) as usize
}
impl Bank {
    fn load() -> Self {
        let mut pcm = Vec::new();
        let packed = PACKED[0].chain(PACKED[1]).chain(PACKED[2]).chain(PACKED[3]);
        flate2::read::ZlibDecoder::new(packed)
            .take(LIMIT as u64 + 1)
            .read_to_end(&mut pcm)
            .expect("validated embedded stereo bank");
        assert!(pcm.len() <= LIMIT && &pcm[..4] == b"CHM1");
        assert_eq!(word(&pcm, 4), 48_000);
        pcm.shrink_to_fit();
        let mut clips = vec![[None; 256]; 3].into_boxed_slice();
        let mut at = 12;
        for _ in 0..word(&pcm, 8) {
            let instrument = usize::from(pcm[at]);
            let pitch = usize::from(pcm[at + 1]);
            let layer = usize::from(pcm[at + 2]) - 1;
            let frames = word(&pcm, at + 4);
            at += 8;
            assert!(frames > 0 && at + frames * 4 <= pcm.len());
            assert!(clips[instrument][pitch * 2 + layer].is_none());
            clips[instrument][pitch * 2 + layer] = Some(Clip { offset: at, frames });
            at += frames * 4;
        }
        assert_eq!(at, pcm.len());
        Self { pcm, clips }
    }
    pub(super) fn clip(&self, instrument: u8, pitch: u8, layer: usize) -> Option<Clip> {
        self.clips[usize::from(instrument)][usize::from(pitch) * 2 + layer]
    }
    #[inline]
    pub(super) fn sample(&self, clip: Clip, frame: usize) -> [f32; 2] {
        if frame >= clip.frames {
            return [0.0; 2];
        }
        let at = clip.offset + frame * 4;
        [
            f32::from(i16::from_le_bytes([self.pcm[at], self.pcm[at + 1]])) / 32768.0,
            f32::from(i16::from_le_bytes([self.pcm[at + 2], self.pcm[at + 3]])) / 32768.0,
        ]
    }
}
pub(super) fn bank() -> &'static Bank {
    static BANK: OnceLock<Bank> = OnceLock::new();
    BANK.get_or_init(Bank::load)
}

#[cfg(test)]
mod tests {
    use super::*;
    use sha2::{Digest, Sha256};
    use std::fmt::Write;

    fn hex(bytes: &[u8]) -> String {
        bytes.iter().fold(String::new(), |mut out, byte| {
            write!(out, "{byte:02x}").unwrap();
            out
        })
    }

    #[test]
    fn packed_recordings_match_the_manifest_and_keep_stereo() {
        let manifest: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../../../art/music/memorial/instruments.json"
        ))
        .unwrap();
        let mut hash = Sha256::new();
        for chunk in PACKED {
            hash.update(chunk);
        }
        assert_eq!(hex(&hash.finalize()), manifest["sha256"]);
        let bank = bank();
        assert_eq!(
            bank.pcm.len(),
            manifest["decoded_bytes"].as_u64().unwrap() as usize
        );
        for row in manifest["samples"].as_array().unwrap() {
            let clip = bank
                .clip(
                    row["instrument"].as_u64().unwrap() as u8,
                    row["pitch"].as_u64().unwrap() as u8,
                    row["layer"].as_u64().unwrap() as usize - 1,
                )
                .unwrap();
            let bytes = &bank.pcm[clip.offset..clip.offset + clip.frames * 4];
            assert_eq!(hex(&Sha256::digest(bytes)), row["pcm_sha256"]);
            assert!(
                bytes
                    .as_chunks::<4>()
                    .0
                    .iter()
                    .any(|frame| frame[..2] != frame[2..])
            );
            assert!(
                bank.sample(clip, clip.frames - 1)
                    .iter()
                    .all(|v| v.abs() < 0.0001)
            );
        }
    }
}
