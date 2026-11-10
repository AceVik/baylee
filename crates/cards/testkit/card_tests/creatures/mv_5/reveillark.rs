//! `cards/creatures/mv_5/reveillark.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Reveillark — {4}{W}, 4/3 flying — "when this creature leaves the
/// battlefield, return up to two target creature cards with power 2 or less
/// from your graveyard to the battlefield."
///
/// The same three creatures as the Sun Titan test and a different reason for
/// the same split: there it was mana value 3, here it is power 2, so the
/// Elephant is off the menu for being a 3/3 rather than for costing {3}{G}.
/// The boundary itself is the neighbour's to pin — no fixture here prints a
/// power of exactly 2 — and what this board says is that a 3 is refused,
/// which is the off-by-one that matters, and that the clause is read at all:
/// it was `CmcAtMost(0xFFFF)` with a comment admitting it, so the card
/// returned anything.
///
/// Two cards come back and not one, which is the half `spec_object` could
/// not do — it answers `first()`, and this card said `Coverage::Implemented`
/// while returning half of what it prints.
#[test]
fn reveillark_returns_both_small_creatures_and_is_not_offered_the_three_power() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(42, forest())
        .battlefield(
            0,
            &[
                reveillark(),
                llanowar_elves(),
                rib_cage_spider(),
                wild_elephant(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let lark = on_battlefield(&engine, p0, reveillark()).expect("the Lark is seated");
    let elves = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elves are seated");
    let spider = on_battlefield(&engine, p0, rib_cage_spider()).expect("the Spider is seated");
    let elephant = on_battlefield(&engine, p0, wild_elephant()).expect("the Elephant is seated");
    bury(&mut engine, &[elves, spider, elephant]);

    // The Lark leaves last, so its own trigger has a graveyard to read.
    bury(&mut engine, &[lark]);
    let Pending::Priority { player, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    engine
        .apply(player, PlayerAction::PassPriority)
        .expect("passing priority is always legal");

    let options = pass_until_targets(&mut engine, p0);
    assert!(
        options.contains(&elves) && options.contains(&spider),
        "a 1/1 and a 1/4 are both \"power 2 or less\": {options:?}"
    );
    assert!(
        !options.contains(&elephant),
        "and a 3/3 is not, however cheap it was: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![elves, spider],
                players: vec![],
            },
        )
        .expect("two targets, which is what \"up to two\" offers");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_some(),
        "the first of the two came back"
    );
    assert!(
        on_battlefield(&engine, p0, rib_cage_spider()).is_some(),
        "and so did the second — the half a reader taking `first()` lost"
    );
    assert!(
        on_battlefield(&engine, p0, wild_elephant()).is_none(),
        "and the one that was never named stayed where it was"
    );
}

/// Reveillark, evoked: "Evoke {5}{W} ... If you do, it's sacrificed when it
/// enters." The sacrifice trigger is the ability under test, and the leave
/// trigger it causes returns the Elves.
#[test]
fn reveillark_evoked_is_sacrificed_and_its_leave_trigger_returns_a_creature() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(2205, forest())
        .battlefield(
            0,
            &[
                plains(),
                plains(),
                plains(),
                plains(),
                plains(),
                plains(),
                llanowar_elves(),
            ],
        )
        .hand(0, &[reveillark()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let elves = on_battlefield(&engine, p0, llanowar_elves()).expect("elves");
    bury(&mut engine, &[elves]);

    tap_all_mana(&mut engine, p0);
    let lark = in_hand(&engine, p0, reveillark()).expect("lark in hand");
    engine
        .apply(p0, PlayerAction::CastSpell { card: lark })
        .unwrap();
    let slot = choose_cast_kind(&engine, CastModeKind::Alternative(0));
    engine.apply(p0, PlayerAction::ChooseMode(slot)).unwrap();

    let options = pass_until_targets(&mut engine, p0);
    assert!(
        options.contains(&elves),
        "the leave trigger came from the sacrifice"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![elves],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p0, reveillark()).is_none(),
        "sacrificed"
    );
    assert!(in_graveyard(&engine, p0, reveillark()).is_some());
    assert!(on_battlefield(&engine, p0, llanowar_elves()).is_some());
}
