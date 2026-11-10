//! `cards/creatures/mv_5/angel_of_grace.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn angel_of_grace() -> CardIndex {
    card_index("fcfcac6b-6d26-4ffe-ae9e-b0ee7fa49787")
}

fn bolt_self(e: &mut Engine<RegistryLookup>, player: PlayerId) {
    cast_from_hand(e, player, lightning_bolt());
    e.apply(
        player,
        PlayerAction::ChooseTargets {
            objects: vec![],
            players: vec![player],
        },
    )
    .expect("Lightning Bolt targets its caster");
}

#[test]
fn angel_of_grace_floors_damage_at_one_only_for_the_entry_turn() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, forest())
        .life(0, 2)
        .battlefield(
            0,
            &[plains(), plains(), plains(), plains(), plains(), mountain()],
        )
        .hand(0, &[angel_of_grace(), lightning_bolt(), lightning_bolt()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    cast_from_hand(&mut e, p0, angel_of_grace());
    pass_until(&mut e, stack_is_empty);
    let angel = on_battlefield(&e, p0, angel_of_grace()).expect("the Angel");
    assert!(keywords(&e, angel).contains(KeywordSet::FLASH));
    assert!(keywords(&e, angel).contains(KeywordSet::FLYING));
    bolt_self(&mut e, p0);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(
        e.state().players[0].life,
        1,
        "lethal damage is reduced to one"
    );

    reach_their_main_phase(&mut e, p1);
    reach_their_main_phase(&mut e, p0);
    bolt_self(&mut e, p0);
    pass_until(&mut e, |e| matches!(e.pending(), Pending::GameOver(_)));
    assert!(
        e.state().players[0].life < 1,
        "the protection ended with the turn"
    );
}

#[test]
fn angel_of_grace_exiles_herself_from_the_graveyard_to_set_life_to_ten() {
    let p0 = PlayerId::new(0);
    let mut e = Duel::new(SEED, forest())
        .life(0, 2)
        .battlefield(0, &[plains(); 6])
        .hand(0, &[angel_of_grace()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let card = hand_to_graveyard(&mut e, p0, angel_of_grace());
    tap_all_mana(&mut e, p0);
    activate(&mut e, p0, angel_of_grace(), 1);
    assert_eq!(
        e.state().object(card).expect("exiled Angel").zone,
        Zone::Exile
    );
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state().players[0].life, 10);
}
