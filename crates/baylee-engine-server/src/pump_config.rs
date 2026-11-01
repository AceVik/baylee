//! Attached transport scheduling, separate from deterministic rules execution.
//!
//! One step exposes each house-pump answer. This does not split internal seat
//! automation inside an engine apply, or a socket answer followed by house work.
//! With two connected socket seats, every explicit answer already publishes
//! each seat's filtered view before another socket answer is processed.

use std::io;

const VARIABLE: &str = "BAYLEE_ENGINE_PUMP_STEPS";

/// Read once before opening the attached engine link; malformed configuration
/// fails the launch instead of silently falling back to batched observations.
pub(super) fn from_env() -> io::Result<usize> {
    match std::env::var(VARIABLE) {
        Ok(value) => parse(Some(&value)),
        Err(std::env::VarError::NotPresent) => parse(None),
        Err(std::env::VarError::NotUnicode(_)) => Err(invalid()),
    }
}

fn invalid() -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidInput,
        "BAYLEE_ENGINE_PUMP_STEPS must be an integer from 1 through 128",
    )
}

fn parse(value: Option<&str>) -> io::Result<usize> {
    let Some(value) = value else {
        return Ok(baylee_engine_server::ATTACHED_PUMP_SLICE);
    };
    if value.is_empty() || !value.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(invalid());
    }
    value
        .parse::<usize>()
        .ok()
        .filter(|steps| (1..=baylee_engine_server::ATTACHED_PUMP_SLICE).contains(steps))
        .ok_or_else(invalid)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn attached_pump_steps_default_and_valid_range() {
        assert_eq!(parse(None).unwrap(), 128);
        for steps in 1..=128 {
            assert_eq!(parse(Some(&steps.to_string())).unwrap(), steps);
        }
    }

    #[test]
    fn attached_pump_steps_reject_invalid_values_without_fallback() {
        for value in [
            "",
            "0",
            "129",
            "-1",
            "+1",
            " 1",
            "1 ",
            "1.0",
            "one",
            "99999999999999999999999999",
        ] {
            assert_eq!(
                parse(Some(value)).unwrap_err().kind(),
                io::ErrorKind::InvalidInput
            );
        }
    }
}
