use baylee_protocol::v1::Envelope;
use baylee_tower::Refusal;

/// One synchronous driver call may retain at most eight MiB before yielding to
/// the persistence boundary. Exhaustion suppresses every resulting player frame.
pub(super) const MAX_BATCH_BYTES: usize = 8 * 1024 * 1024;

#[derive(Default)]
pub(super) struct Capture {
    pieces: Vec<Vec<u8>>,
    bytes: usize,
    refused: bool,
}

impl crate::recording::Transport for Capture {
    fn chunk(&mut self, _game: &str, _seq: u32, data: Vec<u8>, _last: bool) -> Option<Envelope> {
        if self.refused {
            return None;
        }
        if data.len() > MAX_BATCH_BYTES.saturating_sub(self.bytes) {
            self.pieces.clear();
            self.bytes = 0;
            self.refused = true;
        } else if !data.is_empty() {
            self.bytes += data.len();
            self.pieces.push(data);
        }
        None
    }
}

impl Capture {
    pub(super) fn take(&mut self) -> Result<Vec<Vec<u8>>, Refusal> {
        if self.refused {
            return Err(Refusal::Capacity);
        }
        self.bytes = 0;
        Ok(std::mem::take(&mut self.pieces))
    }
}
