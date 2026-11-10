//! `cards/creatures/mv_1/scryb_sprites.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Scryb Sprites is a {G} 1/1 Faerie and "Flying" is the whole of its printed
/// text, so a test that only read the keyword off the permanent would prove
/// nothing about what the card does. The scenario therefore plays the keyword
/// in combat: a turn later the Sprites attacks beside a ground 1/1, and the
/// opponent's own untapped Elf — the control that nothing on this board grants
/// flying to anything else — is offered as a blocker for the ground attacker
/// and refused for the flier. Exactly one point of damage gets through, and it
/// is the flier's.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn scryb_sprites_flies_over_the_ground_and_lands_its_point_of_damage() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), llanowar_elves()])
        .hand(0, &[scryb_sprites()])
        .battlefield(1, &[llanowar_elves()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let flier_card = scryb_sprites();
    let my_elf = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elf is out");
    let their_elf = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elf is out");
    assert!(
        !keywords(&engine, their_elf).contains(KeywordSet::FLYING),
        "the Elf across the table is the control: nothing here grants flying \
         to anything but the card under test"
    );

    // {G} off the one Forest, with the Elf named as the source kept back: it is
    // an attacker in the combat below, and a creature tapped for mana is one
    // the attack step will no longer offer.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "one Forest tapped, and the Elf did not pay for the spell"
    );
    cast_with_floating(&mut engine, p0, flier_card);
    pass_until(&mut engine, stack_is_empty);
    let sprites = on_battlefield(&engine, p0, flier_card).expect("the Sprites resolved");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "{{G}} came out of the pool the Forest filled"
    );
    assert_eq!(pt(&engine, sprites), (1, 1), "the body the card prints");
    assert!(
        types(&engine, sprites).contains(TypeSet::CREATURE),
        "and it arrived as a creature"
    );
    assert!(
        keywords(&engine, sprites).contains(KeywordSet::FLYING),
        "with the one keyword the card prints"
    );

    // A turn around, because a creature that entered this turn may not attack
    // (CR 302.6): a flier the combat step never offered would say nothing at
    // all about the keyword.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(
        !is_tapped(&engine, sprites) && !is_tapped(&engine, my_elf),
        "both of my creatures untapped with me"
    );
    assert!(
        !is_tapped(&engine, their_elf),
        "and the ground Elf across the table is untapped too, so the block it \
         is refused is refused for flying and not for its own tap"
    );

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers { attackers, .. } = engine.pending().clone() else {
        unreachable!("pass_until stops on nothing but the attack declaration")
    };
    assert!(
        attackers.contains(&sprites) && attackers.contains(&my_elf),
        "both are legal attackers now that neither is sick: {attackers:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![
                    (sprites, Defender::Player(p1)),
                    (my_elf, Defender::Player(p1)),
                ],
            },
        )
        .unwrap();

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    let Pending::ChooseBlockers { blockers, .. } = engine.pending().clone() else {
        unreachable!("pass_until stops on nothing but the block declaration")
    };
    let ground = blockers
        .iter()
        .find(|option| option.blocker == their_elf)
        .expect("the Elf can block the ground attacker, so it is on the list");
    assert!(
        ground.attackers.contains(&my_elf),
        "and the ground attacker is exactly what it may block, so the refusal \
         below is a refusal and not an empty list: {blockers:?}"
    );
    assert!(
        !ground.attackers.contains(&sprites),
        "the same Elf may not block the flier — flying is the whole of what \
         the card prints and this is the only place it shows: {blockers:?}"
    );

    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(their_elf, my_elf)],
            },
        )
        .expect("the pairing the offer named is a legal answer");
    // Not `stack_is_empty`: the stack is empty the moment blockers are
    // declared, so that predicate would stop before the combat damage step.
    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::Ending)
    });

    assert_eq!(
        engine.state().players[1].life,
        19,
        "the ground attacker was answered and the flier was not: one point of \
         damage, and it is the flying creature's"
    );
    assert!(
        in_graveyard(&engine, p1, llanowar_elves()).is_some()
            && in_graveyard(&engine, p0, llanowar_elves()).is_some(),
        "the two ground 1/1s killed each other, which is what says the declared \
         block really happened"
    );
    assert_eq!(
        engine.state().players[0].life,
        20,
        "and nothing came back at me"
    );
}
