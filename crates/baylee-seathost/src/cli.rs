//! The seat agent's command line: nothing, or a request for help.
//!
//! Every setting is an environment variable (`docs/dev/environment.md`), and
//! anything else on the command line stops the program before it reads one,
//! runs its bridge or dials a gateway.

/// What `baylee-seathost --help` prints, and a refused command line after
/// its reason.
pub const USAGE: &str = "\
usage: baylee-seathost                  dial the gateway and seat a bridge in each hosted chair
       baylee-seathost --help | -h      this text

It takes no argument and opens no port; its settings are environment
variables: BAYLEE_SEATHOST_TOKEN and BAYLEE_SEATHOST_STATE (required),
BAYLEE_GATEWAY (default http://127.0.0.1:28766, or unix:<path>),
BAYLEE_SEATHOST_BRIDGE_GATEWAY (required on unix:), BAYLEE_SEATHOST_NAME,
BAYLEE_SEATHOST_CAPACITY, BAYLEE_SEAT_BIN and BAYLEE_KEY_STORE=file:<dir>
(docs/llm-seat.md, \"A hosted seat\").";

/// What a command line asks the seat agent for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Asked {
    /// Nothing: run.
    Run,
    /// `--help` or `-h`: print [`USAGE`], start nothing.
    Help,
}

/// Reads a command line, without the program's name.
///
/// # Errors
/// Any argument but a request for help. The refusal names a flag up to its
/// `=` and repeats no value: a value may be a secret.
pub fn asked(args: &[String]) -> Result<Asked, String> {
    match args.first().map(String::as_str) {
        None => Ok(Asked::Run),
        Some("--help" | "-h") => Ok(Asked::Help),
        Some(flag) if flag.starts_with('-') => {
            let name = flag.split_once('=').map_or(flag, |(name, _)| name);
            Err(format!(
                "{name:?} is not a flag this program takes; its settings are environment \
                 variables"
            ))
        }
        Some(_) => Err(
            "this program takes no argument; its settings are environment variables".to_string(),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(args: &[&str]) -> Vec<String> {
        args.iter().map(ToString::to_string).collect()
    }

    #[test]
    fn it_runs_on_nothing_and_helps_on_help() {
        assert_eq!(asked(&[]), Ok(Asked::Run));
        assert_eq!(asked(&line(&["--help"])), Ok(Asked::Help));
        assert_eq!(asked(&line(&["-h"])), Ok(Asked::Help));
    }

    #[test]
    fn any_other_argument_is_refused_without_its_value() {
        for (args, said) in [
            (&["--version"][..], "\"--version\""),
            (&["--token=s3cret"][..], "\"--token\""),
            (&["--name", "main"][..], "\"--name\""),
            (&["s3cret"][..], "takes no argument"),
            (&["--bogus", "--help"][..], "\"--bogus\""),
        ] {
            let refused = asked(&line(args)).expect_err(said);
            assert!(refused.contains(said), "{args:?}: {refused}");
            assert!(!refused.contains("s3cret"), "{args:?}: {refused}");
        }
    }
}
