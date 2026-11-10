//! `cards/lands/deserts/survivors_encampment.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// The convoke lands: "{T}, Tap an untapped creature you control: Add one
/// mana of any color."
///
/// Three cards `landgen` reads out of the printed text now that the DSL can
/// say the cost, and they are played together because what is worth proving
/// is the *shape* — one rule wrote all three, so a test of one of them is a
/// test of the rule and a difference between them would be a difference in
/// the printing rather than in the code.
///
/// Two questions on one activation, and the order is the point: CR 601.2h
/// pays the cost while the ability is being activated, and "one mana of any
/// color" is chosen when it *resolves* (CR 605.3b, immediately, without the
/// stack). So the creature is named first and the colour second, and the
/// mana that arrives is the colour the second answer named.
///
/// The land's own `{T}` is what makes the pair worth reading twice: both of
/// its abilities print one, so paying either takes the other off the offer.
#[allow(clippy::too_many_lines)] // three cards, each activated end to end
#[test]
fn a_convoke_land_taps_a_creature_of_yours_for_a_colour_of_your_choosing() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let lands = [
        survivors_encampment(),
        holdout_settlement(),
        scene_of_the_crime(),
    ];
    let mut engine = Duel::new(83, forest())
        .battlefield(
            0,
            &[
                lands[0],
                lands[1],
                lands[2],
                llanowar_elves(),
                llanowar_elves(),
                llanowar_elves(),
            ],
        )
        // A creature across the table: "a creature you control" is not an
        // invitation to tap somebody else's, and no rule says so — the card
        // prints it and `cost_wizard::options` is what draws the line.
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elf");
    let colours = [ManaColor::Blue, ManaColor::Red, ManaColor::White];

    for (land_index, (card, colour)) in lands.iter().zip(colours).enumerate() {
        let land = on_battlefield(&engine, p0, *card).expect("the land is on the battlefield");
        let Pending::Priority { legal, .. } = engine.pending().clone() else {
            panic!("expected priority, got {:?}", engine.pending())
        };
        // Both of the land's abilities are printed mana abilities, and
        // `legal.mana_abilities` is the CR 305.6 shortcut — lands that make
        // mana off nothing but their own tap — so the one with a cost to ask
        // about is an ordinary pair in `legal.abilities`.
        assert!(
            legal.abilities.contains(&(land, 1)),
            "land {land_index}: the convoke line is offered while a creature \
             of mine stands untapped: {:?}",
            legal.abilities
        );

        engine
            .apply(
                p0,
                PlayerAction::ActivateAbility {
                    source: land,
                    ability_index: 1,
                },
            )
            .expect("the cost asks which creature instead of refusing");

        let Pending::ChooseCards {
            options,
            min,
            max,
            prompt,
            ..
        } = engine.pending().clone()
        else {
            panic!(
                "land {land_index}: the cost asks which creature to tap: {:?}",
                engine.pending()
            )
        };
        assert_eq!(
            prompt,
            ChoicePrompt::CostTap,
            "land {land_index}: a tap, and not a sacrifice"
        );
        assert_eq!((min, max), (1, 1), "land {land_index}: one creature");
        assert!(
            !options.contains(&theirs),
            "land {land_index}: a creature I do not control is not mine to \
             tap: {options:?}"
        );
        assert!(
            !options.contains(&land),
            "land {land_index}: and the land is no creature: {options:?}"
        );
        let victim = *options.first().expect("an untapped creature of my own");
        assert!(
            !is_tapped(&engine, victim),
            "land {land_index}: the menu holds only untapped creatures \
             (CR 118.3)"
        );

        let before = engine.state().players[0].mana_pool.available(colour);
        engine
            .apply(
                p0,
                PlayerAction::ChooseObjects {
                    objects: vec![victim],
                },
            )
            .expect("the Elf is one of the answers the engine listed");

        // CR 605.3b: a mana ability resolves immediately, and "any color"
        // is a choice made on the way.
        let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
            panic!(
                "land {land_index}: any colour is a choice: {:?}",
                engine.pending()
            )
        };
        assert_eq!(options.len(), 5, "land {land_index}: all five colours");
        engine
            .apply(p0, PlayerAction::ChooseColor(colour))
            .expect("a colour the engine offered");

        assert!(
            is_tapped(&engine, victim),
            "land {land_index}: the creature that paid is tapped"
        );
        assert!(
            engine.state().object(victim).is_some(),
            "land {land_index}: and still on the battlefield — a tap is not \
             a sacrifice"
        );
        assert!(
            is_tapped(&engine, land),
            "land {land_index}: the land spent its own {{T}} too"
        );
        assert!(
            stack_is_empty(&engine),
            "land {land_index}: a mana ability uses no stack (CR 605.3b)"
        );
        assert_eq!(
            engine.state().players[0].mana_pool.available(colour),
            before + 1,
            "land {land_index}: one mana of the colour that was named"
        );

        // The shared {T} is spent, so neither of the land's two lines is on
        // offer any more.
        let Pending::Priority { legal, .. } = engine.pending().clone() else {
            panic!("expected priority, got {:?}", engine.pending())
        };
        assert!(
            !legal.abilities.contains(&(land, 1)) && !legal.mana_abilities.contains(&land),
            "land {land_index}: both printed lines cost {{T}}, and it is \
             spent: {:?} / {:?}",
            legal.abilities,
            legal.mana_abilities
        );
    }
}
