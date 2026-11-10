//! `cards/lands/pain/battlefield_forge.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Battlefield Forge prints two mana abilities and nothing else: "{{T}}: Add
/// {{C}}." and "{{T}}: Add {{R}} or {{W}}. This land deals 1 damage to you."
/// One land cannot pay both taps in one turn, so two Forges stand side by
/// side and each half is the other's control: the colourless one has to hand
/// over exactly one colourless *and cost no life*, the coloured one exactly
/// one mana of the colour that was named — not the first option on offer —
/// and exactly one damage, to the seat that tapped and never to the seat
/// across the table.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn battlefield_forge_sells_colorless_for_free_and_color_for_one_life() {
    let p0 = PlayerId::new(0);
    let _p1 = PlayerId::new(1);
    let mut engine = Duel::new(881, plains())
        .battlefield(0, &[battlefield_forge(), battlefield_forge()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let forges = all_on_battlefield(&engine, p0, battlefield_forge());
    assert_eq!(forges.len(), 2, "two Forges, one tap each");
    let (colorless_side, colored_side) = (forges[0], forges[1]);

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    for (slot, forge) in forges.iter().enumerate() {
        for index in [0u32, 1] {
            assert!(
                legal.abilities.contains(&(*forge, index)),
                "the Forge at {slot} prints ability {index} and both are \
                 offered while it stands untapped: {:?}",
                legal.abilities
            );
        }
    }
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "nothing is floating, so whatever ends up in the pool came off these \
         two lands and nothing else"
    );

    // The half whose whole price is the tap.
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: colorless_side,
                ability_index: 0,
            },
        )
        .expect("{{T}}: Add {{C}} is the first thing the card prints");
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so nothing is waiting to resolve"
    );
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Colorless), 1, "one colourless");
    assert_eq!(
        pool.total(),
        1,
        "one mana off one tap, and no colour anywhere beside it"
    );
    assert_eq!(
        engine.state().players[0].life,
        20,
        "the colourless half prints no damage clause, so the life asserted \
         below can only have come from the other one"
    );

    // The half that charges for it. Targets are chosen when an ability is
    // announced (CR 601.2c) and the colour when it resolves, so whatever the
    // engine asks on the way is answered here — the card's only target is its
    // own controller, which is what "to you" means.
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: colored_side,
                ability_index: 1,
            },
        )
        .expect("{{T}}: Add {{R}} or {{W}} is the second thing it prints");
    let mut named_the_color = false;
    for _ in 0..8 {
        match engine.pending().clone() {
            Pending::ChooseTargets {
                player,
                options,
                player_options,
                ..
            } => {
                assert_eq!(player, p0, "the seat that activated answers");
                assert!(
                    options.is_empty(),
                    "the damage is aimed at a player and at no permanent: {options:?}"
                );
                assert!(
                    player_options.contains(&p0),
                    "\"to you\" is the ability's own controller: {player_options:?}"
                );
                engine
                    .apply(
                        p0,
                        PlayerAction::ChooseTargets {
                            objects: vec![],
                            players: vec![p0],
                        },
                    )
                    .expect("the controller was one of the options on offer");
            }
            Pending::ChooseColor { player, options } => {
                assert_eq!(player, p0, "the seat that taps names the colour");
                assert_eq!(
                    options.len(),
                    2,
                    "\"Add {{R}} or {{W}}\" is one mana of one of two colours: {options:?}"
                );
                assert!(
                    options.contains(&ManaColor::Red) && options.contains(&ManaColor::White),
                    "exactly the two it prints: {options:?}"
                );
                assert!(
                    !options.contains(&ManaColor::Colorless),
                    "{{C}} belongs to the other ability, and is no colour at all \
                     (CR 105.4): {options:?}"
                );
                named_the_color = true;
                // The second option, so that the pool read below says the
                // colour was read rather than defaulted to the first.
                engine
                    .apply(p0, PlayerAction::ChooseColor(ManaColor::White))
                    .expect("white was one of the colours it offered");
            }
            Pending::Priority { .. } => break,
            other => panic!("unexpected while the Forge resolves: {other:?}"),
        }
    }
    assert!(
        named_the_color,
        "the coloured half asks which of {{R}} or {{W}} it is making"
    );

    assert!(
        stack_is_empty(&engine),
        "and it too is one mana ability: nothing the card did ever used the stack"
    );
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::White),
        1,
        "one mana of the colour that was named"
    );
    assert_eq!(
        pool.available(ManaColor::Red),
        0,
        "\"or\" is one of the two and not both"
    );
    assert_eq!(
        pool.available(ManaColor::Colorless),
        1,
        "and the colourless half is still standing beside it"
    );
    assert_eq!(
        pool.total(),
        2,
        "two taps and two mana: neither ability handed over more than one"
    );
    assert_eq!(
        engine.state().players[0].life,
        19,
        "\"This land deals 1 damage to you\" — the price is paid by the seat \
         that took the coloured mana, and by it alone"
    );
    assert_eq!(
        engine.state().players[1].life,
        20,
        "and never across the table"
    );
    assert!(
        forges.iter().all(|id| is_tapped(&engine, *id)),
        "both taps were paid for what the pool is holding"
    );
}
