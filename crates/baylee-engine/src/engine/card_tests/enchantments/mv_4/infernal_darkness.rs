//! `cards/enchantments/mv_4/infernal_darkness.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_cards_dsl::Effect;
use baylee_core::{generated::index, mana, mana::ManaCost};

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn infernal_darkness() -> CardIndex {
    card_index("dd821fca-79a0-48a9-b1cf-5a496bc04f65")
}

fn first_upkeep_paid(mine: &[CardIndex], theirs: &[CardIndex]) -> Engine<RegistryLookup> {
    lk::game_with(SEED, mine, theirs, &[], &[], |e| {
        assert!(
            matches!(
                e.pending(),
                Pending::YesNo {
                    prompt: YesNoPrompt::PayManaAndLife { cost, life: 1 },
                    ..
                } if *cost == mana!("{B}")
            ),
            "the first upkeep offers {{B}} and 1 life as one payment, got {:?}",
            e.pending()
        );
        lk::float(e, P0, &[(ManaColor::Black, 1)]);
        lk::yes_no(e, true);
    })
}

#[track_caller]
fn wait_for_combined_upkeep(
    engine: &mut Engine<RegistryLookup>,
    turn: u32,
    cost: ManaCost,
    life: u32,
) {
    pass_until(engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::PayManaAndLife {
                    cost: asked,
                    life: asked_life,
                },
                ..
            } if *asked == cost && *asked_life == life
        )
    });
    assert_eq!(
        (engine.state().turn.number, engine.state().turn.active),
        (turn, P0),
        "cumulative upkeep asks in its controller's own upkeep, not in the opponent's"
    );
}

fn tap_restored_land_colors(engine: &mut Engine<RegistryLookup>) {
    tap_all_mana_but(engine, P0, None);
    for (color, expected) in [
        (ManaColor::Green, 1),
        (ManaColor::White, 1),
        (ManaColor::Blue, 1),
        (ManaColor::Black, 0),
    ] {
        assert_eq!(lk::pool_of(engine, P0, color), expected);
    }
}

/// Infernal Darkness (dd821fca-79a0-48a9-b1cf-5a496bc04f65) — {2}{B}{B} Enchantment:
/// - "Cumulative upkeep—Pay {B} and 1 life."
/// - "If a land is tapped for mana, it produces {B} instead of any other type."
#[test]
fn infernal_darkness_turns_all_land_mana_black_until_cumulative_upkeep_is_declined() {
    let mut engine = first_upkeep_paid(
        &[infernal_darkness(), forest(), plains(), island()],
        &[index::MOUNTAIN],
    );
    let darkness = lk::obj(&engine, P0, infernal_darkness());
    assert_eq!(
        engine
            .state()
            .object(darkness)
            .unwrap()
            .counters
            .get(baylee_cards_dsl::counters::AGE),
        1
    );
    assert_eq!(
        lk::life(&engine, P0),
        19,
        "the first upkeep cost {{B}} and 1 life (one age counter)"
    );

    tap_all_mana_but(&mut engine, P0, None);
    assert_eq!(
        lk::pool_of(&engine, P0, ManaColor::Black),
        3,
        "Forest, Plains, and Island all produce {{B}} instead of their normal colors"
    );
    assert_eq!(lk::pool_total(&engine, P0), 3);

    reach_their_main_phase(&mut engine, P1);
    tap_all_mana_but(&mut engine, P1, None);
    assert_eq!(
        lk::pool_of(&engine, P1, ManaColor::Black),
        1,
        "opponent's Mountain also produces {{B}}"
    );

    wait_for_combined_upkeep(&mut engine, 3, mana!("{B}{B}"), 2);
    assert_eq!(
        engine
            .state()
            .object(darkness)
            .unwrap()
            .counters
            .get(baylee_cards_dsl::counters::AGE),
        2
    );
    lk::yes_no(&mut engine, true);
    tap_all_mana_but(&mut engine, P0, None);
    engine.apply(P0, PlayerAction::PassPriority).unwrap();
    assert!(
        !matches!(
            engine.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::PayLife { .. },
                ..
            }
        ),
        "the life is not a second payment after the mana"
    );
    lk::settle(&mut engine);
    assert!(on_battlefield(&engine, P0, infernal_darkness()).is_some());
    assert_eq!(
        lk::life(&engine, P0),
        17,
        "the second upkeep cost 2 life on top of the first one's 1"
    );
    assert_eq!(
        lk::pool_of(&engine, P0, ManaColor::Black),
        1,
        "two of the three {{B}} paid the second upkeep's {{B}}{{B}}"
    );

    wait_for_combined_upkeep(&mut engine, 5, mana!("{B}{B}{B}"), 3);
    lk::yes_no(&mut engine, false);
    lk::settle(&mut engine);
    assert!(in_graveyard(&engine, P0, infernal_darkness()).is_some());
    assert_eq!(
        lk::life(&engine, P0),
        17,
        "declining the payment costs no life"
    );

    tap_restored_land_colors(&mut engine);
}

#[test]
fn infernal_darkness_declining_the_combined_upkeep_spends_no_mana() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[infernal_darkness(), forest()])
        .start();
    keep_mulligans(&mut engine);
    wait_for_combined_upkeep(&mut engine, 1, mana!("{B}"), 1);
    lk::float(&mut engine, P0, &[(ManaColor::Black, 1)]);
    lk::yes_no(&mut engine, false);
    lk::settle(&mut engine);

    assert!(in_graveyard(&engine, P0, infernal_darkness()).is_some());
    assert_eq!(lk::life(&engine, P0), 20, "the refused life was not paid");
    assert_eq!(
        lk::pool_of(&engine, P0, ManaColor::Black),
        1,
        "declining the complete price cannot consume its mana component"
    );
}

#[test]
fn infernal_darkness_unpayable_life_also_spends_no_mana() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[infernal_darkness(), forest()])
        .life(0, 2)
        .start();
    keep_mulligans(&mut engine);
    wait_for_combined_upkeep(&mut engine, 1, mana!("{B}"), 1);
    lk::float(&mut engine, P0, &[(ManaColor::Black, 1)]);
    lk::yes_no(&mut engine, true);
    if engine.payment_window().is_some() {
        engine.apply(P0, PlayerAction::PassPriority).unwrap();
    }
    assert_eq!(lk::life(&engine, P0), 1, "the first upkeep was paid");

    reach_their_main_phase(&mut engine, P1);
    wait_for_combined_upkeep(&mut engine, 3, mana!("{B}{B}"), 2);
    lk::float(&mut engine, P0, &[(ManaColor::Black, 2)]);
    lk::yes_no(&mut engine, true);
    if engine.payment_window().is_some() {
        engine.apply(P0, PlayerAction::PassPriority).unwrap();
    }
    lk::settle(&mut engine);

    assert!(in_graveyard(&engine, P0, infernal_darkness()).is_some());
    assert_eq!(lk::life(&engine, P0), 1, "the unpayable life was not paid");
    assert_eq!(
        lk::pool_of(&engine, P0, ManaColor::Black),
        2,
        "an unpayable life component prevents the entire payment"
    );
}

#[test]
fn infernal_darkness_replaces_each_unit_of_a_lands_colorless_mana() {
    let mut engine = first_upkeep_paid(&[infernal_darkness(), index::ANCIENT_TOMB], &[]);
    tap_all_mana_but(&mut engine, P0, None);

    assert_eq!(
        lk::pool_of(&engine, P0, ManaColor::Black),
        2,
        "Ancient Tomb's {{C}}{{C}} becomes {{B}}{{B}}, preserving the amount"
    );
    assert_eq!(lk::pool_total(&engine, P0), 2);
    assert_eq!(
        lk::life(&engine, P0),
        17,
        "replacing the mana type does not remove Ancient Tomb's damage rider"
    );
}

#[test]
fn infernal_darkness_changes_land_mana_only_not_an_artifacts() {
    let mut engine = first_upkeep_paid(&[infernal_darkness(), forest(), sol_ring()], &[]);
    tap_all_mana_but(&mut engine, P0, None);
    assert_eq!(
        lk::pool_of(&engine, P0, ManaColor::Black),
        1,
        "the Forest is a land: it produces {{B}}"
    );
    assert_eq!(
        lk::pool_of(&engine, P0, ManaColor::Colorless),
        2,
        "Sol Ring is not a land: its {{C}}{{C}} is not turned black"
    );
}

#[test]
fn infernal_darkness_is_recognized_as_cumulative_upkeep() {
    let def =
        baylee_cards::by_index(infernal_darkness()).expect("Infernal Darkness is in the pool");
    assert!(
        def.abilities.iter().any(|ability| match ability {
            AbilityDef::Triggered { effects, .. } => Effect::is_cumulative_upkeep(effects),
            _ => false,
        }),
        "restricted mana and cumulative-upkeep filters must recognize the printed ability"
    );
}
