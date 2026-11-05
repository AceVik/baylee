//! `/proc/net/sockstat` and `sockstat6`: sockets in use on the whole machine.

use serde::Serialize;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
pub(crate) struct Sockets {
    pub tcp: u32,
    pub udp: u32,
    pub time_wait: u32,
}

/// Adds one file's counts to `sockets`, so the IPv4 and IPv6 files sum up.
pub(crate) fn add(sockets: &mut Sockets, text: &str) {
    for line in text.lines() {
        let Some((protocol, rest)) = line.split_once(':') else {
            continue;
        };
        let value = |wanted: &str| {
            let mut fields = rest.split_whitespace();
            while let Some(key) = fields.next() {
                let value = fields.next()?;
                if key == wanted {
                    return value.parse::<u32>().ok();
                }
            }
            None
        };
        match protocol {
            "TCP" | "TCP6" => {
                sockets.tcp += value("inuse").unwrap_or(0);
                sockets.time_wait += value("tw").unwrap_or(0);
            }
            "UDP" | "UDP6" => sockets.udp += value("inuse").unwrap_or(0),
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn both_address_families_sum_up() {
        let mut sockets = Sockets::default();
        add(
            &mut sockets,
            "sockets: used 1234\nTCP: inuse 12 orphan 0 tw 3 alloc 14 mem 2\n\
             UDP: inuse 4 mem 1\nUDPLITE: inuse 0\nRAW: inuse 0\n",
        );
        add(
            &mut sockets,
            "TCP6: inuse 5\nUDP6: inuse 6\nRAW6: inuse 1\n",
        );
        assert_eq!(
            sockets,
            Sockets {
                tcp: 17,
                udp: 10,
                time_wait: 3,
            }
        );
    }
}
