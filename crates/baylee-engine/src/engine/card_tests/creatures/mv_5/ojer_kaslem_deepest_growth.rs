//! `cards/creatures/mv_5/ojer_kaslem_deepest_growth.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Ojer Kaslem, Deepest Growth` is a legendary creature costing `{3}{G}{G}` under `Coverage::Partial`.
/// It prints 6/5 base power and toughness with trample.
/// Under `Coverage::Partial`, its combat-damage reveal clause and dies trigger are not implemented.
/// Standing on the battlefield, it attacks an opponent unblocked, dealing 6 combat damage
/// and reducing their life from 20 to 14 without raising unsupported triggers.
#[test]
fn ojer_kaslem_has_trample_and_deals_combat_damage() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[ojer_kaslem_deepest_growth()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let ojer = on_battlefield(&engine, p0, ojer_kaslem_deepest_growth())
        .expect("Ojer Kaslem is on the battlefield");
    assert_eq!(pt(&engine, ojer), (6, 5), "base stats are 6/5");
    assert!(
        keywords(&engine, ojer).contains(KeywordSet::TRAMPLE),
        "has trample"
    );

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers { defenders, .. } = engine.pending().clone() else {
        panic!(
            "expected ChooseAttackers prompt, got {:?}",
            engine.pending()
        );
    };
    let def = defenders.into_iter().next().expect("opponent is defender");
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(ojer, def)],
            },
        )
        .unwrap();

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(p1, PlayerAction::DeclareBlockers { blockers: vec![] })
        .unwrap();

    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::Ending)
    });

    assert_eq!(
        engine.state().players[1].life,
        14,
        "deals 6 unblocked combat damage to opponent"
    );
}

/// Temple of Cultivation's "{2}{G}, {T}: Transform this land. Activate only
/// if you control ten or more permanents and only as a sorcery." The land is
/// placed on its back face (its dies trigger, the other way there, is not
/// written) beside nine Forests: ten permanents. It turns over into Ojer
/// Kaslem where it stands (CR 701.27a) and is the same permanent (CR 712.18):
/// still tapped from the activation, and no newcomer to its controller, so it
/// is not summoning sick (CR 302.6). The stand-in (#206) returned a new,
/// untapped, summoning-sick god.
#[test]
fn temple_of_cultivation_turns_back_into_ojer_kaslem_where_it_stands() {
    let p0 = PlayerId::new(0);
    let def = baylee_cards::by_index(ojer_kaslem_deepest_growth()).expect("in the pool");
    let mut board = vec![ojer_kaslem_deepest_growth()];
    board.extend([forest(); 9]);
    let mut engine = Duel::new(SEED, forest()).battlefield(0, &board).start();
    let temple = on_battlefield(&engine, p0, ojer_kaslem_deepest_growth()).expect("the card");
    assert!(
        engine
            .dev_state_mut(p0)
            .expect("the harness may set boards up")
            .transform(temple, def, 1),
        "placed on its back face"
    );
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");
    assert!(
        types(&engine, temple).contains(TypeSet::LAND),
        "Temple of Cultivation"
    );
    let temple_was = identity(&engine, temple);

    tap_mana_except(&mut engine, p0, temple);
    activate(&mut engine, p0, ojer_kaslem_deepest_growth(), 1);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        on_battlefield(&engine, p0, ojer_kaslem_deepest_growth()).map(|id| identity(&engine, id)),
        Some(temple_was),
        "the same object, turned over: a transform changes no zone (CR 712.18)"
    );
    assert_eq!(
        engine.state().object(temple).map(|o| o.face_index),
        Some(0),
        "Ojer Kaslem again"
    );
    assert_eq!(pt(&engine, temple), (6, 5), "the god's own 6/5");
    assert!(
        is_tapped(&engine, temple),
        "tapped for the activation: turning over is not entering"
    );
    let god = engine.state().object(temple).expect("the god");
    assert!(
        !crate::combat::summoning_sick(engine.state(), god),
        "under p0's control since the turn began"
    );
}
