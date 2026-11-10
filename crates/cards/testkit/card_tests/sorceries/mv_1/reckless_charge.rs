//! `cards/sorceries/mv_1/reckless_charge.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_core::generated::index;

fn reckless_charge() -> CardIndex {
    card_index("45bee121-0beb-4901-9473-20e4704ba6dc")
}

#[track_caller]
fn start_flashback(engine: &mut Engine<RegistryLookup>, player: PlayerId, charge: ObjectId) {
    engine
        .apply(player, PlayerAction::CastSpell { card: charge })
        .unwrap();
    if matches!(engine.pending(), Pending::ChooseCastMode { .. }) {
        let flashback = choose_cast_kind(engine, CastModeKind::Flashback);
        engine
            .apply(player, PlayerAction::ChooseMode(flashback))
            .unwrap();
    }
}

/// A creature cast this turn is summoning sick. The ordinary {R} spell gives
/// exactly that creature +3/+0 and haste, so it attacks for five immediately;
/// both modifications then end with the turn.
#[test]
fn reckless_charge_pumps_a_new_creature_and_lets_it_attack_immediately() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(), mountain(), forest()])
        .battlefield(1, &[index::GRIZZLY_BEARS])
        .hand(0, &[index::GRIZZLY_BEARS, reckless_charge()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, index::GRIZZLY_BEARS);
    pass_until(&mut engine, stack_is_empty);
    let bears = on_battlefield(&engine, p0, index::GRIZZLY_BEARS).unwrap();
    let bystander = on_battlefield(&engine, p1, index::GRIZZLY_BEARS).unwrap();
    assert_eq!(pt(&engine, bears), (2, 2));
    assert!(!keywords(&engine, bears).contains(KeywordSet::HASTE));

    cast_from_hand(&mut engine, p0, reckless_charge());
    let Pending::ChooseTargets {
        options,
        player_options,
        ..
    } = engine.pending().clone()
    else {
        panic!("expected a creature target, got {:?}", engine.pending());
    };
    assert!(options.contains(&bears) && options.contains(&bystander));
    assert!(player_options.is_empty(), "a player is not a creature");
    assert!(
        engine
            .apply(
                p0,
                PlayerAction::ChooseTargets {
                    objects: vec![],
                    players: vec![p1],
                },
            )
            .is_err()
    );
    aim_at(&mut engine, p0, bears);
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, bears), (5, 2));
    assert!(keywords(&engine, bears).contains(KeywordSet::HASTE));
    assert_eq!(pt(&engine, bystander), (2, 2));
    assert!(!keywords(&engine, bystander).contains(KeywordSet::HASTE));
    assert!(in_graveyard(&engine, p0, reckless_charge()).is_some());

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers { attackers, .. } = engine.pending().clone() else {
        unreachable!();
    };
    assert!(
        attackers.contains(&bears),
        "haste overrides this turn's summoning sickness: {attackers:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(bears, Defender::Player(p1))],
            },
        )
        .unwrap();
    reach_their_main_phase(&mut engine, p1);

    assert_eq!(life_of(&engine, p1), 15, "the pumped Bears dealt five");
    assert_eq!(pt(&engine, bears), (2, 2), "the pump ended in cleanup");
    assert!(!keywords(&engine, bears).contains(KeywordSet::HASTE));
}

/// Flashback replaces {R} with {2}{R}. The spell pumps its target again and
/// exiles this exact graveyard incarnation after resolving.
#[test]
fn reckless_charge_flashback_costs_three_and_exiles_after_pumping() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(); 3])
        .battlefield(1, &[index::GRIZZLY_BEARS])
        .hand(0, &[reckless_charge()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let charge = hand_to_graveyard(&mut engine, p0, reckless_charge());
    let bears = on_battlefield(&engine, PlayerId::new(1), index::GRIZZLY_BEARS).unwrap();

    tap_all_mana(&mut engine, p0);
    start_flashback(&mut engine, p0, charge);
    aim_at(&mut engine, p0, bears);
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);
    assert_eq!(engine.state().object(charge).unwrap().zone, Zone::Stack);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, bears), (5, 2));
    assert!(keywords(&engine, bears).contains(KeywordSet::HASTE));
    assert_eq!(engine.state().object(charge).unwrap().zone, Zone::Exile);
    assert!(in_graveyard(&engine, p0, reckless_charge()).is_none());
}

/// Two red mana cannot pay {2}{R}, and the cheaper ordinary cost is irrelevant
/// while the card is in the graveyard.
#[test]
fn reckless_charge_flashback_is_not_offered_with_only_two_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(), mountain()])
        .battlefield(1, &[index::GRIZZLY_BEARS])
        .hand(0, &[reckless_charge()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let charge = hand_to_graveyard(&mut engine, p0, reckless_charge());
    tap_all_mana(&mut engine, p0);

    let Pending::Priority { legal, .. } = engine.pending() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(!legal.castable.contains(&charge));
    assert!(!legal.payable.contains(&charge));
    assert!(
        engine
            .apply(p0, PlayerAction::CastSpell { card: charge })
            .is_err()
    );
    assert_eq!(engine.state().object(charge).unwrap().zone, Zone::Graveyard);
    assert_eq!(engine.state().players[0].mana_pool.total(), 2);
}
