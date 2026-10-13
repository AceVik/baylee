//! `cards/lands/towns/lindblum_industrial_regency.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

#[test]
fn lindblum_industrial_regency_enters_tapped_and_taps_for_red() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, mountain())
        .hand(0, &[lindblum_industrial_regency()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, lindblum_industrial_regency());
    assert!(entered_tapped(&engine, land));

    let from = engine.state().turn.number;
    pass_until(&mut engine, |e| {
        e.state().turn.number > from
            && e.state().turn.active == p0
            && matches!(e.state().turn.phase, Phase::FirstMain)
    });
    assert!(!is_tapped(&engine, land));

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority");
    };
    assert!(legal.abilities.contains(&(land, 0)));

    activate(&mut engine, p0, lindblum_industrial_regency(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Red), 1);
    assert!(is_tapped(&engine, land));
}

#[test]
fn mage_siege_is_an_instant_and_its_wizard_triggers_on_later_noncreature_spells() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, forest())
        .hand(
            0,
            &[
                lindblum_industrial_regency(),
                lightning_bolt(),
                llanowar_elves(),
            ],
        )
        .start();
    keep_mulligans(&mut e);
    reach_their_main_phase(&mut e, p1);
    pass_until(
        &mut e,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p0),
    );
    let before = e.state().players[1].life;
    let card = begin_town_adventure(&mut e, p0, lindblum_industrial_regency(), ManaColor::Red, 3);
    pass_until(&mut e, stack_is_empty);
    assert_adventure_exiled(&e, p0, card);
    let wizard = tokens_of(&e, p0)[0];
    assert_eq!(pt(&e, wizard), (0, 1));
    assert_eq!(
        e.state().players[1].life,
        before,
        "the token was not there for its own creation spell"
    );
    reach_their_main_phase(&mut e, p0);
    fund_adventure(&mut e, p0, ManaColor::Green, 1);
    cast_from_hand(&mut e, p0, llanowar_elves());
    pass_until(&mut e, stack_is_empty);
    assert_eq!(
        e.state().players[1].life,
        before,
        "creature spells do not trigger it"
    );
    fund_adventure(&mut e, p0, ManaColor::Red, 1);
    cast_from_hand(&mut e, p0, lightning_bolt());
    e.apply(
        p0,
        PlayerAction::ChooseTargets {
            objects: vec![],
            players: vec![p1],
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state().players[1].life, before - 4);
}

#[test]
fn a_stolen_adventure_grants_its_controller_the_land_play_in_its_owners_exile() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, forest())
        .hand(0, &[lindblum_industrial_regency()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let card = begin_town_adventure(&mut e, p0, lindblum_industrial_regency(), ManaColor::Red, 3);
    // Isolate the resolution rule from the unrelated cost of stealing a spell.
    e.dev_state_mut(p0)
        .unwrap()
        .object_mut(card)
        .unwrap()
        .set_controller(p1);
    pass_until(&mut e, stack_is_empty);
    assert!(
        e.state()
            .zones
            .list(ZoneLocation::Exile(p0))
            .contains(&card)
    );
    assert_adventure_exiled(&e, p1, card);
    assert!(!crate::casting::adventure_permission(e.state(), p0, card));
    assert!(!crate::casting::land_card_open(e.state(), p0, card));
    reach_their_main_phase(&mut e, p1);
    let Pending::Priority { legal, .. } = e.pending() else {
        panic!("priority")
    };
    assert!(legal.lands.contains(&card));
    e.apply(p1, PlayerAction::PlayLand { card }).unwrap();
    assert_eq!(e.state().object(card).unwrap().controller, p1);
    assert_eq!(e.state().object(card).unwrap().owner, p0);
}

/// The Wizard token's "Whenever you cast a noncreature spell, this token
/// deals 1 damage to each opponent": the opponent casting a noncreature spell
/// puts nothing on the stack above their spell and costs them no life from
/// it, while the Wizard's own controller casting one still pings.
#[test]
fn mage_siege_wizard_ignores_the_opponents_noncreature_spells() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, forest())
        .hand(0, &[lindblum_industrial_regency(), lightning_bolt()])
        .hand(1, &[lightning_bolt()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    begin_town_adventure(&mut e, p0, lindblum_industrial_regency(), ManaColor::Red, 3);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(tokens_of(&e, p0).len(), 1, "the Wizard token is in play");
    let before = e.state().players[1].life;

    e.apply(p0, PlayerAction::PassPriority).unwrap();
    assert!(
        matches!(e.pending(), Pending::Priority { player, .. } if *player == p1),
        "the opponent holds priority in our main phase: {:?}",
        e.pending()
    );
    fund_adventure(&mut e, p1, ManaColor::Red, 1);
    cast_with_floating(&mut e, p1, lightning_bolt());
    e.apply(
        p1,
        PlayerAction::ChooseTargets {
            objects: vec![],
            players: vec![p0],
        },
    )
    .unwrap();
    assert_eq!(
        e.state().zones.list(ZoneLocation::Stack).len(),
        1,
        "the opponent's Lightning Bolt alone: the Wizard did not trigger on it"
    );
    pass_until(&mut e, stack_is_empty);
    assert_eq!(
        e.state().players[1].life,
        before,
        "\"Whenever you cast\": the opponent's noncreature spell did not ping them"
    );

    // The same Wizard still answers its own controller's noncreature spell.
    fund_adventure(&mut e, p0, ManaColor::Red, 1);
    cast_with_floating(&mut e, p0, lightning_bolt());
    e.apply(
        p0,
        PlayerAction::ChooseTargets {
            objects: vec![],
            players: vec![p1],
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);
    assert_eq!(
        e.state().players[1].life,
        before - 4,
        "1 from the Wizard, 3 from the Bolt"
    );
}
