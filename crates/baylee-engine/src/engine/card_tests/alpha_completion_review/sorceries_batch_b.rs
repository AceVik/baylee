//! Pending batch B review; registered after its implementation milestone.

#[allow(clippy::wildcard_imports)]
use super::*;

fn fireball() -> CardIndex {
    card_index("aa7714b0-2bfb-458a-8ebf-37ec2c53383e")
}

fn announce_fireball(
    engine: &mut Engine<RegistryLookup>,
    caster: PlayerId,
    x: u32,
    objects: &[ObjectId],
    players: &[PlayerId],
) {
    cast_from_hand(engine, caster, fireball());
    for _ in 0..4 {
        match engine.pending().clone() {
            Pending::ChooseNumber {
                player, min, max, ..
            } => {
                assert!(min <= x && x <= max);
                engine.apply(player, PlayerAction::ChooseNumber(x)).unwrap();
            }
            Pending::ChooseTargets { .. } => aim(engine, objects.to_vec(), players.to_vec()),
            Pending::Priority { .. } => return,
            pending => panic!("unexpected Fireball announcement: {pending:?}"),
        }
    }
    panic!("announcement never finished");
}

#[test]
fn fireball_independent_mixed_targets_pay_extra_cost_and_split_rounded_down() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let body = card_index("8f1dae40-b307-446e-bbd2-86aa35813871");
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(); 10])
        .hand(0, &[fireball()])
        .battlefield(1, &[body])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let target = on_battlefield(&engine, p1, body).unwrap();
    announce_fireball(&mut engine, p0, 5, &[target], &[p0, p1]);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "five X plus R plus two extra targets costs eight"
    );
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().players[0].life, 19);
    assert_eq!(engine.state().players[1].life, 19);
    assert_eq!(
        engine.state().object(target).unwrap().damage,
        1,
        "five divided among three rounds down to one each"
    );
}

#[test]
fn fireball_independent_division_counts_only_targets_still_legal_on_resolution() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let body = card_index("8f1dae40-b307-446e-bbd2-86aa35813871");
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(); 8])
        .hand(0, &[fireball()])
        .battlefield(1, &[body, body, island()])
        .hand(1, &[unsummon()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let targets = all_on_battlefield(&engine, p1, body);
    announce_fireball(&mut engine, p0, 5, &targets, &[p1]);
    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    cast_from_hand(&mut engine, p1, unsummon());
    aim(&mut engine, vec![targets[0]], vec![]);
    pass_until(&mut engine, stack_is_empty);
    assert!(in_hand(&engine, p1, body).is_some());
    assert_eq!(engine.state().object(targets[1]).unwrap().damage, 2);
    assert_eq!(
        engine.state().players[1].life,
        18,
        "five divided among remaining two targets rounds down to two each"
    );
}

#[test]
fn fireball_independent_zero_targets_is_legal_for_nonzero_x() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(); 6])
        .hand(0, &[fireball()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    announce_fireball(&mut engine, p0, 4, &[], &[]);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "no target surcharge"
    );
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().players[0].life, 20);
    assert_eq!(engine.state().players[1].life, 20);
    assert!(in_graveyard(&engine, p0, fireball()).is_some());
}

#[test]
fn fireball_independent_more_targets_than_x_deals_zero_to_every_target() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let body = card_index("8f1dae40-b307-446e-bbd2-86aa35813871");
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(); 4])
        .hand(0, &[fireball()])
        .battlefield(1, &[body])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let creature = on_battlefield(&engine, p1, body).unwrap();
    announce_fireball(&mut engine, p0, 1, &[creature], &[p0, p1]);
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().players[0].life, 20);
    assert_eq!(engine.state().players[1].life, 20);
    assert_eq!(engine.state().object(creature).unwrap().damage, 0);
}

#[test]
fn fireball_independent_fork_copies_x_and_targets_without_paying_surcharge_again() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(); 6])
        .hand(0, &[fireball()])
        .battlefield(1, &[mountain(), mountain()])
        .hand(1, &[fork()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    announce_fireball(&mut engine, p0, 4, &[], &[p0, p1]);
    let original = top(&engine);
    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    cast_from_hand(&mut engine, p1, fork());
    aim(&mut engine, vec![original], vec![]);
    assert_eq!(engine.state().players[1].mana_pool.total(), 0);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    // Keeping both original targets means the copy itself has two targets.
    // Its controller has spent its only two mana on Fork and cannot pay even
    // the one-mana Fireball surcharge if copying were wrongly treated as casting.
    aim(&mut engine, vec![], vec![]);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().players[0].life, 16);
    assert_eq!(engine.state().players[1].life, 16);
    assert_eq!(engine.state().players[1].mana_pool.total(), 0);
    assert!(in_graveyard(&engine, p0, fireball()).is_some());
    assert!(in_graveyard(&engine, p1, fork()).is_some());
}

#[test]
fn fireball_independent_all_targets_illegal_deals_no_damage() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let body = card_index("8f1dae40-b307-446e-bbd2-86aa35813871");
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(); 5])
        .hand(0, &[fireball()])
        .battlefield(1, &[body, island()])
        .hand(1, &[unsummon()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let target = on_battlefield(&engine, p1, body).unwrap();
    announce_fireball(&mut engine, p0, 4, &[target], &[]);
    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    cast_from_hand(&mut engine, p1, unsummon());
    aim(&mut engine, vec![target], vec![]);
    pass_until(&mut engine, stack_is_empty);
    assert!(in_hand(&engine, p1, body).is_some());
    assert!(in_graveyard(&engine, p0, fireball()).is_some());
    assert_eq!(engine.state().players[0].life, 20);
    assert_eq!(engine.state().players[1].life, 20);
}

#[test]
fn fireball_independent_duplicate_player_refused_then_surcharge_reverses_unpayable_cast() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(); 4])
        .hand(0, &[fireball()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    cast_from_hand(&mut engine, p0, fireball());
    engine.apply(p0, PlayerAction::ChooseNumber(3)).unwrap();
    assert!(
        engine
            .apply(
                p0,
                PlayerAction::ChooseTargets {
                    objects: vec![],
                    players: vec![p1, p1],
                }
            )
            .is_err(),
        "a duplicate target is not a second payment option"
    );
    assert!(matches!(engine.pending(), Pending::ChooseTargets { .. }));
    aim(&mut engine, vec![], vec![p0, p1]);
    if engine.payment_window().is_some() {
        engine.apply(p0, PlayerAction::PassPriority).unwrap();
    }
    assert!(matches!(engine.pending(), Pending::Priority { player, .. } if *player == p0));
    assert!(
        in_hand(&engine, p0, fireball()).is_some(),
        "X=3, R, and an extra target costs five"
    );
    assert!(engine.state().zones.stack_is_empty());
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        4,
        "reversed casting cannot debit the surcharge partially"
    );
    assert_eq!(engine.state().players[0].life, 20);
    assert_eq!(engine.state().players[1].life, 20);
}

#[test]
fn fireball_independent_extra_target_can_be_paid_after_targets_with_untapped_land() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(); 5])
        .hand(0, &[fireball()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let lands = all_on_battlefield(&engine, p0, mountain());
    for &source in &lands[..4] {
        engine
            .apply(p0, PlayerAction::ActivateManaAbility { source })
            .unwrap();
    }
    cast_with_floating(&mut engine, p0, fireball());
    engine.apply(p0, PlayerAction::ChooseNumber(3)).unwrap();
    aim(&mut engine, vec![], vec![p0, p1]);
    assert!(
        engine.payment_window().is_some(),
        "X and R were floating, but the extra target adds one"
    );
    assert!(!is_tapped(&engine, lands[4]));
    engine
        .apply(p0, PlayerAction::ActivateManaAbility { source: lands[4] })
        .unwrap();
    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    assert!(on_stack(&engine, fireball()).is_some());
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().players[0].life, 19);
    assert_eq!(engine.state().players[1].life, 19);
}

#[test]
fn fireball_independent_any_number_includes_256_distinct_creatures() {
    use super::super::super::synthetic::{self, SyntheticLookup};
    use baylee_cards_dsl::{AbilityDef, Effect, mana_ability};
    const SOURCE: u32 = 4_000_033;
    static MANA: &[AbilityDef] = &[mana_ability!(&[Effect::mana(ManaColor::Red, 256)])];
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let body = card_index("8f1dae40-b307-446e-bbd2-86aa35813871");
    let mut preset = synthetic::preset_both(SEED, &[SOURCE], &[body.get(); 256]);
    preset.seats[0].starting_hand = Some(vec![baylee_core::preset::DeckEntry {
        card: fireball(),
        print: baylee_core::ids::PrintRef::new(0),
    }]);
    let mut engine = Engine::new(
        &preset,
        SyntheticLookup::new(vec![synthetic::land(
            SOURCE,
            "Large target mana fixture",
            MANA,
        )]),
    )
    .unwrap();
    synthetic::keep_mulligans(&mut engine);
    for _ in 0..20 {
        if engine.state().turn.phase == Phase::FirstMain {
            break;
        }
        let pending = engine.pending().clone();
        assert!(synthetic::walk_past(&mut engine, &pending));
    }
    let source = synthetic::permanents(&engine, SOURCE)[0];
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source,
                ability_index: 0,
            },
        )
        .unwrap();
    assert_eq!(engine.state().players[0].mana_pool.total(), 256);
    let card = engine.state().zones.list(ZoneLocation::Hand(p0))[0];
    engine.apply(p0, PlayerAction::CastSpell { card }).unwrap();
    engine.apply(p0, PlayerAction::ChooseNumber(0)).unwrap();
    let targets = synthetic::permanents(&engine, body.get());
    assert_eq!(targets.len(), 256);
    assert!(
        matches!(engine.pending(), Pending::ChooseTargets { max, .. } if *max >= 256),
        "any number cannot stop at 255 targets"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: targets.clone(),
                players: vec![],
            },
        )
        .unwrap();
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "256 targets add 255 generic to R at X=0"
    );
    for _ in 0..20 {
        if engine.state().zones.stack_is_empty() {
            break;
        }
        let pending = engine.pending().clone();
        assert!(synthetic::walk_past(&mut engine, &pending));
    }
    assert!(engine.state().zones.stack_is_empty());
    assert_eq!(synthetic::permanents(&engine, body.get()).len(), 256);
    assert!(
        targets
            .iter()
            .all(|target| engine.state().object(*target).unwrap().damage == 0)
    );
    assert_eq!(engine.state().players[0].life, 20);
    assert_eq!(engine.state().players[1].life, 20);
    assert_eq!(engine.state().object(targets[0]).unwrap().controller, p1);
}
