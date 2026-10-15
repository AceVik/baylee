//! `cards/creatures/artifacts/mv_2/narnam_cobra.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn narnam_cobra() -> CardIndex {
    card_index("2ac0d584-997b-4d11-aecf-f1494eebc94c")
}

/// The Cobra attacks into a 0/8 Wall of Stone, optionally paying `{G}` for
/// deathtouch once the attack is declared. Says whether the Wall survived.
fn cobra_attacks_a_wall(pay_for_deathtouch: bool) -> bool {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = table(&[], &[narnam_cobra(), forest()], &[wall_of_stone()]);
    let cobra = on_battlefield(&engine, p0, narnam_cobra()).expect("seated");
    let wall = on_battlefield(&engine, p1, wall_of_stone()).expect("seated");
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(cobra, Defender::Player(p1))],
            },
        )
        .expect("the Cobra attacks");
    if pay_for_deathtouch {
        tap_all_mana(&mut engine, p0);
        activate(&mut engine, p0, narnam_cobra(), 0);
        pass_until(&mut engine, |e| {
            stack_is_empty(e) && matches!(e.pending(), Pending::Priority { .. })
        });
        assert!(keywords(&engine, cobra).contains(KeywordSet::DEATHTOUCH));
    } else {
        assert!(!keywords(&engine, cobra).contains(KeywordSet::DEATHTOUCH));
    }
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(wall, cobra)],
            },
        )
        .expect("the Wall blocks");
    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });
    assert!(
        on_battlefield(&engine, p0, narnam_cobra()).is_some(),
        "the Wall deals no damage"
    );
    on_battlefield(&engine, p1, wall_of_stone()).is_some()
}

/// Narnam Cobra: a 2/1 with "{G}: This creature gains deathtouch until end
/// of turn". With the `{G}` its two damage destroys a 0/8 Wall.
#[test]
fn narnam_cobra_with_deathtouch_destroys_a_wall_it_blocks_into() {
    assert!(!cobra_attacks_a_wall(true), "deathtouch: the Wall dies");
}

/// Without the `{G}` two damage does not scratch an eight-toughness Wall.
#[test]
fn narnam_cobra_without_deathtouch_does_not_hurt_the_wall() {
    assert!(cobra_attacks_a_wall(false), "no deathtouch: the Wall lives");
}

/// Green is required: nothing floating, nothing offered.
#[test]
fn narnam_cobra_needs_green_mana() {
    let p0 = PlayerId::new(0);
    let engine = table(&[], &[narnam_cobra(), island()], &[]);
    let cobra = on_battlefield(&engine, p0, narnam_cobra()).expect("seated");
    assert_eq!(pt(&engine, cobra), (2, 1));
    assert!(!priority_offer(&engine).abilities.contains(&(cobra, 0)));
}
