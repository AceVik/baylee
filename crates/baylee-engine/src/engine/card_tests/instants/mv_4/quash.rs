//! `cards/instants/mv_4/quash.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn quash() -> CardIndex {
    card_index("255131a2-616c-4c8d-a36a-14e5b5ab345e")
}

/// Quash (255131a2-616c-4c8d-a36a-14e5b5ab345e) — {2}{U}{U} Instant (Partial):
/// "Counter target instant or sorcery spell. Search its controller's graveyard,
/// hand, and library for all cards with the same name as that spell and exile
/// them. Then that player shuffles."
///
/// Counters target instant or sorcery spell on the stack and refuses creature spells.
#[test]
fn quash_counters_target_instant_or_sorcery_and_excludes_creature_spells() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), island(), island()])
        .battlefield(1, &[mountain(), mountain(), forest()])
        .hand(0, &[quash()])
        .hand(1, &[grizzly_bears(), lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, p1);

    // 1. P1 casts a creature spell (Grizzly Bears); P1 responds with an instant (Lightning Bolt).
    // Both are on the stack when P0 casts Quash; only Lightning Bolt is a legal target.
    tap_all_mana(&mut engine, p1);
    let bears_card = in_hand(&engine, p1, grizzly_bears()).unwrap();
    engine
        .apply(p1, PlayerAction::CastSpell { card: bears_card })
        .unwrap();
    let bears_on_stack = on_stack(&engine, grizzly_bears()).unwrap();

    let bolt_card = in_hand(&engine, p1, lightning_bolt()).unwrap();
    engine
        .apply(p1, PlayerAction::CastSpell { card: bolt_card })
        .unwrap();
    engine
        .apply(
            p1,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p0],
            },
        )
        .unwrap();
    let bolt_on_stack = on_stack(&engine, lightning_bolt()).unwrap();

    engine.apply(p1, PlayerAction::PassPriority).unwrap();
    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, quash());

    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "expected ChooseTargets for Quash, got {:?}",
            engine.pending()
        );
    };
    assert!(
        options.contains(&bolt_on_stack),
        "instant spell on the stack is a legal target"
    );
    assert!(
        !options.contains(&bears_on_stack),
        "creature spell on the stack is not a legal target for Quash"
    );

    aim_at(&mut engine, p0, bolt_on_stack);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().players[0].life,
        20,
        "countered Lightning Bolt dealt no damage"
    );
    assert!(in_graveyard(&engine, p1, lightning_bolt()).is_some());
    assert!(
        on_battlefield(&engine, p1, grizzly_bears()).is_some(),
        "uncountered creature spell resolved"
    );
}
