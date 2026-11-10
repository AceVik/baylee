//! `cards/creatures/mv_4/emberwilde_caliph.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn emberwilde_caliph() -> CardIndex {
    card_index("8434bdb1-286b-484e-904e-946d074614e7")
}

/// Emberwilde Caliph (8434bdb1-286b-484e-904e-946d074614e7) — {2}{U}{R} 4/4:
/// "Flying, trample. This creature attacks each combat if able. Whenever this
/// creature deals damage, you lose 1 life for each 1 damage it deals."
#[test]
fn emberwilde_caliph_deals_combat_damage_and_causes_controller_to_lose_that_much_life() {
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[emberwilde_caliph()])
        .start();
    keep_mulligans(&mut engine);

    let caliph = on_battlefield(&engine, P0, emberwilde_caliph()).unwrap();
    let kw = engine
        .state()
        .object(caliph)
        .unwrap()
        .characteristics()
        .keywords;
    assert!(kw.contains(KeywordSet::FLYING));
    assert!(kw.contains(KeywordSet::TRAMPLE));

    pass_until(&mut engine, |e| {
        e.state().turn.active == P0 && matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            P0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(caliph, Defender::Player(P1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(P1, PlayerAction::DeclareBlockers { blockers: vec![] })
        .unwrap();
    pass_until(&mut engine, |e| e.state().turn.phase == Phase::SecondMain);

    assert_eq!(engine.state().players[1].life, 16);
    assert_eq!(engine.state().players[0].life, 16);
}

/// "This creature attacks each combat if able": the attack question lists it
/// as required, and declaring no attackers is refused.
#[test]
fn emberwilde_caliph_must_attack_each_combat() {
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[emberwilde_caliph()])
        .start();
    keep_mulligans(&mut engine);

    let caliph = on_battlefield(&engine, P0, emberwilde_caliph()).unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers { required, .. } = engine.pending().clone() else {
        unreachable!()
    };
    assert_eq!(required, vec![caliph]);

    assert!(
        engine
            .apply(P0, PlayerAction::DeclareAttackers { attackers: vec![] })
            .is_err(),
        "an empty declaration leaves the required attacker home"
    );
}
