//! `cards/creatures/mv_3/cantivore.rs`, played.
//!
//! Oracle:
//! - "Vigilance"
//! - "Cantivore's power and toughness are each equal to the number of
//!   enchantment cards in all graveyards."

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn cantivore() -> CardIndex {
    card_index("2ed5083e-1c58-48c0-8413-390971021d94")
}

fn cda_pt(engine: &Engine<RegistryLookup>, id: ObjectId) -> (i16, i16) {
    let chars =
        baylee_engine::layers::recompute(engine.state(), engine.state().object(id).unwrap())
            .characteristics;
    (chars.power.unwrap_or(0), chars.toughness.unwrap_or(0))
}

fn tap_plains(engine: &mut Engine<RegistryLookup>, count: usize) {
    let lands = all_on_battlefield(engine, P0, plains());
    for &p in &lands[..count] {
        engine
            .apply(P0, PlayerAction::ActivateManaAbility { source: p })
            .unwrap();
    }
}

#[test]
fn cantivore_counts_enchantments_in_all_graveyards_and_has_vigilance() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                plains(),
                plains(),
                plains(),
                mountain(),
                index::LLANOWAR_ELVES,
            ],
        )
        .battlefield(1, &[index::SAVANNAH_LIONS])
        .hand(
            0,
            &[
                cantivore(),
                bojuka_bog(),
                index::PACIFISM,
                index::PACIFISM,
                index::SHOCK,
            ],
        )
        .hand(
            1,
            &[index::HOLY_STRENGTH, index::WEAKNESS, index::GRIZZLY_BEARS],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    // Put 1 enchantment + 1 creature in P0's yard, and 1 enchantment + 1 creature in P1's yard.
    let own_elves = on_battlefield(&engine, P0, index::LLANOWAR_ELVES).unwrap();
    bury(&mut engine, &[own_elves]);
    hand_to_graveyard(&mut engine, P0, index::PACIFISM);
    hand_to_graveyard(&mut engine, P1, index::HOLY_STRENGTH);
    hand_to_graveyard(&mut engine, P1, index::GRIZZLY_BEARS);

    // Enchantment in hand (P0 Pacifism, P1 Weakness) and non-enchantments in yards do not count.
    let cantivore_in_hand = in_hand(&engine, P0, cantivore()).unwrap();
    assert_eq!(cda_pt(&engine, cantivore_in_hand), (2, 2));

    // Cast Cantivore ({1}{W}{W}): enters as 2/2 with Vigilance.
    tap_plains(&mut engine, 3);
    cast_with_floating(&mut engine, P0, cantivore());
    pass_until(&mut engine, stack_is_empty);
    let canti = on_battlefield(&engine, P0, cantivore()).expect("Cantivore resolved");
    assert_eq!(pt(&engine, canti), (2, 2));
    assert!(keywords(&engine, canti).contains(KeywordSet::VIGILANCE));

    // On P0's next turn: enchant Lions with Pacifism (still on battlefield -> 2/2),
    // then Shock Lions so Pacifism falls to graveyard via SBA -> Cantivore grows to 3/3.
    reach_their_main_phase(&mut engine, P1);
    assert!(walk_to_own_main(&mut engine, P0));
    let lions = on_battlefield(&engine, P1, index::SAVANNAH_LIONS).unwrap();
    tap_plains(&mut engine, 2);
    cast_with_floating(&mut engine, P0, index::PACIFISM);
    aim_at(&mut engine, P0, lions);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, canti), (2, 2));

    cast_from_hand(&mut engine, P0, index::SHOCK);
    aim_at(&mut engine, P0, lions);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, canti), (3, 3));

    // Vigilance lets it attack without tapping.
    attack_and_collect_blocks(&mut engine, canti, P1);
    assert!(
        !is_tapped(&engine, canti),
        "vigilance keeps Cantivore untapped"
    );
    engine
        .apply(P1, PlayerAction::DeclareBlockers { blockers: vec![] })
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });

    // Exiling P1's graveyard via Bojuka Bog drops count from 3 to 2.
    play_land(&mut engine, P0, bojuka_bog());
    let Pending::ChooseTargets { .. } = engine.pending().clone() else {
        panic!("expected Bojuka Bog target");
    };
    engine
        .apply(
            P0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![P1],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, canti), (2, 2));
}
