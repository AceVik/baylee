//! `cards/creatures/mv_1/mtenda_lion.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::ids::Defender;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

// oracle_id = "af029853-cfb0-403b-af51-141ba02ae2e4"
fn mtenda_lion() -> CardIndex {
    card_index("af029853-cfb0-403b-af51-141ba02ae2e4")
}

#[test]
fn mtenda_lion_attack_trigger_lets_defender_pay_u_to_prevent_its_combat_damage() {
    let mut engine = lk::game(SEED, &[mtenda_lion()], &[island()], &[], &[]);
    let lion = lk::obj(&engine, P0, mtenda_lion());

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            P0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(lion, Defender::Player(P1))],
            },
        )
        .unwrap();

    lk::float(&mut engine, P1, &[(ManaColor::Blue, 1)]);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    engine.apply(P1, PlayerAction::YesNo(true)).unwrap();

    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::SecondMain)
    });
    assert_eq!(
        engine.state().players[1].life,
        20,
        "Defending player paid {{U}} so Mtenda Lion's combat damage was prevented"
    );
}

/// "May pay {U}": a defender with the mana who declines takes the 2 damage.
#[test]
fn mtenda_lion_deals_its_combat_damage_when_the_defender_declines_to_pay() {
    let mut engine = lk::game(SEED, &[mtenda_lion()], &[island()], &[], &[]);
    let lion = lk::obj(&engine, P0, mtenda_lion());

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            P0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(lion, Defender::Player(P1))],
            },
        )
        .unwrap();

    lk::float(&mut engine, P1, &[(ManaColor::Blue, 1)]);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    engine.apply(P1, PlayerAction::YesNo(false)).unwrap();

    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::SecondMain)
    });
    assert_eq!(
        engine.state().players[1].life,
        18,
        "the defender declined, so Mtenda Lion's 2 combat damage was dealt"
    );
}
