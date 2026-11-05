//! `cards/instants/mv_6/fireblast.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn fireblast() -> CardIndex {
    card_index("9dd7f27a-e862-47d7-9158-034cf4d353b8")
}

/// How many Mountains `seat` controls.
fn mountains_of(engine: &Engine<RegistryLookup>, seat: PlayerId) -> usize {
    engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .filter(|&&id| {
            let o = engine.state().object(id).unwrap();
            o.controller == seat && o.card.is_some_and(|c| c.index == mountain())
        })
        .count()
}

/// Six Mountains and a Fireblast in P0's hand; P1 controls Grizzly Bears.
fn six_mountains_and_a_fireblast() -> Engine<RegistryLookup> {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[mountain(); 6])
        .battlefield(1, &[grizzly_bears()])
        .hand(0, &[fireblast()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, PlayerId::new(0));
    engine
}

/// `Fireblast` ({4}{R}{R}, Instant, `Coverage::Partial`): "deals 4 damage to
/// any target", here a player, who loses exactly 4 and nobody else is hurt.
/// The mana cost is paid in full and no Mountain is sacrificed.
#[test]
fn fireblast_deals_four_damage_to_a_player_for_its_mana_cost() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = six_mountains_and_a_fireblast();
    cast_from_hand(&mut engine, p0, fireblast());
    if matches!(engine.pending(), Pending::ChooseCastMode { .. }) {
        let normal = choose_cast_kind(&engine, CastModeKind::Normal);
        engine.apply(p0, PlayerAction::ChooseMode(normal)).unwrap();
    }
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().players[1].life, 16, "exactly 4 damage");
    assert_eq!(engine.state().players[0].life, 20, "the caster is spared");
    assert_eq!(
        mountains_of(&engine, p0),
        6,
        "the mana cost was paid; no Mountain was sacrificed"
    );
    assert!(
        in_graveyard(&engine, p0, fireblast()).is_some(),
        "a resolved instant goes to its owner's graveyard"
    );
    assert!(
        on_battlefield(&engine, p1, grizzly_bears()).is_some(),
        "the creature was not the target"
    );
}

/// "Any target" includes a creature: the Bears take 4, die, and the player
/// behind them is not hurt.
#[test]
fn fireblast_deals_four_damage_to_a_creature_and_not_to_its_controller() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = six_mountains_and_a_fireblast();
    let bear = on_battlefield(&engine, p1, grizzly_bears()).unwrap();
    cast_from_hand(&mut engine, p0, fireblast());
    if matches!(engine.pending(), Pending::ChooseCastMode { .. }) {
        let normal = choose_cast_kind(&engine, CastModeKind::Normal);
        engine.apply(p0, PlayerAction::ChooseMode(normal)).unwrap();
    }
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let menu = aim_at(&mut engine, p0, bear);
    assert!(
        menu.contains(&bear),
        "a creature is on the \"any target\" menu"
    );
    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p1, grizzly_bears()).is_some(),
        "4 damage kills a 2/2"
    );
    assert!(on_battlefield(&engine, p1, grizzly_bears()).is_none());
    assert_eq!(
        engine.state().players[1].life,
        20,
        "its controller is spared"
    );
}

/// The alternative cost needs exactly two different Mountains. The Forest is
/// never an answer, a Mountain already named cannot answer the second part,
/// and both permanents leave the battlefield while the spell is being cast.
#[test]
fn fireblast_sacrifices_two_distinct_mountains_instead_of_paying_mana() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[mountain(), mountain(), forest()])
        .hand(0, &[fireblast()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let spell = in_hand(&engine, p0, fireblast()).unwrap();
    let first = on_battlefield(&engine, p0, mountain()).unwrap();
    let second = engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .find(|id| {
            *id != first
                && engine
                    .state()
                    .object(*id)
                    .is_some_and(|object| object.card.is_some_and(|card| card.index == mountain()))
        })
        .unwrap();
    let forest = on_battlefield(&engine, p0, forest()).unwrap();

    engine
        .apply(p0, PlayerAction::CastSpell { card: spell })
        .unwrap();
    if matches!(engine.pending(), Pending::ChooseCastMode { .. }) {
        let alternative = choose_cast_kind(&engine, CastModeKind::Alternative(0));
        engine
            .apply(p0, PlayerAction::ChooseMode(alternative))
            .unwrap();
    }
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .unwrap();
    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        panic!("expected the first sacrifice, got {:?}", engine.pending());
    };
    assert!(options.contains(&first) && options.contains(&second));
    assert!(!options.contains(&forest), "a Forest is not a Mountain");
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![first],
            },
        )
        .unwrap();

    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        panic!("expected the second sacrifice, got {:?}", engine.pending());
    };
    assert_eq!(options, vec![second], "the first Mountain cannot pay twice");
    assert!(
        engine
            .apply(
                p0,
                PlayerAction::ChooseObjects {
                    objects: vec![first]
                }
            )
            .is_err(),
        "a stale duplicate answer is refused"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![second],
            },
        )
        .unwrap();

    assert_eq!(
        mountains_of(&engine, p0),
        0,
        "both Mountains were sacrificed"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "no mana was needed"
    );
    assert_eq!(engine.state().object(spell).unwrap().zone, Zone::Stack);
    assert!(engine.state().object(spell).unwrap().alt_cast);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().players[1].life,
        16,
        "the spell still deals 4"
    );
}

/// One Mountain cannot pay a cost written as two sacrifices, even though it
/// could produce mana. Without the printed price the card is not offered.
#[test]
fn fireblast_needs_two_mountains_for_its_alternative_cost() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[mountain()])
        .hand(0, &[fireblast()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let spell = in_hand(&engine, p0, fireblast()).unwrap();

    let result = engine.apply(p0, PlayerAction::CastSpell { card: spell });
    let alternative_offered = match result {
        Err(_) => false,
        Ok(()) => matches!(
            engine.pending(),
            Pending::ChooseCastMode { options, .. }
                if options.iter().any(|option| matches!(option.kind, CastModeKind::Alternative(_)))
        ),
    };
    assert!(!alternative_offered, "one Mountain cannot pay twice");
    assert!(in_hand(&engine, p0, fireblast()).is_some());
    assert_eq!(mountains_of(&engine, p0), 1);
}

/// The two Mountains are costs paid while casting, not an effect of resolution.
/// Countering Fireblast neither returns them nor deals its damage.
#[test]
fn fireblast_keeps_its_alternative_costs_paid_when_countered() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[mountain(), mountain()])
        .hand(0, &[fireblast()])
        .battlefield(1, &[island(), island()])
        .hand(1, &[counterspell()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let spell = in_hand(&engine, p0, fireblast()).unwrap();

    engine
        .apply(p0, PlayerAction::CastSpell { card: spell })
        .unwrap();
    if matches!(engine.pending(), Pending::ChooseCastMode { .. }) {
        let alternative = choose_cast_kind(&engine, CastModeKind::Alternative(0));
        engine
            .apply(p0, PlayerAction::ChooseMode(alternative))
            .unwrap();
    }
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .unwrap();
    for _ in 0..2 {
        let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
            panic!("expected a Mountain sacrifice, got {:?}", engine.pending());
        };
        engine
            .apply(
                p0,
                PlayerAction::ChooseObjects {
                    objects: vec![options[0]],
                },
            )
            .unwrap();
    }
    assert_eq!(mountains_of(&engine, p0), 0, "costs are already paid");
    assert_eq!(engine.state().object(spell).unwrap().zone, Zone::Stack);

    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    cast_from_hand(&mut engine, p1, counterspell());
    aim_at(&mut engine, p1, spell);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(mountains_of(&engine, p0), 0, "the costs are not refunded");
    assert_eq!(engine.state().players[1].life, 20, "no damage was dealt");
    assert!(
        in_graveyard(&engine, p0, fireblast()).is_some(),
        "the countered spell goes to the graveyard"
    );
}
