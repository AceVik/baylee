//! `cards/instants/mv_3/kazuul_s_fury.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Kazuul's Cliffs, the land face: "This land enters tapped. {T}: Add {R}."
/// The test plays the back face as a land, confirms it enters tapped, advances
/// to the next turn so it untaps, and activates its mana ability to add `{R}`.
#[test]
fn kazuuls_cliffs_enters_tapped_and_taps_for_red_mana() {
    let (mut engine, cliffs) =
        play_land_face(kazuul_s_fury(), 1).expect("plays as Kazuul's Cliffs");
    assert!(is_tapped(&engine, cliffs), "Kazuul's Cliffs enters tapped");

    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);

    assert!(!is_tapped(&engine, cliffs), "untaps on next turn");
    let red_before = engine.state().players[0]
        .mana_pool
        .available(ManaColor::Red);

    activate(&mut engine, p0, kazuul_s_fury(), 0);

    assert!(
        is_tapped(&engine, cliffs),
        "tapped to activate mana ability"
    );
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Red),
        red_before + 1,
        "adds one red mana to the pool"
    );
}

/// A 2/1 Cat: power and toughness differ.
fn savannah_lions() -> CardIndex {
    card_index("60ba93eb-39e6-4af2-9c66-cd38f72daff2")
}

/// A 3/3 that two damage does not kill and five does.
fn hill_giant() -> CardIndex {
    card_index("342199e0-15b6-4824-83da-25caef2592b3")
}

fn glorious_anthem() -> CardIndex {
    card_index("e3886fe8-9b76-4613-8891-4ec74657c087")
}

/// Casts the front face off three floating Mountains and answers whatever
/// the cast asks: the target (a creature or a player) and the creature to
/// sacrifice as the additional cost. Returns the sacrifice options seen.
#[track_caller]
fn cast_fury(
    engine: &mut Engine<RegistryLookup>,
    target_creature: Option<ObjectId>,
    target_player: Option<PlayerId>,
    sacrifice: ObjectId,
) -> Vec<ObjectId> {
    let p0 = PlayerId::new(0);
    let mountains = mine(engine, p0, mountain(), Zone::Battlefield);
    tap_mana_where(engine, p0, |id| mountains.contains(&id));
    cast_front_face(engine, p0, kazuul_s_fury());
    let mut offered = Vec::new();
    for _ in 0..6 {
        match engine.pending().clone() {
            Pending::ChooseTargets { .. } => {
                engine
                    .apply(
                        p0,
                        PlayerAction::ChooseTargets {
                            objects: target_creature.into_iter().collect(),
                            players: target_player.into_iter().collect(),
                        },
                    )
                    .expect("the target was on the menu");
            }
            Pending::ChooseCards { options, .. } => {
                offered.clone_from(&options);
                assert!(options.contains(&sacrifice), "{options:?}");
                engine
                    .apply(
                        p0,
                        PlayerAction::ChooseObjects {
                            objects: vec![sacrifice],
                        },
                    )
                    .expect("sacrifice it");
            }
            _ => break,
        }
    }
    pass_until(engine, stack_is_empty);
    offered
}

fn fury_board(extra_mine: &[CardIndex]) -> Engine<RegistryLookup> {
    let p0 = PlayerId::new(0);
    let mut mine_board = vec![
        mountain(),
        mountain(),
        mountain(),
        forest(),
        savannah_lions(),
    ];
    mine_board.extend_from_slice(extra_mine);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &mine_board)
        .battlefield(1, &[hill_giant()])
        .hand(0, &[kazuul_s_fury(), giant_growth()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    engine
}

/// "Kazuul's Fury deals damage equal to the sacrificed creature's power to
/// any target." The Lions (2/1) are pumped to 5/4 first, so a Hill Giant
/// (3/3) targeted dies: two damage (printed) would not kill it, and the
/// sacrificed Lions are gone as the additional cost (CR 601.2h).
#[test]
fn kazuuls_fury_deals_the_pumped_power_to_a_creature() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = fury_board(&[]);
    let lions = on_battlefield(&engine, p0, savannah_lions()).expect("the Lions");
    let giant = on_battlefield(&engine, p1, hill_giant()).expect("their Giant");

    let forest_id = on_battlefield(&engine, p0, forest()).expect("a Forest");
    tap_mana_where(&mut engine, p0, |id| id == forest_id);
    cast_with_floating(&mut engine, p0, giant_growth());
    aim_at(&mut engine, p0, lions);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, lions), (5, 4), "Giant Growth: +3/+3");

    cast_fury(&mut engine, Some(giant), None, lions);
    assert!(
        in_graveyard(&engine, p0, savannah_lions()).is_some(),
        "sacrificed"
    );
    assert!(
        in_graveyard(&engine, p1, hill_giant()).is_some(),
        "five damage kills the 3/3"
    );
    assert!(
        in_graveyard(&engine, p0, kazuul_s_fury()).is_some(),
        "the spell resolved"
    );
    assert_eq!(
        engine.state().players[1].life,
        20,
        "only the creature was hit"
    );
}

/// The same amount goes to a player: the Lions (2/1) under Glorious Anthem
/// are a 3/2 when sacrificed, so the opponent takes 3.
#[test]
fn kazuuls_fury_deals_the_anthem_power_to_a_player() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = fury_board(&[glorious_anthem()]);
    let lions = on_battlefield(&engine, p0, savannah_lions()).expect("the Lions");
    assert_eq!(pt(&engine, lions), (3, 2), "Glorious Anthem: +1/+1");

    cast_fury(&mut engine, None, Some(p1), lions);
    assert_eq!(engine.state().players[1].life, 17);
    assert_eq!(engine.state().players[0].life, 20);
    assert!(in_graveyard(&engine, p0, savannah_lions()).is_some());
}

/// A +1/+1 counter counts, and the sacrifice is of a creature its caster
/// controls: the opposing Giant is never on the offer.
#[test]
fn kazuuls_fury_counts_a_counter_and_sacrifices_only_your_creature() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = fury_board(&[]);
    let lions = on_battlefield(&engine, p0, savannah_lions()).expect("the Lions");
    let giant = on_battlefield(&engine, p1, hill_giant()).expect("their Giant");
    let state = engine
        .dev_state_mut(p0)
        .expect("the harness may set boards up");
    crate::replacement::put_counters(state, lions, baylee_cards_dsl::CounterKind::P1P1, 1);
    reach_their_main_phase(&mut engine, p1);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");
    assert_eq!(pt(&engine, lions), (3, 2), "a +1/+1 counter");

    let offered = cast_fury(&mut engine, None, Some(p1), lions);
    assert!(!offered.contains(&giant), "{offered:?}");
    assert_eq!(engine.state().players[1].life, 17);
}

/// "As an additional cost to cast this spell, sacrifice a creature": with
/// no creature of its own, the spell cannot be cast even with {2}{R} floating
/// and an opposing creature to point at.
#[test]
fn kazuuls_fury_cannot_be_cast_without_a_creature_to_sacrifice() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[mountain(), mountain(), mountain()])
        .battlefield(1, &[hill_giant()])
        .hand(0, &[kazuul_s_fury()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    tap_all_mana(&mut engine, p0);
    assert_eq!(engine.state().players[0].mana_pool.total(), 3);
    let fury = in_hand(&engine, p0, kazuul_s_fury()).expect("in hand");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("priority expected")
    };
    assert!(!legal.castable.contains(&fury), "no creature to sacrifice");
    assert!(
        engine
            .apply(p0, PlayerAction::CastSpell { card: fury })
            .is_err(),
        "casting is refused"
    );
    assert!(in_hand(&engine, p0, kazuul_s_fury()).is_some());
}
