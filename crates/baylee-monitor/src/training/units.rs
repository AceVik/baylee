//! The `baylee-*` system units, read from systemd over D-Bus, and the ones
//! that finished while Pulse was watching.
//!
//! systemd forgets a transient (`systemd-run`) unit as soon as it exits
//! successfully, so a finished run is remembered here, in memory, from the
//! last time it was seen. Unit descriptions are dropped unread: for a
//! `systemd-run` unit they are its command line.

use super::dbus::{Arg, Bus, Reader, SYSTEM_BUS};
use serde::Serialize;
use std::{collections::VecDeque, io, path::Path};

const PATTERN: &str = "baylee-*";
const SYSTEMD: &str = "org.freedesktop.systemd1";
const UNIT: &str = "org.freedesktop.systemd1.Unit";
const SERVICE: &str = "org.freedesktop.systemd1.Service";
const MAX_UNITS: usize = 24;
const KEEP_FINISHED: usize = 8;
const FINISHED_FOR_MS: u64 = 24 * 3600 * 1000;

#[derive(Clone, Debug, PartialEq, Serialize)]
pub(crate) struct Unit {
    pub name: String,
    /// systemd's `ActiveState`, e.g. `active` or `failed`.
    pub active_state: String,
    /// systemd's `SubState`, e.g. `running` or `exited`.
    pub sub_state: String,
    /// A service's `Result`, e.g. `success` or `exit-code`.
    pub result: Option<String>,
    pub main_pid: Option<u32>,
    pub active_since_ms: Option<u64>,
    pub inactive_since_ms: Option<u64>,
    /// When Pulse noticed that systemd no longer knows the unit.
    pub gone_ms: Option<u64>,
}

#[derive(Default)]
pub(crate) struct Units {
    bus: Option<Bus>,
    current: Vec<Unit>,
    finished: VecDeque<Unit>,
}

impl Units {
    /// Asks systemd again. On failure the previous view is kept and the
    /// connection is reopened on the next call.
    pub fn refresh(&mut self, now_ms: u64) -> io::Result<()> {
        let mut bus = match self.bus.take() {
            Some(bus) => bus,
            None => Bus::connect(Path::new(SYSTEM_BUS))?,
        };
        let listed = query(&mut bus)?;
        self.bus = Some(bus);
        self.update(listed, now_ms);
        Ok(())
    }

    fn update(&mut self, listed: Vec<Unit>, now_ms: u64) {
        for unit in self.current.drain(..) {
            if !listed.iter().any(|u| u.name == unit.name) {
                self.finished.push_front(Unit {
                    gone_ms: Some(now_ms),
                    ..unit
                });
            }
        }
        self.current = listed;
        self.finished.retain(|u| {
            u.gone_ms
                .is_some_and(|gone| now_ms.saturating_sub(gone) <= FINISHED_FOR_MS)
        });
        self.finished.truncate(KEEP_FINISHED);
    }

    /// Units systemd knows now, then the ones it has forgotten, newest first.
    pub fn view(&self) -> Vec<Unit> {
        self.current.iter().chain(&self.finished).cloned().collect()
    }
}

fn query(bus: &mut Bus) -> io::Result<Vec<Unit>> {
    let reply = bus.call(
        SYSTEMD,
        "/org/freedesktop/systemd1",
        "org.freedesktop.systemd1.Manager",
        "ListUnitsByPatterns",
        &[Arg::Strs(&[]), Arg::Strs(&[PATTERN])],
    )?;
    expect(&reply.signature, "a(ssssssouso)")?;
    let mut listed = Vec::new();
    reply.reader().array(8, |r| {
        r.begin_struct()?;
        let name = r.string()?;
        r.string()?; // Description: a systemd-run unit's command line.
        let load_state = r.string()?;
        let active_state = r.string()?;
        let sub_state = r.string()?;
        r.string()?; // Followed unit.
        let path = r.string()?; // Object path.
        r.u32()?; // Job id, type and path.
        r.string()?;
        r.string()?;
        if load_state != "not-found" && listed.len() < MAX_UNITS {
            listed.push((
                name.to_owned(),
                active_state.to_owned(),
                sub_state.to_owned(),
                path.to_owned(),
            ));
        }
        Ok(())
    })?;
    let mut units = Vec::with_capacity(listed.len());
    for (name, active_state, sub_state, path) in listed {
        let service = name.ends_with(".service");
        units.push(Unit {
            active_since_ms: timestamp(property(
                bus,
                &path,
                UNIT,
                "ActiveEnterTimestamp",
                u64_value,
            )?),
            inactive_since_ms: timestamp(property(
                bus,
                &path,
                UNIT,
                "InactiveEnterTimestamp",
                u64_value,
            )?),
            result: if service {
                Some(property(bus, &path, SERVICE, "Result", string_value)?)
            } else {
                None
            },
            main_pid: if service {
                Some(property(bus, &path, SERVICE, "MainPID", u32_value)?).filter(|&pid| pid != 0)
            } else {
                None
            },
            name,
            active_state,
            sub_state,
            gone_ms: None,
        });
    }
    units.sort_by(|a, b| {
        (a.active_state != "active")
            .cmp(&(b.active_state != "active"))
            .then(b.active_since_ms.cmp(&a.active_since_ms))
            .then_with(|| a.name.cmp(&b.name))
    });
    Ok(units)
}

/// systemd timestamps are microseconds since the epoch; zero means never.
fn timestamp(micros: u64) -> Option<u64> {
    (micros > 0).then_some(micros / 1000)
}

fn expect(signature: &str, wanted: &str) -> io::Result<()> {
    if signature == wanted {
        Ok(())
    } else {
        Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("systemd replied with {signature:?}, expected {wanted:?}"),
        ))
    }
}

/// Reads one property through `org.freedesktop.DBus.Properties.Get`.
fn property<T>(
    bus: &mut Bus,
    path: &str,
    interface: &str,
    name: &str,
    read: fn(&mut Reader<'_>, &str) -> io::Result<T>,
) -> io::Result<T> {
    let reply = bus.call(
        SYSTEMD,
        path,
        "org.freedesktop.DBus.Properties",
        "Get",
        &[Arg::Str(interface), Arg::Str(name)],
    )?;
    expect(&reply.signature, "v")?;
    let mut reader = reply.reader();
    let signature = reader.signature()?;
    read(&mut reader, signature)
}

fn u64_value(reader: &mut Reader<'_>, signature: &str) -> io::Result<u64> {
    expect(signature, "t")?;
    reader.u64()
}

fn u32_value(reader: &mut Reader<'_>, signature: &str) -> io::Result<u32> {
    expect(signature, "u")?;
    reader.u32()
}

fn string_value(reader: &mut Reader<'_>, signature: &str) -> io::Result<String> {
    expect(signature, "s")?;
    Ok(reader.string()?.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unit(name: &str, state: &str) -> Unit {
        Unit {
            name: name.into(),
            active_state: state.into(),
            sub_state: "running".into(),
            result: Some("success".into()),
            main_pid: Some(42),
            active_since_ms: Some(1_000),
            inactive_since_ms: None,
            gone_ms: None,
        }
    }

    #[test]
    fn remembers_units_systemd_forgot_for_a_day() {
        let mut units = Units::default();
        units.update(
            vec![
                unit("baylee-a.service", "active"),
                unit("baylee-b.service", "active"),
            ],
            10,
        );
        units.update(vec![unit("baylee-b.service", "active")], 20);
        let view = units.view();
        assert_eq!(view.len(), 2);
        assert_eq!(view[0].name, "baylee-b.service");
        assert_eq!(view[0].gone_ms, None);
        assert_eq!(view[1].name, "baylee-a.service");
        assert_eq!(view[1].gone_ms, Some(20));
        // A restarted unit is current again; its earlier run stays history.
        units.update(vec![unit("baylee-a.service", "active")], 30);
        let names: Vec<_> = units
            .view()
            .iter()
            .map(|u| (u.name.clone(), u.gone_ms))
            .collect();
        assert_eq!(
            names,
            [
                ("baylee-a.service".into(), None),
                ("baylee-b.service".into(), Some(30)),
                ("baylee-a.service".into(), Some(20)),
            ]
        );
        units.update(vec![], 20 + FINISHED_FOR_MS + 1);
        assert_eq!(units.view().len(), 2, "the oldest finished run expired");
        for n in 0..20 {
            units.update(
                vec![unit(&format!("baylee-{n}.service"), "active")],
                40 + FINISHED_FOR_MS,
            );
        }
        assert_eq!(units.view().len(), 1 + KEEP_FINISHED);
    }

    #[test]
    fn timestamps_of_zero_mean_never() {
        assert_eq!(timestamp(0), None);
        assert_eq!(timestamp(1_793_695_273_407_243), Some(1_793_695_273_407));
    }

    #[test]
    #[ignore = "talks to this machine's system bus"]
    fn reads_baylee_units_from_systemd() {
        let mut units = Units::default();
        units.refresh(0).unwrap();
        for unit in units.view() {
            assert!(unit.name.starts_with("baylee-"), "{unit:?}");
        }
    }
}
