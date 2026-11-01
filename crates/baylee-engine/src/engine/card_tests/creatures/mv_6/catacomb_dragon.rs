//! `cards/creatures/mv_6/catacomb_dragon.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::{
    LUMENGRID_GARGOYLE, MAHAMOTI_DJINN, SERRA_ANGEL, SHIVAN_DRAGON,
};

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn catacomb_dragon() -> CardIndex {
    card_index("a5e9f72b-7628-4265-8b43-592a0bf678b3")
}

/// Catacomb Dragon (a5e9f72b-7628-4265-8b43-592a0bf678b3) — {4}{B}{B} 4/4 Flying:
/// "Whenever this creature becomes blocked by a nonartifact, non-Dragon
/// creature, that creature gets -X/-0 until end of turn, where X is half the
/// creature's power, rounded down."
#[test]
fn catacomb_dragon_halves_power_of_nonartifact_nondragon_blocker() {
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[catacomb_dragon()])
        .battlefield(1, &[SERRA_ANGEL, SHIVAN_DRAGON])
        .start();
    keep_mulligans(&mut engine);

    let dragon = on_battlefield(&engine, P0, catacomb_dragon()).unwrap();
    let angel = on_battlefield(&engine, P1, SERRA_ANGEL).unwrap();
    let shivan = on_battlefield(&engine, P1, SHIVAN_DRAGON).unwrap();

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            P0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(dragon, Defender::Player(P1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(
            P1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(angel, dragon), (shivan, dragon)],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    // Serra Angel (4/4 nonartifact non-Dragon) gets -2/-0 -> 2/4.
    assert_eq!(pt(&engine, angel), (2, 4));
    // Shivan Dragon (5/5 Dragon) does not trigger the debuff -> stays 5/5.
    assert_eq!(pt(&engine, shivan), (5, 5));
}

/// Declares the Catacomb Dragon as P0's attacker, blocked by `blocker`, and
/// lets the trigger (if any) resolve.
fn dragon_is_blocked_by(blocker: CardIndex) -> (Engine<RegistryLookup>, ObjectId) {
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[catacomb_dragon()])
        .battlefield(1, &[blocker])
        .start();
    keep_mulligans(&mut engine);

    let dragon = on_battlefield(&engine, P0, catacomb_dragon()).unwrap();
    let blocking = on_battlefield(&engine, P1, blocker).unwrap();

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            P0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(dragon, Defender::Player(P1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(
            P1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(blocking, dragon)],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    (engine, blocking)
}

/// "nonartifact": a flying artifact creature (Lumengrid Gargoyle, 4/4) that
/// blocks keeps its power.
#[test]
fn catacomb_dragon_does_not_weaken_an_artifact_blocker() {
    let (engine, gargoyle) = dragon_is_blocked_by(LUMENGRID_GARGOYLE);

    assert_eq!(pt(&engine, gargoyle), (4, 4));
}

/// "half the creature's power, rounded down": Mahamoti Djinn (5/6) loses 2.
#[test]
fn catacomb_dragon_rounds_half_the_blockers_power_down() {
    let (engine, djinn) = dragon_is_blocked_by(MAHAMOTI_DJINN);

    assert_eq!(pt(&engine, djinn), (3, 6));
}
