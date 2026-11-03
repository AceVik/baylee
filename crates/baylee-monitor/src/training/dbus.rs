//! A minimal, read-only D-Bus client: just enough to ask systemd about units.
//!
//! It authenticates as the service's own user (SASL `EXTERNAL`), sends method
//! calls whose arguments are strings or string arrays, and parses the replies.
//! It subscribes to no signals and has no way to call a method with other
//! arguments; which methods are called is fixed in `units.rs`, and each is a
//! read. Messages are bounded in size and every socket operation has a
//! timeout, so a stuck bus cannot hold the training thread.

use std::{
    fmt::Write as _,
    io::{self, Read, Write},
    os::unix::{fs::MetadataExt, net::UnixStream},
    path::Path,
    time::Duration,
};

pub(crate) const SYSTEM_BUS: &str = "/run/dbus/system_bus_socket";
const MAX_MESSAGE: usize = 1 << 20;
const TIMEOUT: Duration = Duration::from_secs(2);
const METHOD_CALL: u8 = 1;
const METHOD_RETURN: u8 = 2;
const ERROR: u8 = 3;

/// One argument of a method call.
pub(crate) enum Arg<'a> {
    Str(&'a str),
    Strs(&'a [&'a str]),
}

pub(crate) struct Bus {
    stream: UnixStream,
    serial: u32,
}

/// A method's return value: its signature and body.
pub(crate) struct Reply {
    pub signature: String,
    body: Vec<u8>,
    big_endian: bool,
}

impl Reply {
    pub fn reader(&self) -> Reader<'_> {
        Reader {
            buf: &self.body,
            pos: 0,
            big_endian: self.big_endian,
        }
    }
}

impl Bus {
    pub fn connect(path: &Path) -> io::Result<Self> {
        let mut stream = UnixStream::connect(path)?;
        stream.set_read_timeout(Some(TIMEOUT))?;
        stream.set_write_timeout(Some(TIMEOUT))?;
        // The bus checks the claimed uid against the socket's peer credentials.
        let uid = std::fs::metadata("/proc/self")?.uid();
        let mut command = String::from("\0AUTH EXTERNAL ");
        for byte in uid.to_string().bytes() {
            let _ = write!(command, "{byte:02x}");
        }
        command.push_str("\r\n");
        stream.write_all(command.as_bytes())?;
        let answer = read_line(&mut stream)?;
        if !answer.starts_with("OK ") {
            return Err(invalid(format!("authentication refused: {answer}")));
        }
        stream.write_all(b"BEGIN\r\n")?;
        let mut bus = Self { stream, serial: 0 };
        bus.call(
            "org.freedesktop.DBus",
            "/org/freedesktop/DBus",
            "org.freedesktop.DBus",
            "Hello",
            &[],
        )?;
        Ok(bus)
    }

    pub fn call(
        &mut self,
        destination: &str,
        path: &str,
        interface: &str,
        member: &str,
        args: &[Arg<'_>],
    ) -> io::Result<Reply> {
        self.serial = self.serial.wrapping_add(1).max(1);
        let serial = self.serial;
        let message = encode_call(serial, destination, path, interface, member, args);
        self.stream.write_all(&message)?;
        // Skip anything that is not our reply, such as `NameAcquired`.
        for _ in 0..64 {
            let message = read_message(&mut self.stream)?;
            if message.reply_serial != Some(serial) {
                continue;
            }
            return match message.kind {
                METHOD_RETURN => Ok(Reply {
                    signature: message.signature,
                    body: message.body,
                    big_endian: message.big_endian,
                }),
                ERROR => Err(io::Error::other(
                    message
                        .error_name
                        .unwrap_or_else(|| "D-Bus error".to_owned()),
                )),
                _ => continue,
            };
        }
        Err(invalid("no reply".to_owned()))
    }
}

fn invalid(message: String) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}

fn truncated() -> io::Error {
    invalid("truncated D-Bus message".to_owned())
}

/// Reads one `\r\n`-terminated line of the authentication exchange.
fn read_line(stream: &mut impl Read) -> io::Result<String> {
    let mut line = Vec::new();
    let mut byte = [0];
    while !line.ends_with(b"\r\n") {
        if line.len() > 512 {
            return Err(invalid("authentication line too long".to_owned()));
        }
        stream.read_exact(&mut byte)?;
        line.push(byte[0]);
    }
    line.truncate(line.len() - 2);
    Ok(String::from_utf8_lossy(&line).into_owned())
}

/// Little-endian marshalling; positions are relative to an 8-aligned start.
#[derive(Default)]
struct Writer {
    buf: Vec<u8>,
}

impl Writer {
    fn pad(&mut self, align: usize) {
        while !self.buf.len().is_multiple_of(align) {
            self.buf.push(0);
        }
    }

    fn u32(&mut self, value: u32) {
        self.pad(4);
        self.buf.extend_from_slice(&value.to_le_bytes());
    }

    fn string(&mut self, value: &str) {
        self.u32(value.len() as u32);
        self.buf.extend_from_slice(value.as_bytes());
        self.buf.push(0);
    }

    fn signature(&mut self, value: &str) {
        self.buf.push(value.len() as u8);
        self.buf.extend_from_slice(value.as_bytes());
        self.buf.push(0);
    }

    fn strings(&mut self, values: &[&str]) {
        self.u32(0);
        let start = self.buf.len();
        for value in values {
            self.string(value);
        }
        let length = (self.buf.len() - start) as u32;
        self.buf[start - 4..start].copy_from_slice(&length.to_le_bytes());
    }

    /// One `(byte, variant)` header field holding a string-like value.
    fn field(&mut self, code: u8, signature: &str, value: &str) {
        self.pad(8);
        self.buf.push(code);
        self.signature(signature);
        if signature == "g" {
            self.signature(value);
        } else {
            self.string(value);
        }
    }
}

fn encode_call(
    serial: u32,
    destination: &str,
    path: &str,
    interface: &str,
    member: &str,
    args: &[Arg<'_>],
) -> Vec<u8> {
    let mut body = Writer::default();
    let mut signature = String::new();
    for arg in args {
        match arg {
            Arg::Str(value) => {
                signature.push('s');
                body.string(value);
            }
            Arg::Strs(values) => {
                signature.push_str("as");
                body.strings(values);
            }
        }
    }
    let mut message = Writer::default();
    message.buf.extend_from_slice(&[b'l', METHOD_CALL, 0, 1]);
    message.u32(body.buf.len() as u32);
    message.u32(serial);
    message.u32(0); // Header field array length, patched below.
    let start = message.buf.len();
    message.field(1, "o", path);
    message.field(2, "s", interface);
    message.field(3, "s", member);
    message.field(6, "s", destination);
    if !signature.is_empty() {
        message.field(8, "g", &signature);
    }
    let length = (message.buf.len() - start) as u32;
    message.buf[12..16].copy_from_slice(&length.to_le_bytes());
    message.pad(8);
    message.buf.extend_from_slice(&body.buf);
    message.buf
}

struct Message {
    kind: u8,
    reply_serial: Option<u32>,
    error_name: Option<String>,
    signature: String,
    body: Vec<u8>,
    big_endian: bool,
}

fn read_message(stream: &mut impl Read) -> io::Result<Message> {
    let mut fixed = [0; 16];
    stream.read_exact(&mut fixed)?;
    let big_endian = match fixed[0] {
        b'l' => false,
        b'B' => true,
        _ => return Err(invalid("not a D-Bus message".to_owned())),
    };
    let word = |at: usize| {
        let bytes = [fixed[at], fixed[at + 1], fixed[at + 2], fixed[at + 3]];
        if big_endian {
            u32::from_be_bytes(bytes) as usize
        } else {
            u32::from_le_bytes(bytes) as usize
        }
    };
    let (body_length, fields_length) = (word(4), word(12));
    let fields_end = 16 + fields_length;
    let header_length = fields_end.next_multiple_of(8);
    let total = header_length
        .checked_add(body_length)
        .filter(|&total| total <= MAX_MESSAGE)
        .ok_or_else(|| invalid("D-Bus message too large".to_owned()))?;
    let mut buf = vec![0; total];
    buf[..16].copy_from_slice(&fixed);
    stream.read_exact(&mut buf[16..])?;
    let mut fields = Reader {
        buf: &buf[..fields_end],
        pos: 16,
        big_endian,
    };
    let (mut reply_serial, mut error_name, mut signature) = (None, None, String::new());
    while fields.pos < fields.buf.len() {
        fields.align(8)?;
        let code = fields.u8()?;
        match (code, fields.signature()?) {
            (5, "u") => reply_serial = Some(fields.u32()?),
            (4, "s") => error_name = Some(fields.string()?.to_owned()),
            (8, "g") => fields.signature()?.clone_into(&mut signature),
            (_, "s" | "o") => _ = fields.string()?,
            (_, "u") => _ = fields.u32()?,
            (_, "g") => _ = fields.signature()?,
            _ => return Err(invalid("unsupported D-Bus header field".to_owned())),
        }
    }
    Ok(Message {
        kind: fixed[1],
        reply_serial,
        error_name,
        signature,
        body: buf.split_off(header_length),
        big_endian,
    })
}

/// Unmarshals values; positions are relative to an 8-aligned start.
pub(crate) struct Reader<'a> {
    buf: &'a [u8],
    pos: usize,
    big_endian: bool,
}

impl<'a> Reader<'a> {
    fn align(&mut self, align: usize) -> io::Result<()> {
        let pos = self.pos.next_multiple_of(align);
        if pos > self.buf.len() {
            return Err(truncated());
        }
        self.pos = pos;
        Ok(())
    }

    fn take(&mut self, count: usize) -> io::Result<&'a [u8]> {
        let end = self
            .pos
            .checked_add(count)
            .filter(|&end| end <= self.buf.len())
            .ok_or_else(truncated)?;
        let bytes = &self.buf[self.pos..end];
        self.pos = end;
        Ok(bytes)
    }

    fn u8(&mut self) -> io::Result<u8> {
        Ok(self.take(1)?[0])
    }

    pub fn u32(&mut self) -> io::Result<u32> {
        self.align(4)?;
        let mut bytes = [0; 4];
        bytes.copy_from_slice(self.take(4)?);
        Ok(if self.big_endian {
            u32::from_be_bytes(bytes)
        } else {
            u32::from_le_bytes(bytes)
        })
    }

    pub fn u64(&mut self) -> io::Result<u64> {
        self.align(8)?;
        let mut bytes = [0; 8];
        bytes.copy_from_slice(self.take(8)?);
        Ok(if self.big_endian {
            u64::from_be_bytes(bytes)
        } else {
            u64::from_le_bytes(bytes)
        })
    }

    /// A string or an object path; both are encoded alike.
    pub fn string(&mut self) -> io::Result<&'a str> {
        let length = self.u32()? as usize;
        self.text(length)
    }

    pub fn signature(&mut self) -> io::Result<&'a str> {
        let length = usize::from(self.u8()?);
        self.text(length)
    }

    fn text(&mut self, length: usize) -> io::Result<&'a str> {
        let bytes = self.take(length)?;
        if self.u8()? != 0 {
            return Err(invalid("unterminated D-Bus string".to_owned()));
        }
        std::str::from_utf8(bytes).map_err(|_| invalid("D-Bus string is not UTF-8".to_owned()))
    }

    /// Reads an array whose elements need `align`, calling `each` per element.
    pub fn array(
        &mut self,
        align: usize,
        mut each: impl FnMut(&mut Self) -> io::Result<()>,
    ) -> io::Result<()> {
        let length = self.u32()? as usize;
        self.align(align)?;
        let end = self
            .pos
            .checked_add(length)
            .filter(|&end| end <= self.buf.len())
            .ok_or_else(truncated)?;
        while self.pos < end {
            let before = self.pos;
            each(self)?;
            if self.pos == before {
                return Err(invalid("empty D-Bus array element".to_owned()));
            }
        }
        if self.pos == end {
            Ok(())
        } else {
            Err(invalid("D-Bus array overrun".to_owned()))
        }
    }

    pub fn begin_struct(&mut self) -> io::Result<()> {
        self.align(8)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encodes_a_call_its_own_reader_can_parse() {
        let message = encode_call(
            7,
            "org.freedesktop.systemd1",
            "/org/freedesktop/systemd1",
            "org.freedesktop.systemd1.Manager",
            "ListUnitsByPatterns",
            &[Arg::Strs(&[]), Arg::Strs(&["baylee-*", "x"])],
        );
        assert_eq!(&message[..4], &[b'l', METHOD_CALL, 0, 1]);
        assert_eq!(u32::from_le_bytes(message[8..12].try_into().unwrap()), 7);
        let parsed = read_message(&mut message.as_slice()).unwrap();
        assert_eq!(parsed.kind, METHOD_CALL);
        assert_eq!(parsed.signature, "asas");
        assert_eq!(parsed.reply_serial, None);
        let mut reader = Reader {
            buf: &parsed.body,
            pos: 0,
            big_endian: false,
        };
        let mut arrays = Vec::new();
        for _ in 0..2 {
            let mut items = Vec::new();
            reader
                .array(4, |r| {
                    items.push(r.string()?.to_owned());
                    Ok(())
                })
                .unwrap();
            arrays.push(items);
        }
        assert_eq!(
            arrays,
            [vec![], vec!["baylee-*".to_owned(), "x".to_owned()]]
        );
        assert_eq!(reader.pos, parsed.body.len());
    }

    #[test]
    fn hello_matches_the_specification_layout() {
        // `Hello`, serial 1, laid out by hand from the specification.
        let message = encode_call(
            1,
            "org.freedesktop.DBus",
            "/org/freedesktop/DBus",
            "org.freedesktop.DBus",
            "Hello",
            &[],
        );
        let mut expected = vec![b'l', 1, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, 109, 0, 0, 0];
        expected.extend_from_slice(&[1, 1, b'o', 0, 21, 0, 0, 0]);
        expected.extend_from_slice(b"/org/freedesktop/DBus\0");
        expected.extend_from_slice(&[0, 0]);
        expected.extend_from_slice(&[2, 1, b's', 0, 20, 0, 0, 0]);
        expected.extend_from_slice(b"org.freedesktop.DBus\0");
        expected.extend_from_slice(&[0, 0, 0]);
        expected.extend_from_slice(&[3, 1, b's', 0, 5, 0, 0, 0]);
        expected.extend_from_slice(b"Hello\0");
        expected.extend_from_slice(&[0, 0]);
        expected.extend_from_slice(&[6, 1, b's', 0, 20, 0, 0, 0]);
        expected.extend_from_slice(b"org.freedesktop.DBus\0");
        expected.extend_from_slice(&[0, 0, 0]);
        assert_eq!(message, expected);
    }

    #[test]
    fn reads_big_endian_replies_and_rejects_oversized_or_broken_ones() {
        // A big-endian METHOD_RETURN for serial 3 with signature "t": header
        // fields end at 31, so one byte pads the body to offset 32.
        let message: Vec<u8> = [
            b'B',
            METHOD_RETURN,
            0,
            1,
            0,
            0,
            0,
            8,
            0,
            0,
            0,
            9,
            0,
            0,
            0,
            15,
        ]
        .into_iter()
        .chain([5, 1, b'u', 0, 0, 0, 0, 3])
        .chain([8, 1, b'g', 0, 1, b't', 0, 0])
        .chain(1_793_695_273_000_000_u64.to_be_bytes())
        .collect();
        let parsed = read_message(&mut message.as_slice()).unwrap();
        assert_eq!(parsed.kind, METHOD_RETURN);
        assert_eq!(parsed.reply_serial, Some(3));
        assert_eq!(parsed.signature, "t");
        let reply = Reply {
            signature: parsed.signature,
            body: parsed.body,
            big_endian: true,
        };
        assert_eq!(reply.reader().u64().unwrap(), 1_793_695_273_000_000);

        let mut huge = vec![b'l', METHOD_RETURN, 0, 1];
        huge.extend_from_slice(&(MAX_MESSAGE as u32).to_le_bytes());
        huge.extend_from_slice(&[1, 0, 0, 0, 0, 0, 0, 0]);
        assert!(read_message(&mut huge.as_slice()).is_err());
        assert!(read_message(&mut &b"GET / HTTP/1.1\r\n"[..]).is_err());
        let mut short = Reader {
            buf: &[3, 0, 0, 0, b'a'],
            pos: 0,
            big_endian: false,
        };
        assert!(short.string().is_err());
    }

    #[test]
    #[ignore = "talks to this machine's system bus"]
    fn lists_units_on_the_system_bus() {
        let mut bus = Bus::connect(Path::new(SYSTEM_BUS)).unwrap();
        let reply = bus
            .call(
                "org.freedesktop.systemd1",
                "/org/freedesktop/systemd1",
                "org.freedesktop.systemd1.Manager",
                "ListUnitsByPatterns",
                &[Arg::Strs(&["active"]), Arg::Strs(&["*.target"])],
            )
            .unwrap();
        assert_eq!(reply.signature, "a(ssssssouso)");
    }
}
