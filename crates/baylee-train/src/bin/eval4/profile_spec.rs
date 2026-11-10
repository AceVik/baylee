//! A house level named on the command line, optionally with some of its
//! knobs changed: what an ablation measures.
//!
//! `--baseline steady:hold_up=none` is `steady` keeping no mana up, and
//! `--against sharp:generation=2` is `sharp` playing the house code of
//! generation 2, before a later strength change. The name is looked up as
//! every tool that measures against the house looks it up
//! (`AIProfile::lookup`); the knobs then change the copy. Sight is not a
//! knob: a level that peeks is chosen by its name (`seer`), never made out
//! of a fair one, so a measurement cannot hand a fair level what it may not
//! see. Neither is a generation newer than this build's.

use anyhow::{Context as _, bail};
use baylee_core::preset::{AIProfile, HoldUp, Politics};

/// `spec` as a profile: `<level>` or `<level>:<knob>=<value>,…`.
///
/// Knobs: `lookahead` (0–2), `temperature` (milli-units), `mulligan` (0–2),
/// `hold_up` (`none`, `basic`, `threat_aware`), `politics` (`random`,
/// `attack_leader`, `archenemy`) and `generation` (at most
/// [`AIProfile::GENERATION`]).
pub(crate) fn parse(spec: &str) -> anyhow::Result<AIProfile> {
    let (name, knobs) = spec.split_once(':').unwrap_or((spec, ""));
    let mut profile =
        AIProfile::resolve(name).with_context(|| format!("unknown AIProfile {name}"))?;
    for knob in knobs.split(',').filter(|knob| !knob.is_empty()) {
        let (key, value) = knob
            .split_once('=')
            .with_context(|| format!("{spec}: a knob is <name>=<value>, not {knob}"))?;
        let number = || -> anyhow::Result<u32> {
            value
                .parse()
                .with_context(|| format!("{spec}: {key} takes a number, not {value}"))
        };
        match key {
            "lookahead" => {
                let depth = number()?;
                if depth > 2 {
                    bail!("{spec}: lookahead is 0, 1 or 2");
                }
                profile.lookahead = u8::try_from(depth)?;
            }
            "temperature" => profile.temperature_milli = number()?,
            "mulligan" => {
                let skill = number()?;
                if skill > 2 {
                    bail!("{spec}: mulligan is 0, 1 or 2");
                }
                profile.mulligan_skill = u8::try_from(skill)?;
            }
            "hold_up" => {
                profile.hold_up = match value {
                    "none" => HoldUp::None,
                    "basic" => HoldUp::Basic,
                    "threat_aware" => HoldUp::ThreatAware,
                    _ => bail!("{spec}: hold_up is none, basic or threat_aware"),
                };
            }
            "politics" => {
                profile.politics = match value {
                    "random" => Politics::Random,
                    "attack_leader" => Politics::AttackLeader,
                    "archenemy" => Politics::Archenemy,
                    _ => bail!("{spec}: politics is random, attack_leader or archenemy"),
                };
            }
            "generation" => {
                let generation = u16::try_from(number()?)?;
                if generation > AIProfile::GENERATION {
                    bail!(
                        "{spec}: this build plays generations up to {}",
                        AIProfile::GENERATION
                    );
                }
                profile.generation = generation;
            }
            _ => bail!(
                "{spec}: unknown knob {key} (lookahead, temperature, mulligan, hold_up, politics, \
                 generation)"
            ),
        }
    }
    Ok(profile)
}

#[cfg(test)]
mod tests {
    use super::parse;
    use baylee_core::preset::{AIProfile, HoldUp, Politics};

    #[test]
    fn a_bare_name_is_the_level_it_names() {
        for (name, profile) in AIProfile::NAMED.iter().chain(&AIProfile::REFERENCES) {
            assert_eq!(parse(name).unwrap(), *profile, "{name}");
        }
        assert!(parse("grandmaster").is_err());
    }

    #[test]
    fn knobs_change_only_what_they_name() {
        let steady = parse("steady:hold_up=none,mulligan=1").unwrap();
        assert_eq!(steady.hold_up, HoldUp::None);
        assert_eq!(steady.mulligan_skill, 1);
        assert_eq!(
            AIProfile {
                hold_up: HoldUp::Basic,
                mulligan_skill: 2,
                ..steady
            },
            AIProfile::STEADY
        );
        let expert =
            parse("expert:lookahead=1,temperature=30,politics=random,generation=0").unwrap();
        assert_eq!(
            (
                expert.lookahead,
                expert.temperature_milli,
                expert.politics,
                expert.generation
            ),
            (1, 30, Politics::Random, 0)
        );
        assert_eq!(parse("sharp:").unwrap(), AIProfile::SHARP);
    }

    /// A knob never makes a fair level peek, nor plays code this build does
    /// not have, nor takes a value the profile does not mean.
    #[test]
    fn a_spec_outside_the_knobs_is_refused() {
        for spec in [
            "expert:sight=peeking",
            "expert:lookahead=3",
            "steady:mulligan=3",
            "steady:hold_up=always",
            "steady:temperature=-1",
            "steady:mulligan",
            "steady:politics=kingmaker",
        ] {
            assert!(parse(spec).is_err(), "{spec}");
        }
        let newer = format!("sharp:generation={}", AIProfile::GENERATION + 1);
        assert!(parse(&newer).is_err());
        assert!(
            parse("seer:lookahead=1").unwrap().peeks(),
            "seer peeks by its name"
        );
        assert!(!parse("expert:lookahead=1").unwrap().peeks());
    }
}
