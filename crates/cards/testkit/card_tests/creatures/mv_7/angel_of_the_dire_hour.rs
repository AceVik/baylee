//! `cards/creatures/mv_7/angel_of_the_dire_hour.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn angel_of_the_dire_hour() -> CardIndex {
    card_index("8b1e6b7c-892d-4058-b183-9121cdd623f0")
}

#[test]
fn angel_of_the_dire_hour_flashes_in_after_attacks_and_exiles_only_attackers() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[plains(); 7])
        .hand(0, &[angel_of_the_dire_hour()])
        .battlefield(1, &[gray_ogre(), gray_ogre()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    reach_their_main_phase(&mut e, p1);
    let ogres = all_on_battlefield(&e, p1, gray_ogre());
    let attacker = ogres[0];
    let nonattacker = ogres[1];
    pass_until(
        &mut e,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == p1),
    );
    e.apply(
        p1,
        PlayerAction::DeclareAttackers {
            attackers: vec![(attacker, Defender::Player(p0))],
        },
    )
    .expect("one Ogre attacks");
    pass_until(
        &mut e,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p0),
    );
    tap_all_mana(&mut e, p0);
    cast_with_floating(&mut e, p0, angel_of_the_dire_hour());
    pass_until(&mut e, stack_is_empty);
    assert_eq!(
        e.state().object(attacker).expect("attacker").zone,
        Zone::Exile
    );
    assert_eq!(
        e.state().object(nonattacker).expect("nonattacker").zone,
        Zone::Battlefield
    );
    let angel = on_battlefield(&e, p0, angel_of_the_dire_hour()).expect("the Angel");
    assert!(keywords(&e, angel).contains(KeywordSet::FLASH));
    assert!(keywords(&e, angel).contains(KeywordSet::FLYING));
}
