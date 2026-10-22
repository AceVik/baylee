//! `cards/instants/mv_2/gorilla_war_cry.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn gorilla_war_cry() -> CardIndex {
    card_index("60a7f61c-41ad-4dce-b556-a99bf38c3d73")
}

#[test]
fn gorilla_war_cry_grants_menace_before_blockers_and_draws_on_next_upkeep() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(), mountain(), grizzly_bears()])
        .hand(0, &[gorilla_war_cry()])
        .start();

    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    reach_their_main_phase(&mut engine, p0);

    let spell = in_hand(&engine, p0, gorilla_war_cry()).unwrap();
    assert!(
        engine
            .apply(p0, PlayerAction::CastSpell { card: spell })
            .is_err(),
        "Gorilla War Cry cannot be cast during a main phase"
    );

    let bears = on_battlefield(&engine, p0, grizzly_bears()).unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(bears, Defender::Player(p1))],
            },
        )
        .unwrap();

    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    cast_from_hand(&mut engine, p0, gorilla_war_cry());
    pass_until(&mut engine, stack_is_empty);

    assert!(
        keywords(&engine, bears).contains(KeywordSet::MENACE),
        "Gorilla War Cry should grant menace to creatures until end of turn"
    );

    pass_until(&mut engine, |e| {
        e.state().turn.active == p1 && e.state().turn.step == Step::Upkeep
    });
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before,
        "Gorilla War Cry should draw a card at the beginning of the next turn's upkeep"
    );
}
