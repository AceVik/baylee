//! `cards/enchantments/mv_2/nowhere_to_run.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn nowhere() -> CardIndex {
    card_index("da82ca96-a613-4d00-9e6b-1fece4fc23d0")
}

fn twining_twins() -> CardIndex {
    card_index("105aea98-8eb9-4fb2-a0cb-7c7513317c5b")
}

#[test]
fn nowhere_flashes_in_and_debuffs_only_an_opponents_creature() {
    let mut e = lk::game(
        SEED,
        &[grizzly_bears()],
        &[rootbreaker_wurm()],
        &[nowhere()],
        &[],
    );
    reach_their_main_phase(&mut e, P1);
    e.apply(P1, PlayerAction::PassPriority).unwrap();
    lk::float(
        &mut e,
        P0,
        &[(ManaColor::Colorless, 1), (ManaColor::Black, 1)],
    );
    cast_with_floating(&mut e, P0, nowhere());
    lk::settle(&mut e);

    let Pending::ChooseTargets { options, .. } = e.pending().clone() else {
        panic!("the ETB trigger asks for its target: {:?}", e.pending())
    };
    let theirs = lk::obj(&e, P1, rootbreaker_wurm());
    let ours = lk::obj(&e, P0, grizzly_bears());
    assert_eq!(options, vec![theirs], "our creature is not a legal target");
    lk::aim(&mut e, &[theirs], &[]);
    lk::settle(&mut e);

    let projected = e.state().object(theirs).unwrap().characteristics();
    assert_eq!((projected.power, projected.toughness), (Some(3), Some(3)));
    let own = e.state().object(ours).unwrap().characteristics();
    assert_eq!((own.power, own.toughness), (Some(2), Some(2)));
}

#[test]
fn nowhere_opens_hexproof_and_stops_ward_without_removing_either_ability() {
    let mut e = lk::game(
        SEED,
        &[nowhere()],
        &[sylvan_caryatid(), twining_twins()],
        &[swords_to_plowshares(), swords_to_plowshares()],
        &[],
    );
    let caryatid = lk::obj(&e, P1, sylvan_caryatid());
    let twins = lk::obj(&e, P1, twining_twins());

    for card in [sylvan_caryatid(), twining_twins()] {
        let target = if card == sylvan_caryatid() {
            caryatid
        } else {
            twins
        };
        lk::float(&mut e, P0, &[(ManaColor::White, 1)]);
        lk::cast_at(&mut e, P0, swords_to_plowshares(), &[target], &[]);
        assert_eq!(lk::zone_of(&e, target), Zone::Exile);
        assert!(
            !matches!(e.pending(), Pending::YesNo { .. }),
            "no Ward payment interrupts either removal spell"
        );
    }
    assert!(
        e.state()
            .object(twins)
            .unwrap()
            .ability_list(e.state().bases.as_ref())
            .abilities
            .iter()
            .any(|ability| matches!(ability, baylee_cards_dsl::AbilityDef::Ward { .. })),
        "the Ward ability is suppressed, not erased"
    );
}
