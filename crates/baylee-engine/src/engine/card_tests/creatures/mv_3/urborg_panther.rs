//! `cards/creatures/mv_3/urborg_panther.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn urborg_panther() -> CardIndex {
    card_index("0678beaf-564c-4e98-b45f-a544925683a1")
}

fn feral_shadow() -> CardIndex {
    baylee_core::generated::index::FERAL_SHADOW
}

fn breathstealer() -> CardIndex {
    baylee_core::generated::index::BREATHSTEALER
}

fn spirit_of_the_night() -> CardIndex {
    baylee_core::generated::index::SPIRIT_OF_THE_NIGHT
}

fn hill_giant() -> CardIndex {
    baylee_core::generated::index::HILL_GIANT
}

/// Urborg Panther (`{2}{B}`, `Coverage::Implemented`):
/// - `{B}`, Sacrifice Urborg Panther: destroy target creature blocking it.
/// - Sacrifice a creature named Feral Shadow, a creature named Breathstealer,
///   and Urborg Panther: search your library for a card named Spirit of the
///   Night and put it onto the battlefield.
#[test]
fn urborg_panther_destroys_blocker_and_tutors_spirit_of_the_night() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));

    // Part 1: {B}, Sacrifice Urborg Panther to destroy target creature blocking it.
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[urborg_panther(), swamp()])
        .battlefield(1, &[hill_giant()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);

    let panther = on_battlefield(&engine, p0, urborg_panther()).unwrap();
    let giant = on_battlefield(&engine, p1, hill_giant()).unwrap();

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(panther, Defender::Player(p1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(giant, panther)],
            },
        )
        .unwrap();
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p0),
    );

    tap_all_mana(&mut engine, p0);
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: panther,
                ability_index: 0,
            },
        )
        .unwrap();
    aim_at(&mut engine, p0, giant);
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, urborg_panther()).is_none());
    assert!(on_battlefield(&engine, p1, hill_giant()).is_none());

    // Part 2: Sacrifice Feral Shadow, Breathstealer, and Urborg Panther to search for Spirit of the Night.
    let mut engine2 = Duel::new(SEED, spirit_of_the_night())
        .battlefield(0, &[urborg_panther(), feral_shadow(), breathstealer()])
        .start();
    keep_mulligans(&mut engine2);
    reach_main_phase(&mut engine2, p0);

    let panther2 = on_battlefield(&engine2, p0, urborg_panther()).unwrap();
    let shadow = on_battlefield(&engine2, p0, feral_shadow()).unwrap();
    let stealer = on_battlefield(&engine2, p0, breathstealer()).unwrap();

    engine2
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: panther2,
                ability_index: 1,
            },
        )
        .unwrap();
    engine2
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![shadow],
            },
        )
        .unwrap();
    engine2
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![stealer],
            },
        )
        .unwrap();
    pass_until(&mut engine2, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards { options, .. } = engine2.pending().clone() else {
        unreachable!();
    };
    engine2
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![options[0]],
            },
        )
        .unwrap();
    pass_until(&mut engine2, stack_is_empty);

    assert!(on_battlefield(&engine2, p0, urborg_panther()).is_none());
    assert!(on_battlefield(&engine2, p0, feral_shadow()).is_none());
    assert!(on_battlefield(&engine2, p0, breathstealer()).is_none());
    assert!(
        on_battlefield(&engine2, p0, spirit_of_the_night()).is_some(),
        "Spirit of the Night put onto the battlefield from library"
    );
}
