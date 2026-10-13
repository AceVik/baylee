#[allow(clippy::wildcard_imports)]
use super::*;

#[test]
fn batch45_kess_consumes_one_permission_only_on_a_successful_cast() {
    let mut e = prepared(
        &[index::GIANT_GROWTH, index::COUNSEL_OF_THE_SORATAMI],
        &[index::KESS_DISSIDENT_MAGE, index::GRIZZLY_BEARS],
        &[],
    );
    let spell = hand_to_graveyard(&mut e, P0, index::GIANT_GROWTH);
    let other = hand_to_graveyard(&mut e, P0, index::COUNSEL_OF_THE_SORATAMI);
    let bear = on_battlefield(&e, P0, index::GRIZZLY_BEARS).unwrap();
    cast(&mut e, spell);
    e.apply(P0, PlayerAction::CancelCast).unwrap();
    assert!(castable(&e, spell));
    cast(&mut e, spell);
    target(&mut e, vec![bear], vec![]);
    assert!(!castable(&e, other), "the Kess permission was consumed");
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state.object(spell).unwrap().zone, Zone::Exile);
    assert_eq!(pt(&e, bear), (5, 5));
}

#[test]
fn batch45_breach_can_escape_repeatedly_without_flashback_exiling_the_spell() {
    let mut e = prepared(
        &[index::COUNSEL_OF_THE_SORATAMI],
        &[index::UNDERWORLD_BREACH],
        &[],
    );
    let spell = hand_to_graveyard(&mut e, P0, index::COUNSEL_OF_THE_SORATAMI);
    seed_graveyard(&mut e, P0, 6);
    for _ in 0..2 {
        cast(&mut e, spell);
        let offered = cards_offered(&e);
        assert!(!offered.contains(&spell));
        choose(&mut e, offered[..3].to_vec());
        assert!(
            e.state
                .object(spell)
                .unwrap()
                .riders
                .contains(&Rider::Escaped)
        );
        pass_until(&mut e, stack_is_empty);
        assert_eq!(e.state.object(spell).unwrap().zone, Zone::Graveyard);
    }
    assert!(!castable(&e, spell));
}

#[test]
fn batch45_breach_exiles_a_card_sacrificed_for_the_same_spell() {
    let mut e = prepared(
        &[index::CROP_ROTATION],
        &[index::UNDERWORLD_BREACH, index::FOREST],
        &[],
    );
    let spell = hand_to_graveyard(&mut e, P0, index::CROP_ROTATION);
    let forest = on_battlefield(&e, P0, index::FOREST).unwrap();
    seed_graveyard(&mut e, P0, 2);
    assert!(castable(&e, spell));
    cast(&mut e, spell);
    choose(&mut e, vec![forest]);
    let offered = cards_offered(&e);
    assert!(offered.contains(&forest));
    choose(&mut e, offered);
    assert_eq!(e.state.object(forest).unwrap().zone, Zone::Exile);
    pass_until(&mut e, library_choices::searched);
    choose(&mut e, vec![]);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state.object(spell).unwrap().zone, Zone::Graveyard);
}

#[test]
fn batch45_beam_me_up_returns_a_controlled_creature_as_a_cost_and_exiles_afterward() {
    let mut e = prepared(
        &[index::OPEN_COMMUNICATIONS],
        &[index::GRIZZLY_BEARS],
        &[index::LLANOWAR_ELVES],
    );
    let spell = hand_to_graveyard(&mut e, P0, index::OPEN_COMMUNICATIONS);
    let bear = on_battlefield(&e, P0, index::GRIZZLY_BEARS).unwrap();
    cast(&mut e, spell);
    assert_eq!(cards_offered(&e), vec![bear]);
    choose(&mut e, vec![bear]);
    assert_eq!(e.state.object(bear).unwrap().zone, Zone::Hand);
    assert_eq!(e.state.object(spell).unwrap().zone, Zone::Stack);
    let before = e.state.zones.list(ZoneLocation::Hand(P0)).len();
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state.zones.list(ZoneLocation::Hand(P0)).len(), before + 1);
    assert_eq!(e.state.object(spell).unwrap().zone, Zone::Exile);
}

#[test]
fn batch45_agadeem_rejects_duplicate_mana_values_without_mutating_state() {
    let mut e = prepared(
        &[
            index::AGADEEM_S_AWAKENING,
            index::GRIZZLY_BEARS,
            index::CANOPY_SPIDER,
            index::LLANOWAR_ELVES,
        ],
        &[],
        &[],
    );
    let a = hand_to_graveyard(&mut e, P0, index::GRIZZLY_BEARS);
    let b = hand_to_graveyard(&mut e, P0, index::CANOPY_SPIDER);
    let c = hand_to_graveyard(&mut e, P0, index::LLANOWAR_ELVES);
    cast_with_floating(&mut e, P0, index::AGADEEM_S_AWAKENING);
    e.apply(P0, PlayerAction::ChooseNumber(2)).unwrap();
    let before = e.snapshot_hash();
    assert!(
        e.apply(
            P0,
            PlayerAction::ChooseTargets {
                objects: vec![a, b],
                players: vec![]
            }
        )
        .is_err()
    );
    assert_eq!(e.snapshot_hash(), before);
    target(&mut e, vec![a, c], vec![]);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state.object(a).unwrap().zone, Zone::Battlefield);
    assert_eq!(e.state.object(c).unwrap().zone, Zone::Battlefield);
    assert_eq!(e.state.object(b).unwrap().zone, Zone::Graveyard);
}

#[test]
fn batch45_tarrian_discards_the_hand_and_the_tomb_grants_vampire_and_finality() {
    let mut e = prepared(
        &[index::GRIZZLY_BEARS],
        &[index::TARRIAN_S_JOURNAL, index::DOUBLING_SEASON],
        &[],
    );
    let bear = in_hand(&e, P0, index::GRIZZLY_BEARS).unwrap();
    let journal = on_battlefield(&e, P0, index::TARRIAN_S_JOURNAL).unwrap();
    activate(&mut e, P0, index::TARRIAN_S_JOURNAL, 1);
    assert!(e.state.zones.list(ZoneLocation::Hand(P0)).is_empty());
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state.object(journal).unwrap().face_index, 1);
    e.state.set_tapped(journal, false);
    e.refresh_offer();
    activate(&mut e, P0, index::TARRIAN_S_JOURNAL, 1);
    pass_until(&mut e, stack_is_empty);
    cast(&mut e, bear);
    assert!(
        e.state
            .object(bear)
            .unwrap()
            .characteristics()
            .subtypes
            .contains(subtypes::creature::VAMPIRE)
    );
    pass_until(&mut e, stack_is_empty);
    assert_eq!(counters_on(&e, bear, CounterKind::Finality), 2);
    assert!(
        e.state
            .object(bear)
            .unwrap()
            .characteristics()
            .subtypes
            .contains(subtypes::creature::VAMPIRE)
    );
    kill(&mut e, bear);
    assert_eq!(e.state.object(bear).unwrap().zone, Zone::Exile);
}

/// "{T}: You may cast **a** creature spell from your graveyard this turn": one
/// spell. With two creature cards and an instant in the graveyard, the first
/// cast uses the permission up, and the second creature stays where it is. The
/// instant was never castable from there at all.
#[test]
fn batch45_tomb_of_aclazotz_permits_one_creature_spell_only() {
    let mut e = prepared(
        &[
            index::GRIZZLY_BEARS,
            index::CANOPY_SPIDER,
            index::GIANT_GROWTH,
        ],
        &[index::TARRIAN_S_JOURNAL, index::LLANOWAR_ELVES],
        &[],
    );
    let bear = in_hand(&e, P0, index::GRIZZLY_BEARS).unwrap();
    let spider = in_hand(&e, P0, index::CANOPY_SPIDER).unwrap();
    let growth = in_hand(&e, P0, index::GIANT_GROWTH).unwrap();
    let journal = on_battlefield(&e, P0, index::TARRIAN_S_JOURNAL).unwrap();
    activate(&mut e, P0, index::TARRIAN_S_JOURNAL, 1);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state.object(journal).unwrap().face_index, 1);
    for card in [bear, spider, growth] {
        assert_eq!(
            e.state.object(card).unwrap().zone,
            Zone::Graveyard,
            "the discard cost put the whole hand in the graveyard"
        );
    }
    e.state.set_tapped(journal, false);
    e.refresh_offer();
    assert!(
        !castable(&e, bear) && !castable(&e, spider),
        "before the Tomb's ability, no creature card is castable from the graveyard"
    );
    activate(&mut e, P0, index::TARRIAN_S_JOURNAL, 1);
    pass_until(&mut e, stack_is_empty);
    assert!(
        castable(&e, bear) && castable(&e, spider),
        "with the permission, either creature card may be cast"
    );
    assert!(
        !castable(&e, growth),
        "the permission is for a creature spell: the instant stays in the graveyard"
    );

    cast(&mut e, bear);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state.object(bear).unwrap().zone, Zone::Battlefield);
    assert!(
        !castable(&e, spider),
        "one creature spell was cast: the second creature card is no longer castable"
    );
    assert!(
        !castable(&e, growth),
        "and the instant still is not, with a creature on the battlefield to aim at"
    );
}
