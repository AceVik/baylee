//! `cards/creatures/mv_3/femeref_knight.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::{BURNING_SHIELD_ASKARI, HILL_GIANT};

const P0: PlayerId = PlayerId::new(0);

/// Femeref Knight (4365521e-996d-4eca-bbc9-b0988fa973ae) — {2}{W} 2/2:
/// "Flanking. {W}: This creature gains vigilance until end of turn."
#[test]
fn femeref_knight_has_flanking_and_gains_vigilance_until_end_of_turn() {
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[femeref_knight(), plains()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let knight = on_battlefield(&engine, P0, femeref_knight()).unwrap();
    let kw = engine
        .state()
        .object(knight)
        .unwrap()
        .characteristics()
        .keywords;
    assert!(kw.contains(KeywordSet::FLANKING));
    assert!(!kw.contains(KeywordSet::VIGILANCE));

    tap_all_mana(&mut engine, P0);
    activate(&mut engine, P0, femeref_knight(), 0);
    pass_until(&mut engine, stack_is_empty);

    assert!(
        engine
            .state()
            .object(knight)
            .unwrap()
            .characteristics()
            .keywords
            .contains(KeywordSet::VIGILANCE)
    );
}

const P1: PlayerId = PlayerId::new(1);

fn femeref_knight() -> CardIndex {
    card_index("4365521e-996d-4eca-bbc9-b0988fa973ae")
}

/// P0's Femeref Knight attacks P1, who blocks it with `blocker`; returns the
/// engine once the blockers are declared and the stack is empty, with both.
fn knight_is_blocked_by(blocker: CardIndex) -> (Engine<RegistryLookup>, ObjectId) {
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[femeref_knight()])
        .battlefield(1, &[blocker])
        .start();
    keep_mulligans(&mut engine);

    let knight = on_battlefield(&engine, P0, femeref_knight()).unwrap();
    let blocking = on_battlefield(&engine, P1, blocker).unwrap();

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            P0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(knight, Defender::Player(P1))],
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
                blockers: vec![(blocking, knight)],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    (engine, blocking)
}

/// "Flanking": a Hill Giant (3/3, no flanking) that blocks is 2/2.
#[test]
fn femeref_knight_flanking_shrinks_a_blocker_without_flanking() {
    let (engine, giant) = knight_is_blocked_by(HILL_GIANT);

    assert_eq!(pt(&engine, giant), (2, 2), "one -1/-1 on the 3/3, not two");
}

/// "without flanking": a blocker with flanking (Burning Shield Askari) is not
/// shrunk.
#[test]
fn femeref_knight_flanking_does_not_shrink_a_flanking_blocker() {
    let (engine, askari) = knight_is_blocked_by(BURNING_SHIELD_ASKARI);

    assert_eq!(pt(&engine, askari), (2, 2));
}

/// "{W}: gains vigilance": attacking with the ability active leaves the
/// Knight untapped; without it the Knight is tapped by the attack.
#[test]
fn femeref_knight_attacks_without_tapping_only_after_its_ability() {
    let attack = |vigilance: bool| -> bool {
        let mut engine = Duel::new(SEED, plains())
            .battlefield(0, &[femeref_knight(), plains()])
            .start();
        keep_mulligans(&mut engine);
        reach_main_phase(&mut engine, P0);
        let knight = on_battlefield(&engine, P0, femeref_knight()).unwrap();
        if vigilance {
            tap_all_mana(&mut engine, P0);
            activate(&mut engine, P0, femeref_knight(), 0);
            pass_until(&mut engine, stack_is_empty);
        }
        pass_until(&mut engine, |e| {
            matches!(e.pending(), Pending::ChooseAttackers { .. })
        });
        engine
            .apply(
                P0,
                PlayerAction::DeclareAttackers {
                    attackers: vec![(knight, Defender::Player(P1))],
                },
            )
            .unwrap();
        engine
            .state()
            .object(knight)
            .unwrap()
            .status
            .contains(Status::TAPPED)
    };

    assert!(attack(false), "no vigilance: the attack taps it");
    assert!(!attack(true), "vigilance: it stays untapped");
}
