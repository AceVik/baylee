//! Shared fixtures for the package, calendar and expedition tests.

mod package;
mod stock;
mod tower;

use super::*;
use baylee_core::ids::{CardIndex, PrintRef};
use baylee_core::preset::{AIProfile, DeckEntry, FormatId, HouseRules, SeatCapabilities, SeatSpec};
use central_api::challenges::{
    Certificate, Challenge, Description, Kind, Period, Publication, SolutionFamily, Witness, Words,
};

/// Monday 2026-11-09, 00:00 UTC: both a daily and a weekly slot start here.
const MONDAY: i64 = 1_794_182_400;
const DAY: i64 = 86_400;

fn seat(controller: SeatController) -> SeatSpec {
    SeatSpec {
        controller,
        capabilities: SeatCapabilities::default(),
        deck: vec![DeckEntry {
            card: CardIndex::new(0),
            print: PrintRef::new(0),
            rule_print: None,
            choices: baylee_core::card_metadata::DeckChoices::default(),
        }],
        sideboard: Vec::new(),
        commanders: Vec::new(),
        starting_life: None,
        starting_hand: None,
        starting_battlefield: Vec::new(),
        emblems: Vec::new(),
        team: None,
    }
}

/// A package the official executor accepts.
fn package() -> Package {
    let house_rules = HouseRules {
        challenge_policy_seed: Some(74),
        decision_timeout_secs: 0,
        reconnect_window_secs: 0,
        takebacks: false,
        time_extension_votes: false,
        ..HouseRules::default()
    };
    Package {
        schema: 1,
        executor: executor().into(),
        preset: GamePreset {
            format: FormatId::Custom,
            seed: 1,
            house_rules,
            modifiers: Vec::new(),
            prints: serde_json::from_str(
                r#"[{"scryfall_id":"00000000-0000-0000-0000-000000000000","lang":"EN","finish":"Normal"}]"#,
            )
            .unwrap(),
            rules: baylee_core::card_metadata::RulesMetadata::default(),
            seats: vec![
                seat(SeatController::Open),
                seat(SeatController::Ai(AIProfile {
                    foresight: 0,
                    ..AIProfile::STEADY
                })),
            ],
        },
        target_turn: 8,
    }
}

fn words(text: &str) -> Words {
    Words {
        en: text.into(),
        de: text.into(),
    }
}

/// A certified publication of `package` for `period`, told apart by `title`.
fn publish(package: &Package, period: Period, title: &str) -> Publication {
    let mut publication = Publication {
        description: Description {
            period,
            title: words(title),
            instructions: words("Win on time."),
            deck: "1 Plains".into(),
            executor: package.executor.clone(),
            target_turn: package.target_turn,
            expected_minutes: 10,
        },
        digest: [0; 32],
        package: package.encode().unwrap(),
        certificate: Certificate {
            validator: "validator".into(),
            witnesses: [
                SolutionFamily::Combat,
                SolutionFamily::DirectDamage,
                SolutionFamily::AlternativeVictory,
            ]
            .into_iter()
            .zip(1..)
            .map(|(family, n)| Witness {
                record_digest: [n; 32],
                strategy_digest: [n; 32],
                family,
            })
            .collect(),
        },
        hints: [words("Attack."), words("Attack harder.")],
    };
    sign(&mut publication);
    publication
}

/// Recomputes the content digest after a test changed the publication.
fn sign(publication: &mut Publication) {
    publication.digest = publication.calculated_digest().unwrap();
}

/// A version 7 identifier, distinct per `n`.
fn id(n: u8) -> [u8; 16] {
    let mut id = [n; 16];
    id[6] = 0x70;
    id[8] = 0x80;
    id
}

/// The authority's own record of `publication`.
fn held(publication: &Publication, n: u8) -> Challenge {
    Challenge {
        id: id(n),
        description: publication.description.clone(),
        digest: publication.digest,
        withdrawn: false,
    }
}

fn slot(kind: Kind, at: i64) -> Period {
    Period::containing(kind, at).unwrap()
}
