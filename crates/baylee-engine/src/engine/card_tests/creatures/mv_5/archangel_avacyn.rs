//! `cards/creatures/mv_5/archangel_avacyn.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// "When Archangel Avacyn enters, creatures you control gain indestructible
/// until end of turn." Her own side only, and only for the turn; she has
/// flash, flying and vigilance.
#[test]
fn archangel_avacyn_makes_her_side_indestructible_for_the_turn() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(
            0,
            &[
                plains(),
                plains(),
                plains(),
                plains(),
                plains(),
                steadfast_guard(),
            ],
        )
        .battlefield(1, &[steadfast_guard()])
        .hand(0, &[archangel_avacyn()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    cast_from_hand(&mut engine, p0, archangel_avacyn());
    pass_until(&mut engine, stack_is_empty);
    let avacyn = on_battlefield(&engine, p0, archangel_avacyn()).expect("resolved");
    let mine = on_battlefield(&engine, p0, steadfast_guard()).unwrap();
    let theirs = on_battlefield(&engine, PlayerId::new(1), steadfast_guard()).unwrap();
    let front = keywords(&engine, avacyn);
    assert!(
        front.contains(
            KeywordSet::FLASH
                .union(KeywordSet::FLYING)
                .union(KeywordSet::VIGILANCE)
        )
    );
    assert!(
        front.contains(KeywordSet::INDESTRUCTIBLE),
        "she is one of them"
    );
    assert!(keywords(&engine, mine).contains(KeywordSet::INDESTRUCTIBLE));
    assert!(
        !keywords(&engine, theirs).contains(KeywordSet::INDESTRUCTIBLE),
        "an opponent's creature is not one you control"
    );
    pass_until(&mut engine, |e| e.state().turn.active == PlayerId::new(1));
    assert!(
        !keywords(&engine, mine).contains(KeywordSet::INDESTRUCTIBLE),
        "until end of turn"
    );
}

/// A non-Angel creature of hers dies: at the beginning of the next upkeep —
/// the opponent's here, "regardless of whose turn it is" — she transforms,
/// and Avacyn, the Purifier deals 3 to each other creature and each
/// opponent, and none to her controller.
#[test]
fn archangel_avacyn_transforms_at_the_next_upkeep_and_purifies_the_board() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(
            0,
            &[archangel_avacyn(), steadfast_guard(), thundering_giant()],
        )
        .battlefield(1, &[thundering_giant(), steadfast_guard()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let avacyn = on_battlefield(&engine, p0, archangel_avacyn()).unwrap();
    let guard = on_battlefield(&engine, p0, steadfast_guard()).unwrap();
    kill(&mut engine, guard);
    assert_eq!(face_shown(&engine, avacyn), 0, "not yet: the next upkeep");
    assert_eq!(
        engine.state().delayed.len(),
        1,
        "one delayed transform waits"
    );

    through_upkeep_of(&mut engine, p1);
    assert_eq!(face_shown(&engine, avacyn), 1, "Avacyn, the Purifier");
    assert_eq!(pt(&engine, avacyn), (6, 5));
    let back = keywords(&engine, avacyn);
    assert!(back.contains(KeywordSet::FLYING));
    assert!(
        !back.contains(KeywordSet::VIGILANCE),
        "the back face prints flying only"
    );
    let purifier = engine.state().object(avacyn).unwrap().characteristics();
    assert!(
        purifier.colors.contains(baylee_core::color::Color::Red),
        "the color indicator"
    );
    assert!(
        on_battlefield(&engine, p1, steadfast_guard()).is_none(),
        "3 kills a 2/2"
    );
    assert!(
        on_battlefield(&engine, p1, thundering_giant()).is_none(),
        "and a 4/3"
    );
    assert!(
        on_battlefield(&engine, p0, thundering_giant()).is_none(),
        "each other creature, her own side's too"
    );
    assert_eq!(
        engine.state().object(avacyn).unwrap().damage,
        0,
        "not herself"
    );
    assert_eq!(engine.state().players[1].life, 17, "each opponent");
    assert_eq!(engine.state().players[0].life, 20, "not her controller");
}

/// Two non-Angels die in one turn: two delayed triggers, one transform. The
/// second is ignored because she has transformed since it was created
/// (CR 701.27f) — she does not flip back, and the board is purified once.
#[test]
fn archangel_avacyn_does_not_flip_back_when_two_delayed_transforms_fire() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(
            0,
            &[archangel_avacyn(), steadfast_guard(), steadfast_guard()],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let avacyn = on_battlefield(&engine, p0, archangel_avacyn()).unwrap();
    for _ in 0..2 {
        let guard = on_battlefield(&engine, p0, steadfast_guard()).unwrap();
        kill(&mut engine, guard);
    }
    assert_eq!(engine.state().delayed.len(), 2);
    through_upkeep_of(&mut engine, p1);
    assert_eq!(
        face_shown(&engine, avacyn),
        1,
        "transformed once, and stayed"
    );
    assert_eq!(engine.state().players[1].life, 17, "purified once");
    assert!(engine.state().delayed.is_empty());
}
