//! Small complete freeform lists probe policies beyond the two acceptance
//! decks. They do not stand in for legality or coverage of future MTG cards.

use baylee_ai::{AIProfile, HeuristicAgent};
use baylee_core::ids::PrintRef;
use baylee_core::preset::DeckEntry;
use baylee_gamehost::{RegistryLookup, harness};

const FAMILIES: &[(&str, &[&str])] = &[
    (
        "tribal",
        &[
            "Plains",
            "Island",
            "Umara Raptor",
            "Kazandu Blademaster",
            "Ondu Cleric",
        ],
    ),
    (
        "artifacts",
        &[
            "Island",
            "Swamp",
            "Baleful Strix",
            "Sol Ring",
            "Solemn Simulacrum",
            "Darksteel Forge",
        ],
    ),
    (
        "control",
        &[
            "Island",
            "Plains",
            "Island",
            "Counterspell",
            "Jace, the Mind Sculptor",
            "Supreme Verdict",
            "Restoration Angel",
            "Brainstorm",
        ],
    ),
    (
        "graveyard",
        &[
            "Swamp",
            "Swamp",
            "Island",
            "Reanimate",
            "Entreat the Dead",
            "Baleful Strix",
            "Archaeomancer",
            "Brainstorm",
        ],
    ),
];

#[test]
fn implemented_tribal_artifact_control_and_graveyard_lists_finish() {
    let forest = baylee_cards::decks::by_name("Forest").unwrap();
    for (index, (name, list)) in FAMILIES.iter().enumerate() {
        for seed in [7, 41] {
            let mut preset = baylee_cards::decks::probe_preset(seed, forest).unwrap();
            for (seat, names) in [*list, FAMILIES[(index + 1) % FAMILIES.len()].1]
                .into_iter()
                .enumerate()
            {
                preset.seats[seat].starting_hand = None;
                preset.seats[seat].starting_battlefield.clear();
                preset.seats[seat].deck = names
                    .iter()
                    .cycle()
                    .take(48)
                    .map(|name| {
                        let card = baylee_cards::decks::by_name(name)
                            .unwrap_or_else(|| panic!("unregistered fixture: {name}"));
                        assert_eq!(
                            baylee_cards::by_index(card).unwrap().coverage,
                            baylee_cards_dsl::Coverage::Implemented,
                            "{name}: an unimplemented card cannot count as an end-to-end fixture"
                        );
                        DeckEntry {
                            card,
                            print: PrintRef::new(0),
                        }
                    })
                    .collect();
            }
            let agents = [
                HeuristicAgent::new(AIProfile::EXPERT),
                HeuristicAgent::new(AIProfile::SHARP),
            ];
            let report = harness::play_report(RegistryLookup, &preset, &agents, 10_000);
            assert!(
                report.finished(),
                "{name}, seed {seed}: {:?}; {:?}",
                report.halt,
                report.trail
            );
            assert_eq!(
                report
                    .tally
                    .iter()
                    .map(|t| t.refused_cost + t.refused_other)
                    .sum::<usize>(),
                0,
                "{name}, seed {seed}: {:?}",
                report.trail
            );
        }
    }
}
