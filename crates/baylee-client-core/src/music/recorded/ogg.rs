//! A bounded reader for the Ogg-Opus files we ship (RFC 3533, RFC 7845).
//!
//! It walks the pages once, checks every page's CRC, and indexes the audio
//! packets so the decoder can be fed one at a time. Every length it reads is
//! checked against what is left of the buffer; malformed input is an error,
//! never a panic. A packet that continues across a page boundary is not one
//! contiguous byte range in the file, so it is copied once, at load, into
//! [`Stream::joined`].

/// What is wrong with a buffer that is not one of our Ogg-Opus files.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Malformed {
    /// It ends inside a page or a packet.
    Truncated,
    /// A page does not start with `OggS` version 0.
    Capture,
    /// A page's checksum does not match its bytes.
    Checksum,
    /// A second logical stream, or a page that should continue a packet and
    /// does not (or the other way round).
    Framing,
    /// The `OpusHead` is missing or not stereo, mapping family 0.
    Head,
    /// An audio packet is empty, too long or has an impossible TOC.
    Packet,
    /// The final granule position does not fit the packets' duration.
    Length,
}

/// The largest audio packet accepted: 120 ms of Opus at the highest bitrate
/// is far below this.
const MAX_PACKET: usize = 1 << 16;
/// The largest header packet (`OpusTags`) accepted.
const MAX_HEADER: usize = 1 << 20;
/// The most samples one Opus packet holds per channel: 120 ms at 48 kHz.
pub(crate) const MAX_FRAMES: usize = 5760;

/// Where an audio packet's bytes are.
#[derive(Clone, Copy, Debug)]
struct Packet {
    start: u32,
    len: u32,
    /// In [`Stream::joined`] rather than in the file.
    joined: bool,
}

/// One indexed Ogg-Opus stream.
#[derive(Debug)]
pub(crate) struct Stream<'a> {
    data: &'a [u8],
    joined: Vec<u8>,
    packets: Vec<Packet>,
    /// Samples (per channel, at 48 kHz) the decoder emits before the music
    /// starts (`OpusHead` pre-skip).
    pub(crate) pre_skip: usize,
    /// The playable length in samples per channel: the last granule
    /// position less the pre-skip (RFC 7845 §4.4 end trimming).
    pub(crate) length: usize,
    /// Every packet's decoded length added up, pre-skip included.
    pub(crate) decoded: usize,
    /// The header's output gain, as an amplitude factor.
    pub(crate) gain: f32,
}

impl Stream<'static> {
    /// A stream with nothing in it: a length of 0 never yields a frame.
    pub(crate) const fn silent() -> Self {
        Self {
            data: &[],
            joined: Vec::new(),
            packets: Vec::new(),
            pre_skip: 0,
            length: 0,
            decoded: 0,
            gain: 1.0,
        }
    }
}

impl<'a> Stream<'a> {
    /// Indexes a whole file.
    pub(crate) fn parse(data: &'a [u8]) -> Result<Self, Malformed> {
        let mut stream = Self {
            data,
            joined: Vec::new(),
            packets: Vec::new(),
            pre_skip: 0,
            length: 0,
            decoded: 0,
            gain: 1.0,
        };
        let mut at = 0usize;
        let mut serial = None;
        // Bytes of a packet that started on an earlier page.
        let mut carry: Vec<u8> = Vec::new();
        let mut carrying = false;
        let mut headers = 0u8;
        let mut granule = None;
        while at < data.len() {
            let page = Page::read(data, at)?;
            if *serial.get_or_insert(page.serial) != page.serial {
                return Err(Malformed::Framing);
            }
            if page.continued != carrying {
                return Err(Malformed::Framing);
            }
            let mut body = page.body;
            let mut start = None::<usize>;
            let mut len = 0usize;
            for &lace in page.lacing {
                let lace = usize::from(lace);
                start.get_or_insert(body);
                len += lace;
                body += lace;
                if lace < 255 {
                    let from = start.take().unwrap_or(body);
                    if carrying {
                        carry.extend_from_slice(&data[from..from + len]);
                        stream.finish(Piece::Joined(&carry), &mut headers)?;
                        carry.clear();
                        carrying = false;
                    } else {
                        stream.finish(Piece::Whole(from, len), &mut headers)?;
                    }
                    len = 0;
                }
            }
            if let Some(from) = start {
                // The page ends inside a packet.
                if carry.len() + len > MAX_HEADER {
                    return Err(Malformed::Packet);
                }
                carry.extend_from_slice(&data[from..from + len]);
                carrying = true;
            }
            if page.granule != u64::MAX {
                granule = Some(page.granule);
            }
            at = page.end;
        }
        if carrying || headers < 2 || stream.packets.is_empty() {
            return Err(Malformed::Truncated);
        }
        let granule =
            usize::try_from(granule.ok_or(Malformed::Length)?).map_err(|_| Malformed::Length)?;
        stream.length = granule
            .checked_sub(stream.pre_skip)
            .filter(|&length| {
                length > 0 && length <= stream.decoded.saturating_sub(stream.pre_skip)
            })
            .ok_or(Malformed::Length)?;
        Ok(stream)
    }

    fn finish(&mut self, piece: Piece<'_>, headers: &mut u8) -> Result<(), Malformed> {
        let data = self.data;
        let bytes = match piece {
            Piece::Whole(from, len) => &data[from..from + len],
            Piece::Joined(bytes) => bytes,
        };
        match *headers {
            0 => {
                self.head(bytes)?;
                *headers = 1;
                return Ok(());
            }
            1 => {
                if !bytes.starts_with(b"OpusTags") {
                    return Err(Malformed::Head);
                }
                *headers = 2;
                return Ok(());
            }
            _ => {}
        }
        if bytes.is_empty() || bytes.len() > MAX_PACKET {
            return Err(Malformed::Packet);
        }
        self.decoded += duration(bytes).ok_or(Malformed::Packet)?;
        let packet = match piece {
            Piece::Whole(from, len) => Packet {
                start: u32::try_from(from).map_err(|_| Malformed::Packet)?,
                len: u32::try_from(len).map_err(|_| Malformed::Packet)?,
                joined: false,
            },
            Piece::Joined(bytes) => {
                let start = u32::try_from(self.joined.len()).map_err(|_| Malformed::Packet)?;
                self.joined.extend_from_slice(bytes);
                Packet {
                    start,
                    len: u32::try_from(bytes.len()).map_err(|_| Malformed::Packet)?,
                    joined: true,
                }
            }
        };
        self.packets.push(packet);
        Ok(())
    }

    fn head(&mut self, bytes: &[u8]) -> Result<(), Malformed> {
        // Magic, version, channels, pre-skip, input rate, gain, family.
        if bytes.len() < 19 || !bytes.starts_with(b"OpusHead") || bytes[8] >> 4 != 0 {
            return Err(Malformed::Head);
        }
        if bytes[9] != 2 || bytes[18] != 0 {
            return Err(Malformed::Head);
        }
        self.pre_skip = usize::from(u16::from_le_bytes([bytes[10], bytes[11]]));
        // Q7.8 decibels (RFC 7845 §5.1).
        let gain = i16::from_le_bytes([bytes[16], bytes[17]]);
        self.gain = 10f32.powf(f32::from(gain) / (256.0 * 20.0));
        Ok(())
    }

    /// How many audio packets it holds.
    #[cfg(test)]
    pub(crate) fn packets(&self) -> usize {
        self.packets.len()
    }

    /// One audio packet's bytes.
    pub(crate) fn packet(&self, index: usize) -> Option<&[u8]> {
        let packet = self.packets.get(index)?;
        let from = packet.start as usize;
        let to = from + packet.len as usize;
        if packet.joined {
            self.joined.get(from..to)
        } else {
            self.data.get(from..to)
        }
    }
}

#[derive(Clone, Copy)]
enum Piece<'b> {
    Whole(usize, usize),
    Joined(&'b [u8]),
}

/// One page's header, read and checked.
struct Page<'a> {
    serial: u32,
    continued: bool,
    granule: u64,
    lacing: &'a [u8],
    /// Where the body starts.
    body: usize,
    /// Where the next page starts.
    end: usize,
}

impl<'a> Page<'a> {
    fn read(data: &'a [u8], at: usize) -> Result<Self, Malformed> {
        let header = data.get(at..at + 27).ok_or(Malformed::Truncated)?;
        if &header[..4] != b"OggS" || header[4] != 0 {
            return Err(Malformed::Capture);
        }
        let word = |from: usize| {
            [
                header[from],
                header[from + 1],
                header[from + 2],
                header[from + 3],
            ]
        };
        let segments = usize::from(header[26]);
        let lacing = data
            .get(at + 27..at + 27 + segments)
            .ok_or(Malformed::Truncated)?;
        let body = at + 27 + segments;
        let size: usize = lacing.iter().map(|&lace| usize::from(lace)).sum();
        let end = body + size;
        if end > data.len() {
            return Err(Malformed::Truncated);
        }
        let stored = u32::from_le_bytes(word(22));
        if crc(&data[at..end], 22) != stored {
            return Err(Malformed::Checksum);
        }
        let mut granule = [0u8; 8];
        granule.copy_from_slice(&header[6..14]);
        Ok(Self {
            serial: u32::from_le_bytes(word(14)),
            continued: header[5] & 1 == 1,
            granule: u64::from_le_bytes(granule),
            lacing,
            body,
            end,
        })
    }
}

/// The Ogg CRC-32 (polynomial 0x04C11DB7, unreflected, initial 0) of a
/// page, its own four checksum bytes at `hole` read as zero.
fn crc(page: &[u8], hole: usize) -> u32 {
    const TABLE: [u32; 256] = {
        let mut table = [0u32; 256];
        let mut i = 0;
        while i < 256 {
            #[allow(clippy::cast_possible_truncation)] // i < 256
            let mut r = (i as u32) << 24;
            let mut bit = 0;
            while bit < 8 {
                r = if r & 0x8000_0000 == 0 {
                    r << 1
                } else {
                    (r << 1) ^ 0x04C1_1DB7
                };
                bit += 1;
            }
            table[i] = r;
            i += 1;
        }
        table
    };
    page.iter().enumerate().fold(0u32, |crc, (i, &byte)| {
        let byte = if (hole..hole + 4).contains(&i) {
            0
        } else {
            byte
        };
        (crc << 8) ^ TABLE[usize::from((crc >> 24) as u8 ^ byte)]
    })
}

/// How many samples per channel at 48 kHz an Opus packet holds, read from
/// its TOC byte (RFC 6716 §3.1); `None` for an impossible one.
pub(crate) fn duration(packet: &[u8]) -> Option<usize> {
    let toc = *packet.first()?;
    let config = toc >> 3;
    let frame = match config {
        0..=11 => [480, 960, 1920, 2880][usize::from(config % 4)],
        12..=15 => [480, 960][usize::from(config % 2)],
        _ => [120, 240, 480, 960][usize::from(config % 4)],
    };
    let count = match toc & 3 {
        0 => 1,
        1 | 2 => 2,
        _ => usize::from(*packet.get(1)? & 0x3F),
    };
    let total = frame * count;
    (count > 0 && total <= MAX_FRAMES).then_some(total)
}
