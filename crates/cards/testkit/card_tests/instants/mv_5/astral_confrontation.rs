//! Astral Confrontation's price changes with the current attackers.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn astral_confrontation() -> CardIndex {
    card_index("80788ac7-19d6-42d2-9f6e-676d7b5c912e")
}

fn attack_with_bear(engine: &mut Engine<RegistryLookup>) -> ObjectId {
    let bear = on_battlefield(engine, me(), grizzly_bears()).expect("our Bear");
    pass_until(
        engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == me()),
    );
    engine
        .apply(
            me(),
            PlayerAction::DeclareAttackers {
                attackers: vec![(bear, Defender::Player(them()))],
            },
        )
        .expect("the Bear can attack the opponent");
    pass_until_priority(engine, me());
    bear
}

#[test]
fn astral_confrontation_costs_four_mana_with_an_active_attacker() {
    let mut engine = table(
        SEED,
        &[astral_confrontation()],
        &[plains(); 4],
        &[grizzly_bears()],
        &[llanowar_elves()],
    );
    attack_with_bear(&mut engine);
    let spell = in_hand(&engine, me(), astral_confrontation()).unwrap();
    assert_eq!(
        priority_offer(&engine).spell_reduction(spell, CastModeKind::Normal),
        1
    );
    let elf = on_battlefield(&engine, them(), llanowar_elves()).expect("their Elf");
    cast_at(&mut engine, astral_confrontation(), elf);
    assert!(exiled(&engine, them(), llanowar_elves()));
}

#[test]
fn astral_confrontation_loses_the_discount_after_the_last_attacker_dies() {
    let mut board = vec![plains(); 4];
    board.push(grizzly_bears());
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &board)
        .battlefield(1, &[mountain(), llanowar_elves()])
        .hand(0, &[astral_confrontation()])
        .hand(1, &[lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, me()));
    let bear = attack_with_bear(&mut engine);
    let spell = in_hand(&engine, me(), astral_confrontation()).unwrap();
    assert_eq!(
        priority_offer(&engine).spell_reduction(spell, CastModeKind::Normal),
        1
    );

    pass_until_priority(&mut engine, them());
    cast_from_hand(&mut engine, them(), lightning_bolt());
    aim_at(&mut engine, them(), bear);
    settle(&mut engine);
    assert!(in_graveyard(&engine, me(), grizzly_bears()).is_some());

    tap_all_mana(&mut engine, me());
    let offer = priority_offer(&engine);
    assert_eq!(offer.spell_reduction(spell, CastModeKind::Normal), 0);
    assert!(!offer.castable.contains(&spell));
}
