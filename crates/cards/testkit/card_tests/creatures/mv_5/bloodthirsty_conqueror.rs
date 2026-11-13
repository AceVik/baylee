//! `cards/creatures/mv_5/bloodthirsty_conqueror.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn conqueror() -> CardIndex {
    card_index("fd505c02-c59e-476d-8e88-35da862ddc23")
}

fn sign_in_blood() -> CardIndex {
    card_index("c6207f6a-a624-4754-88f5-dbe700c841ff")
}

fn phyrexian_purge() -> CardIndex {
    card_index("df032801-fccf-4bff-aa88-d01650b6c81e")
}

#[test]
fn bloodthirsty_conqueror_plays_flying_and_deathtouch_in_combat() {
    let mut e = lk::game(
        SEED,
        &[conqueror()],
        &[giant_spider(), grizzly_bears()],
        &[],
        &[],
    );
    let attacker = lk::obj(&e, P0, conqueror());
    let spider = lk::obj(&e, P1, giant_spider());
    let bears = lk::obj(&e, P1, grizzly_bears());
    let keywords = keywords(&e, attacker);
    assert!(keywords.contains(KeywordSet::FLYING));
    assert!(keywords.contains(KeywordSet::DEATHTOUCH));

    let offer = lk::attack(&mut e, &[attacker]);
    assert!(lk::may_block(&offer, spider, attacker), "reach can block");
    assert!(
        !lk::may_block(&offer, bears, attacker),
        "the ground creature cannot block flying"
    );
    lk::block(&mut e, &[(spider, attacker)]);
    lk::through_combat(&mut e);

    assert_eq!(lk::zone_of(&e, spider), Zone::Graveyard);
    assert_eq!(lk::zone_of(&e, attacker), Zone::Battlefield);
    assert_eq!(lk::damage_on(&e, attacker), 2);
    assert_eq!(lk::life(&e, P1), 20, "the Spider blocked all five damage");
}

#[test]
fn bloodthirsty_conqueror_gains_exactly_the_life_an_opponent_loses() {
    let mut e = lk::game(SEED, &[conqueror()], &[], &[sign_in_blood()], &[]);
    lk::float(&mut e, P0, &[(ManaColor::Black, 2)]);
    cast_with_floating(&mut e, P0, sign_in_blood());
    let Pending::ChoosePlayer { player, .. } = e.pending().clone() else {
        panic!("Sign in Blood asks for a player: {:?}", e.pending())
    };
    e.apply(player, PlayerAction::ChoosePlayer(P1)).unwrap();
    lk::settle(&mut e);

    assert_eq!(lk::life(&e, P1), 18);
    assert_eq!(lk::life(&e, P0), 22, "two lost life becomes two gained");
}

#[test]
fn bloodthirsty_conqueror_also_triggers_on_an_opponents_life_payment() {
    let mut e = lk::game(SEED, &[conqueror()], &[], &[], &[phyrexian_purge()]);
    reach_their_main_phase(&mut e, P1);
    let conqueror = lk::obj(&e, P0, conqueror());
    lk::float(
        &mut e,
        P1,
        &[
            (ManaColor::Colorless, 2),
            (ManaColor::Black, 1),
            (ManaColor::Red, 1),
        ],
    );
    lk::cast_at(&mut e, P1, phyrexian_purge(), &[conqueror], &[]);

    assert_eq!(
        lk::life(&e, P0),
        23,
        "the three-life additional cost triggered while Conqueror was present"
    );
    assert_eq!(lk::life(&e, P1), 17);
    assert_eq!(lk::zone_of(&e, conqueror), Zone::Graveyard);
}

#[test]
fn bloodthirsty_conqueror_ignores_its_controllers_own_life_loss() {
    let mut e = lk::game(SEED, &[conqueror()], &[], &[sign_in_blood()], &[]);
    lk::float(&mut e, P0, &[(ManaColor::Black, 2)]);
    cast_with_floating(&mut e, P0, sign_in_blood());
    e.apply(P0, PlayerAction::ChoosePlayer(P0)).unwrap();
    lk::settle(&mut e);

    assert_eq!(
        lk::life(&e, P0),
        18,
        "no trigger replaces or refunds this loss"
    );
    assert_eq!(lk::life(&e, P1), 20);
}
