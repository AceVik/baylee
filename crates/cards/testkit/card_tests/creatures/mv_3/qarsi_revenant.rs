//! `cards/creatures/mv_3/qarsi_revenant.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn revenant() -> CardIndex {
    card_index("bbea79bc-71d4-4100-92f4-783c2c4b46da")
}

#[test]
fn qarsi_revenant_plays_all_three_keywords_in_combat() {
    let mut e = lk::game(
        SEED,
        &[revenant()],
        &[giant_spider(), grizzly_bears()],
        &[],
        &[],
    );
    let attacker = lk::obj(&e, P0, revenant());
    let spider = lk::obj(&e, P1, giant_spider());
    let bears = lk::obj(&e, P1, grizzly_bears());
    let offer = lk::attack(&mut e, &[attacker]);
    assert!(lk::may_block(&offer, spider, attacker));
    assert!(!lk::may_block(&offer, bears, attacker));
    lk::block(&mut e, &[(spider, attacker)]);
    lk::through_combat(&mut e);

    assert_eq!(lk::zone_of(&e, spider), Zone::Graveyard, "deathtouch");
    assert_eq!(lk::zone_of(&e, attacker), Zone::Battlefield);
    assert_eq!(lk::damage_on(&e, attacker), 2);
    assert_eq!(lk::life(&e, P0), 23, "lifelink gains the three dealt");
}

#[test]
fn qarsi_renew_pays_its_exact_cost_exiles_itself_and_grants_three_counters() {
    let mut e = lk::game(SEED, &[grizzly_bears()], &[], &[revenant()], &[]);
    let source = hand_to_graveyard(&mut e, P0, revenant());
    let bears = lk::obj(&e, P0, grizzly_bears());
    assert!(!lk::offered(&e, revenant(), P0, 0), "no mana, no Renew");

    lk::float(&mut e, P0, &[(ManaColor::Colorless, 2)]);
    assert!(
        !lk::offered(&e, revenant(), P0, 0),
        "the black mana is not optional"
    );
    lk::float(&mut e, P0, &[(ManaColor::Black, 1)]);
    assert!(lk::offered(&e, revenant(), P0, 0));

    activate(&mut e, P0, revenant(), 0);
    let Pending::ChooseTargets { options, .. } = e.pending().clone() else {
        panic!("Renew asks for a creature: {:?}", e.pending())
    };
    assert!(options.contains(&bears));
    lk::aim(&mut e, &[bears], &[]);
    assert_eq!(lk::zone_of(&e, source), Zone::Exile, "the cost is paid");
    assert_eq!(lk::pool_total(&e, P0), 0, "exactly {{2}}{{B}} was spent");
    lk::settle(&mut e);

    for kind in [
        CounterKind::Flying,
        CounterKind::Deathtouch,
        CounterKind::Lifelink,
    ] {
        assert_eq!(counters_on(&e, bears, kind), 1, "{kind:?}");
    }
    let granted = keywords(&e, bears);
    assert!(granted.contains(KeywordSet::FLYING));
    assert!(granted.contains(KeywordSet::DEATHTOUCH));
    assert!(granted.contains(KeywordSet::LIFELINK));
}

#[test]
fn qarsi_renew_is_not_offered_on_an_opponents_turn() {
    let mut e = lk::game(SEED, &[grizzly_bears()], &[], &[revenant()], &[]);
    hand_to_graveyard(&mut e, P0, revenant());
    lk::float(
        &mut e,
        P0,
        &[(ManaColor::Colorless, 2), (ManaColor::Black, 1)],
    );
    reach_their_main_phase(&mut e, P1);
    e.apply(P1, PlayerAction::PassPriority).unwrap();

    assert!(matches!(e.pending(), Pending::Priority { player: P0, .. }));
    assert!(
        !lk::offered(&e, revenant(), P0, 0),
        "Renew is sorcery-speed even with mana and a legal target"
    );
}
