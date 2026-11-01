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

/// The pinned limitation: "You may sacrifice two Mountains rather than pay
/// this spell's mana cost" is not on the card. With only the mana of two
/// Mountains, Fireblast is not offered and a cast attempt is refused; the
/// Mountains stay.
#[test]
fn fireblast_cannot_be_cast_for_two_sacrificed_mountains() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[mountain(), mountain()])
        .battlefield(1, &[grizzly_bears()])
        .hand(0, &[fireblast()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let spell = in_hand(&engine, p0, fireblast()).unwrap();
    tap_all_mana(&mut engine, p0);

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.castable.contains(&spell),
        "{{R}}{{R}} does not pay {{4}}{{R}}{{R}}, and no alternative cost is offered: {legal:?}"
    );
    assert!(
        engine
            .apply(p0, PlayerAction::CastSpell { card: spell })
            .is_err(),
        "the cast is refused"
    );
    assert!(in_hand(&engine, p0, fireblast()).is_some());
    assert_eq!(mountains_of(&engine, p0), 2, "no Mountain was sacrificed");
}
