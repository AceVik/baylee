//! `cards/instants/mv_2/sylvan_might.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;
use baylee_core::generated::index;

fn sylvan_might() -> CardIndex {
    card_index("c8187c8d-23fa-436f-9efa-ea4cb80cdc08")
}

#[track_caller]
fn start_flashback(engine: &mut Engine<RegistryLookup>, player: PlayerId, might: ObjectId) {
    engine
        .apply(player, PlayerAction::CastSpell { card: might })
        .unwrap();
    if matches!(engine.pending(), Pending::ChooseCastMode { .. }) {
        let flashback = choose_cast_kind(engine, CastModeKind::Flashback);
        engine
            .apply(player, PlayerAction::ChooseMode(flashback))
            .unwrap();
    }
}

/// {1}{G} gives only the targeted creature +2/+2 and trample. A 4/4 attacker
/// blocked by a 2/2 assigns two lethal damage and tramples the other two over
/// to the defending player. Both modifications expire in cleanup (CR 702.19b).
#[test]
fn sylvan_might_pumps_one_creature_and_tramples_over_a_blocker() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), index::GRIZZLY_BEARS])
        .battlefield(1, &[index::GRAY_OGRE])
        .hand(0, &[sylvan_might()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let bears = on_battlefield(&engine, p0, index::GRIZZLY_BEARS).unwrap();
    let ogre = on_battlefield(&engine, p1, index::GRAY_OGRE).unwrap();

    cast_from_hand(&mut engine, p0, sylvan_might());
    let Pending::ChooseTargets {
        options,
        player_options,
        ..
    } = engine.pending().clone()
    else {
        panic!("expected a creature target, got {:?}", engine.pending());
    };
    assert!(options.contains(&bears) && options.contains(&ogre));
    assert!(player_options.is_empty(), "a player is not a creature");
    aim_at(&mut engine, p0, bears);
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, bears), (4, 4));
    assert!(keywords(&engine, bears).contains(KeywordSet::TRAMPLE));
    assert_eq!(pt(&engine, ogre), (2, 2));
    assert!(!keywords(&engine, ogre).contains(KeywordSet::TRAMPLE));
    assert!(in_graveyard(&engine, p0, sylvan_might()).is_some());

    let offer = lk::attack(&mut engine, &[bears]);
    assert!(lk::may_block(&offer, ogre, bears));
    lk::block(&mut engine, &[(ogre, bears)]);
    lk::through_combat(&mut engine);
    assert_eq!(life_of(&engine, p1), 18, "two damage tramples over");
    assert_eq!(lk::damage_on(&engine, bears), 2);
    assert_eq!(engine.state().object(ogre).unwrap().zone, Zone::Graveyard);

    reach_their_main_phase(&mut engine, p1);
    assert_eq!(pt(&engine, bears), (2, 2), "the pump ended in cleanup");
    assert!(!keywords(&engine, bears).contains(KeywordSet::TRAMPLE));
    assert_eq!(lk::damage_on(&engine, bears), 0);
}

/// Flashback replaces {1}{G} with {2}{G}{G}, applies the same +2/+2 and
/// trample to the chosen creature, then exiles the exact graveyard card.
#[test]
fn sylvan_might_flashback_costs_four_pumps_and_exiles() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[forest(), forest(), forest(), forest(), index::GRIZZLY_BEARS],
        )
        .battlefield(1, &[index::GRAY_OGRE])
        .hand(0, &[sylvan_might()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let bears = on_battlefield(&engine, p0, index::GRIZZLY_BEARS).unwrap();
    let ogre = on_battlefield(&engine, p1, index::GRAY_OGRE).unwrap();
    let might = hand_to_graveyard(&mut engine, p0, sylvan_might());

    tap_all_mana(&mut engine, p0);
    start_flashback(&mut engine, p0, might);
    aim_at(&mut engine, p0, bears);
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);
    assert_eq!(engine.state().object(might).unwrap().zone, Zone::Stack);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, bears), (4, 4));
    assert!(keywords(&engine, bears).contains(KeywordSet::TRAMPLE));
    assert_eq!(pt(&engine, ogre), (2, 2));
    assert!(!keywords(&engine, ogre).contains(KeywordSet::TRAMPLE));
    assert_eq!(engine.state().object(might).unwrap().zone, Zone::Exile);
    assert!(in_graveyard(&engine, p0, sylvan_might()).is_none());
}

/// Three green mana cannot pay {2}{G}{G}, and the cheaper ordinary cost does
/// not make the card available from the graveyard.
#[test]
fn sylvan_might_flashback_is_not_offered_with_only_three_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest(), index::GRIZZLY_BEARS])
        .hand(0, &[sylvan_might()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let might = hand_to_graveyard(&mut engine, p0, sylvan_might());
    tap_all_mana(&mut engine, p0);

    let Pending::Priority { legal, .. } = engine.pending() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(!legal.castable.contains(&might));
    assert!(!legal.payable.contains(&might));
    assert!(
        engine
            .apply(p0, PlayerAction::CastSpell { card: might })
            .is_err()
    );
    assert_eq!(engine.state().object(might).unwrap().zone, Zone::Graveyard);
    assert_eq!(engine.state().players[0].mana_pool.total(), 3);
}
