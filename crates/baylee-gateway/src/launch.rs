//! How the gateway is started: what its command line may say, and which
//! address it listens on.
//!
//! Every setting is an environment variable (`docs/dev/environment.md`). The
//! command line names a subcommand, asks for help, or says nothing; anything
//! else stops the program before it reads a setting, opens the database or
//! binds a port. An argument it does not know is a mistake, and the answer
//! to a mistake is to say so, not to serve with whatever the shell held.

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};

/// The port the gateway listens on when `PORT` names none.
pub(crate) const DEFAULT_PORT: u16 = 28766;

/// The address the gateway listens on when `BAYLEE_BIND` names none: this
/// machine only.
///
/// A public gateway sits behind a TLS proxy on the same machine, which dials
/// it on loopback (`scripts/server/play.caddy`), and its agent and engines
/// come in on the unix socket. Every other interface is the operator's to
/// open, by naming it (`BAYLEE_BIND=0.0.0.0`), never a default.
pub(crate) const DEFAULT_BIND: IpAddr = IpAddr::V4(Ipv4Addr::LOCALHOST);

/// What `baylee-gateway --help` prints, and a refused command line after its
/// reason.
pub(crate) const USAGE: &str = "\
usage: baylee-gateway                       serve; every setting is an environment variable
       baylee-gateway records export --out <dir>
                                            export the anonymised game records
       baylee-gateway legacy export         export the accounts for Central's cutover
       baylee-gateway help | --help | -h    this text

It listens on 127.0.0.1:28766, this machine only, unless told otherwise:
PORT names the port (0 = any free one) and BAYLEE_BIND the address, a
numeric IP (0.0.0.0 or :: = every interface). DATABASE_URL is required.
Every variable: docs/dev/environment.md.";

/// What a command line asks the gateway for.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Command<'a> {
    /// No argument: serve.
    Serve,
    /// `help`, `--help` or `-h`: print [`USAGE`], start nothing.
    Help,
    /// The closed beta's key command, which is gone
    /// ([`crate::registration::INVITE_COMMAND_GONE`]).
    Invite,
    /// `records …`, with the arguments after the word.
    Records(&'a [String]),
    /// `legacy …`, with the arguments after the word.
    Legacy(&'a [String]),
}

/// Reads a command line, without the program's name.
///
/// Only the first argument is read here; a subcommand checks the rest itself.
///
/// # Errors
/// A first argument that is no subcommand and no request for help, named in
/// the reason up to an `=` (what follows one may be a secret). Nothing has
/// been read or opened by then.
pub(crate) fn command(args: &[String]) -> Result<Command<'_>, String> {
    let Some((first, rest)) = args.split_first() else {
        return Ok(Command::Serve);
    };
    match first.as_str() {
        "help" | "--help" | "-h" => Ok(Command::Help),
        "invite" => Ok(Command::Invite),
        "records" => Ok(Command::Records(rest)),
        "legacy" => Ok(Command::Legacy(rest)),
        other => {
            let name = other.split_once('=').map_or(other, |(name, _)| name);
            Err(format!(
                "{name:?} is not an argument this program takes; its settings are environment \
                 variables"
            ))
        }
    }
}

/// Stops on a command line the gateway does not take: the reason and
/// [`USAGE`] on standard error, exit status 2.
pub(crate) fn refuse(why: &str) -> ! {
    eprintln!("baylee-gateway: {why}\n\n{USAGE}");
    std::process::exit(2);
}

/// The address `BAYLEE_BIND` names, or [`DEFAULT_BIND`] when it is unset or
/// empty.
///
/// # Errors
/// A value that is not a numeric IP address, so that a typo stops the start
/// with the other settings, before the database is opened.
pub(crate) fn bind_address(value: Option<&str>) -> Result<IpAddr, String> {
    match value.filter(|v| !v.is_empty()) {
        None => Ok(DEFAULT_BIND),
        Some(value) => value.parse().map_err(|_| {
            format!(
                "BAYLEE_BIND: {value:?} is not a numeric IP address (unset is {DEFAULT_BIND}; \
                 0.0.0.0 or :: listens on every interface)"
            )
        }),
    }
}

/// What an engine on this machine is told to dial when `BAYLEE_ENGINE_URL`
/// names nothing: the address the gateway is bound to, or loopback when it
/// is bound to every interface.
pub(crate) fn engine_url(bind: IpAddr, port: u16) -> String {
    let dial = match bind {
        IpAddr::V4(v4) if v4.is_unspecified() => IpAddr::V4(Ipv4Addr::LOCALHOST),
        IpAddr::V6(v6) if v6.is_unspecified() => IpAddr::V6(Ipv6Addr::LOCALHOST),
        bound => bound,
    };
    format!("ws://{}/engine/ws", SocketAddr::new(dial, port))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(args: &[&str]) -> Vec<String> {
        args.iter().map(ToString::to_string).collect()
    }

    #[test]
    fn help_is_help_and_nothing_else_serves() {
        assert_eq!(command(&[]), Ok(Command::Serve));
        for help in ["help", "--help", "-h"] {
            assert_eq!(command(&line(&[help])), Ok(Command::Help), "{help}");
            assert_eq!(
                command(&line(&[help, "anything"])),
                Ok(Command::Help),
                "{help}"
            );
        }
    }

    #[test]
    fn an_argument_it_does_not_take_is_refused_by_name() {
        for unknown in ["--version", "--bind", "serve", "-", "", "--HELP", "-help"] {
            let refused = command(&line(&[unknown])).expect_err(unknown);
            assert!(refused.contains(&format!("{unknown:?}")), "{refused}");
        }
        // A flag is named up to its `=`, and what follows is not repeated.
        let refused = command(&line(&["--admin-token=s3cret"])).unwrap_err();
        assert!(refused.contains("\"--admin-token\""), "{refused}");
        assert!(!refused.contains("s3cret"), "{refused}");
        // An unknown argument before a known one is still refused.
        assert!(command(&line(&["--bogus", "--help"])).is_err());
        assert!(command(&line(&["--bogus", "records"])).is_err());
    }

    #[test]
    fn subcommands_keep_their_own_arguments() {
        let records = line(&["records", "export", "--out", "/tmp/r"]);
        assert_eq!(command(&records), Ok(Command::Records(&records[1..])));
        let legacy = line(&["legacy", "export"]);
        assert_eq!(command(&legacy), Ok(Command::Legacy(&legacy[1..])));
        assert_eq!(command(&line(&["invite", "list"])), Ok(Command::Invite));
    }

    /// Unset, the gateway is reachable from this machine alone; every other
    /// interface is opened by naming it.
    #[test]
    fn the_default_address_is_loopback_and_never_every_interface() {
        for unset in [None, Some("")] {
            let bind = bind_address(unset).unwrap();
            assert!(bind.is_loopback(), "{unset:?} → {bind}");
            assert!(!bind.is_unspecified(), "{unset:?} → {bind}");
        }
        assert_eq!(DEFAULT_BIND, IpAddr::V4(Ipv4Addr::LOCALHOST));
        assert!(USAGE.contains("127.0.0.1:28766"));
        assert_eq!(DEFAULT_PORT, 28766);
    }

    #[test]
    fn every_interface_is_what_an_operator_names() {
        assert_eq!(
            bind_address(Some("0.0.0.0")),
            Ok(IpAddr::V4(Ipv4Addr::UNSPECIFIED))
        );
        assert_eq!(
            bind_address(Some("::")),
            Ok(IpAddr::V6(Ipv6Addr::UNSPECIFIED))
        );
        assert_eq!(
            bind_address(Some("192.168.0.5")),
            Ok(IpAddr::V4(Ipv4Addr::new(192, 168, 0, 5)))
        );
        assert_eq!(
            bind_address(Some("::1")),
            Ok(IpAddr::V6(Ipv6Addr::LOCALHOST))
        );
    }

    #[test]
    fn a_bind_that_is_not_an_address_stops_the_start() {
        for bad in [
            "localhost",
            "0.0.0.0:28766",
            " 127.0.0.1",
            "127.0.0.1 ",
            "*",
            "[::]",
        ] {
            let refused = bind_address(Some(bad)).expect_err(bad);
            assert!(refused.starts_with("BAYLEE_BIND"), "{refused}");
        }
    }

    #[test]
    fn engines_dial_the_bound_address_or_loopback() {
        assert_eq!(
            engine_url(DEFAULT_BIND, 28766),
            "ws://127.0.0.1:28766/engine/ws"
        );
        assert_eq!(
            engine_url(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 1234),
            "ws://127.0.0.1:1234/engine/ws"
        );
        assert_eq!(
            engine_url(IpAddr::V6(Ipv6Addr::UNSPECIFIED), 1234),
            "ws://[::1]:1234/engine/ws"
        );
        assert_eq!(
            engine_url(IpAddr::V6(Ipv6Addr::LOCALHOST), 1234),
            "ws://[::1]:1234/engine/ws"
        );
        assert_eq!(
            engine_url(IpAddr::V4(Ipv4Addr::new(10, 0, 0, 2)), 1234),
            "ws://10.0.0.2:1234/engine/ws"
        );
    }
}
