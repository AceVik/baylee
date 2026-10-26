//! `cards/enchantments/auras/mv_1/private_research.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn private_research() -> CardIndex {
    card_index("e8578152-ee97-4471-bf32-77abe5ab80f1")
}

/// Private Research (e8578152-ee97-4471-bf32-77abe5ab80f1) — {U} Enchantment — Aura:
/// "Enchant creature"
/// "At the beginning of your upkeep, you may put a page counter on this Aura."
/// "When enchanted creature dies, draw a card for each page counter on this Aura."
#[test]
fn private_research_accumulates_page_counters_and_draws_when_enchanted_creature_dies() {
    use baylee_cards_dsl::counters;

    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, island())
        .battlefield(
            0,
            &[
                island(),
                mountain(),
                mountain(),
                grizzly_bears(),
                llanowar_elves(),
            ],
        )
        .hand(0, &[private_research(), lightning_bolt(), lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bears = on_battlefield(&engine, p0, grizzly_bears()).unwrap();
    let elf = on_battlefield(&engine, p0, llanowar_elves()).unwrap();
    let land = on_battlefield(&engine, p0, island()).unwrap();

    // Cast Private Research: targets only a creature, not a land.
    cast_from_hand(&mut engine, p0, private_research());
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected creature targets")
    };
    assert!(options.contains(&bears));
    assert!(
        !options.contains(&land),
        "non-creature is not a legal target"
    );
    aim_at(&mut engine, p0, bears);
    pass_until(&mut engine, stack_is_empty);

    let aura = on_battlefield(&engine, p0, private_research()).unwrap();
    assert_eq!(counters_on(&engine, aura, counters::PAGE), 0);

    // Next P0 upkeep: accept MayDo -> 1 page counter.
    reach_their_main_phase(&mut engine, p1);
    pass_until(&mut engine, |e| {
        e.state().turn.active == p0
            && matches!(
                e.pending(),
                Pending::YesNo {
                    prompt: crate::choice::YesNoPrompt::MayDo,
                    ..
                }
            )
    });
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    reach_main_phase(&mut engine, p0);
    assert_eq!(counters_on(&engine, aura, counters::PAGE), 1);

    // Put a second page counter on Private Research so we verify multiple-card draw.
    crate::replacement::put_counters(engine.dev_state_mut(p0).unwrap(), aura, counters::PAGE, 1);
    assert_eq!(counters_on(&engine, aura, counters::PAGE), 2);

    // Negative boundary: killing an unenchanted creature (Llanowar Elves) does not trigger Private Research.
    cast_from_hand(&mut engine, p0, lightning_bolt());
    aim_at(&mut engine, p0, elf);
    let hand_after_first_bolt = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_after_first_bolt,
        "unenchanted creature dying does not trigger Private Research"
    );

    // Killing the enchanted creature (Grizzly Bears) triggers Private Research,
    // which reads its 2 LKI page counters even after SBA puts the Aura into the graveyard.
    cast_from_hand(&mut engine, p0, lightning_bolt());
    aim_at(&mut engine, p0, bears);
    let hand_after_second_bolt = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().object(bears).unwrap().zone, Zone::Graveyard);
    assert_eq!(engine.state().object(aura).unwrap().zone, Zone::Graveyard);
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_after_second_bolt + 2,
        "draws 1 card for each of the 2 page counters on Private Research"
    );
}
