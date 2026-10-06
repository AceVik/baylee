//! `cards/creatures/mv_1/mogg_raider.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Mogg Raider — {R} 1/1 Goblin: "Sacrifice a Goblin: Target creature gets
/// +1/+1 until end of turn."
///
/// Both halves of the cost are read off what the engine is willing to offer.
/// The target question is `Filter::CREATURE`, so it names the creatures on
/// either side of the table and declines the Forest; the sacrifice question
/// is a Goblin **you control**, so the opponent's Raider — same printing,
/// same type line, one seat over — is a legal target and an illegal price in
/// the same activation. One of my two Goblins is then eaten, the +1/+1 lands
/// on the targeted Elf alone, and it is gone by my next main phase.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn mogg_raider_eats_a_goblin_to_pump_the_creature_it_targets() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(147, forest())
        .battlefield(
            0,
            &[mogg_raider(), mogg_raider(), llanowar_elves(), forest()],
        )
        .battlefield(1, &[mogg_raider(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let raiders = all_on_battlefield(&engine, p0, mogg_raider());
    assert_eq!(raiders.len(), 2, "two of mine, one of which is the price");
    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elves are out");
    let land = on_battlefield(&engine, p0, forest()).expect("my Forest is out");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elves are out");
    let their_raider = on_battlefield(&engine, p1, mogg_raider()).expect("their Raider is out");
    assert_eq!(pt(&engine, elf), (1, 1), "a printed 1/1 before the pump");

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    let (source, ability_index) = legal
        .abilities
        .iter()
        .copied()
        .find(|(id, _)| raiders.contains(id))
        .expect("the Raider's only line is offered: a Goblin stands beside it");
    let fodder = *raiders
        .iter()
        .find(|id| **id != source)
        .expect("the other Goblin is the price this activation names");
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source,
                ability_index,
            },
        )
        .expect("an untapped Raider with a Goblin on the table activates");

    // CR 601.2c chooses the target and CR 601.2h pays the cost, so the mark is
    // still on nobody while the sacrifice question is open — the two questions
    // are answered in the order they arrive rather than the order they are
    // expected in.
    let mut offer: Vec<ObjectId> = Vec::new();
    let mut menu: Vec<ObjectId> = Vec::new();
    for _ in 0..12 {
        if !offer.is_empty() && !menu.is_empty() {
            break;
        }
        match engine.pending().clone() {
            Pending::ChooseTargets {
                player, options, ..
            } => {
                assert_eq!(player, p0, "the activating seat aims the pump");
                offer = options;
                engine
                    .apply(player, PlayerAction::ChooseObjects { objects: vec![elf] })
                    .unwrap();
            }
            Pending::ChooseCards {
                player,
                options,
                min,
                max,
                prompt,
                ..
            } => {
                assert_eq!(player, p0, "the activating seat pays the cost");
                assert_eq!(
                    prompt,
                    crate::choice::ChoicePrompt::CostSacrifice,
                    "a cost and not a search, which is all a client has to tell \
                     the two apart"
                );
                assert_eq!((min, max), (1, 1), "one Goblin, and the cost asks once");
                menu = options;
                engine
                    .apply(
                        player,
                        PlayerAction::ChooseObjects {
                            objects: vec![fodder],
                        },
                    )
                    .unwrap();
            }
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            other => panic!("unexpected while the Raider's activation resolves: {other:?}"),
        }
    }
    pass_until(&mut engine, stack_is_empty);

    assert!(
        offer.contains(&elf) && offer.contains(&theirs),
        "\"target creature\" reaches either side of the table: {offer:?}"
    );
    assert!(
        offer.contains(&their_raider) && offer.contains(&source),
        "and a Goblin is a creature like any other: {offer:?}"
    );
    assert!(
        !offer.contains(&land),
        "the Forest is no creature: {offer:?}"
    );
    assert_eq!(
        menu.len(),
        2,
        "the two Goblins this seat controls are the whole menu: {menu:?}"
    );
    assert!(
        menu.contains(&source) && menu.contains(&fodder),
        "both of them, the source among them: {menu:?}"
    );
    assert!(!menu.contains(&elf), "the Elves are no Goblins: {menu:?}");
    assert!(
        !menu.contains(&their_raider),
        "`Filter::ControlledByYou` (CR 701.21a): the opponent's Goblin is a \
         legal target and no more sacrificeable than its Elf: {menu:?}"
    );

    assert!(
        in_graveyard(&engine, p0, mogg_raider()).is_some(),
        "the Goblin the menu offered went to its owner's graveyard"
    );
    assert!(
        !engine
            .state()
            .zones
            .list(ZoneLocation::Battlefield)
            .contains(&fodder),
        "and it is no longer on the battlefield"
    );
    assert_eq!(
        all_on_battlefield(&engine, p0, mogg_raider()).len(),
        1,
        "one of mine paid the price and the other still stands"
    );
    assert!(
        on_battlefield(&engine, p1, mogg_raider()).is_some(),
        "while the Raider across the table never moved"
    );
    assert_eq!(
        pt(&engine, elf),
        (2, 2),
        "+1/+1 until end of turn, on the creature the ability targeted"
    );
    assert_eq!(
        pt(&engine, theirs),
        (1, 1),
        "and nothing at all for the Elf it did not target"
    );

    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert_eq!(
        pt(&engine, elf),
        (1, 1),
        "\"until end of turn\": the pump is gone by this seat's next main phase"
    );
}
