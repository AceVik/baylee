//! `cards/instants/mv_2/false_summoning.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// False Summoning — {1}{U} instant: "Counter target creature spell."
///
/// The whole card is one target spec, and the two words a wider filter would
/// lose are "spell" and "creature": an Elf already standing on the battlefield
/// is the permanent that a bare `Filter::CREATURE` would have offered, so the
/// counter's menu has to name the Elf *card* on the stack and decline the Elf
/// permanent while it does. Playing it means playing both sides of a real
/// stack — p0 casts the Elf, p1 answers it — and the {1}{U} leaves the pool only
/// after the target is named (CR 601.2c, then CR 601.2h), which is why the
/// mana is read on both sides of that answer.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn false_summoning_counters_a_creature_spell_and_never_a_creature() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(409, forest())
        .battlefield(0, &[forest(), llanowar_elves()])
        .hand(0, &[llanowar_elves()])
        .battlefield(1, &[island(), island()])
        .hand(1, &[false_summoning()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // A real creature spell on the stack, with p1 to answer it: the walk stops
    // on p1's priority rather than letting the Elf resolve, because a resolved
    // Elf is no longer a spell and the card has nothing left to target.
    cast_from_hand(&mut engine, p0, llanowar_elves());
    pass_until(&mut engine, |e| {
        on_stack(e, llanowar_elves()).is_some()
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p1)
    });
    let spell = on_stack(&engine, llanowar_elves()).expect("the Elf is a spell on the stack");
    let permanent =
        on_battlefield(&engine, p0, llanowar_elves()).expect("an Elf is a permanent already");

    // Mana before the claim: `castable` is filtered through `can_afford`,
    // which reads the pool and not the untapped Islands.
    tap_all_mana(&mut engine, p1);
    let Pending::Priority { player, legal } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert_eq!(player, p1, "the answering seat holds priority");
    let card = in_hand(&engine, p1, false_summoning()).expect("the counter is in hand");
    assert!(
        legal.castable.contains(&card),
        "{{1}}{{U}} is floating, so the counter is castable: {:?}",
        legal.castable
    );
    assert_eq!(
        engine.state().players[1].mana_pool.total(),
        2,
        "two Islands, two blue"
    );

    engine
        .apply(p1, PlayerAction::CastSpell { card })
        .expect("the counter is cast");
    let Pending::ChooseTargets {
        player,
        options,
        player_options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"target creature spell\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p1, "the casting seat aims it");
    assert_eq!((min, max), (1, 1), "one target, no more and no fewer");
    assert!(
        player_options.is_empty(),
        "\"target creature spell\" counts objects and no players: {player_options:?}"
    );
    assert!(
        options.contains(&spell),
        "the creature spell on the stack is the target the card prints: {options:?}"
    );
    assert!(
        !options.contains(&permanent),
        "\"spell\" is not \"permanent\": the Elf already on the battlefield is a \
         creature and never a legal target: {options:?}"
    );
    assert_eq!(options.len(), 1, "and that spell is the whole menu");
    assert_eq!(
        engine.state().players[1].mana_pool.total(),
        2,
        "CR 601.2h pays last: the target is named first, so the {{1}}{{U}} is \
         still floating while the question stands"
    );

    engine
        .apply(
            p1,
            PlayerAction::ChooseObjects {
                objects: vec![spell],
            },
        )
        .expect("the spell the question offered is the one that was named");
    assert_eq!(
        engine.state().players[1].mana_pool.total(),
        0,
        "and the cost came out of the pool with the answer"
    );
    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p0, llanowar_elves()).is_some(),
        "\"counter\" sends the spell to its owner's graveyard: the countered Elf \
         is the card that was on the stack, and a spell that had resolved would \
         have left nothing there"
    );
    assert!(
        in_graveyard(&engine, p1, false_summoning()).is_some(),
        "and the counter itself resolved into its caster's graveyard"
    );
    assert_eq!(
        all_on_battlefield(&engine, p0, llanowar_elves()).len(),
        1,
        "the Elf already on the battlefield never became a target and is still \
         the one permanent it was"
    );
}
