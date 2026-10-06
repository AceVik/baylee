//! `cards/creatures/mv_3/wild_aesthir.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Wild Aesthir prints flying and first strike on a printed 1/1, plus
/// `{W}{W}: This creature gets +2/+0 until end of turn. Activate only once
/// each turn.` Seven Plains pay `{2}{W}` for the cast and leave exactly the
/// `{W}{W}` the pump charges, so the 3/1 that follows is a real payment and not
/// a label. The Elf beside it reads the same before and after (the control for
/// `Filter::This`), the line is *not* offered a second time while two white is
/// still floating — the printed once-a-turn limit and not a price the seat
/// cannot pay — and one turn later the printed body is back with the line
/// offered again, which is `PerTurn(1)` and `UntilEndOfTurn` rather than
/// `Once`.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn wild_aesthir_pumps_only_itself_and_only_once_a_turn() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                plains(),
                plains(),
                plains(),
                plains(),
                plains(),
                plains(),
                plains(),
                llanowar_elves(),
            ],
        )
        .hand(0, &[wild_aesthir()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bystander = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elf is out");
    let bystander_pt = pt(&engine, bystander);

    // Seven Plains, and the Elf named as the thing kept back: it is the control
    // creature this test reads again at the end, and a source tapped for mana
    // is one whose body has already changed for a reason of its own.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        7,
        "seven Plains tapped and nothing else"
    );
    cast_with_floating(&mut engine, p0, wild_aesthir());
    pass_until(&mut engine, stack_is_empty);

    let bird = on_battlefield(&engine, p0, wild_aesthir()).expect("the Aesthir resolved");
    assert_eq!(pt(&engine, bird), (1, 1), "the body the card prints");
    let printed = keywords(&engine, bird);
    assert!(printed.contains(KeywordSet::FLYING), "printed flying");
    assert!(
        printed.contains(KeywordSet::FIRST_STRIKE),
        "printed first strike"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        4,
        "{{2}}{{W}} is spent, leaving the {{W}}{{W}} the pump charges"
    );

    // Ability 0 is the only ability the card prints; the index is taken out of
    // the offer rather than guessed at.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("the seat holds a quiet main phase: {:?}", engine.pending())
    };
    let (source, ability_index) = legal
        .abilities
        .iter()
        .copied()
        .find(|(src, _)| *src == bird)
        .expect("{W}{W}: +2/+0 is the only activated ability the card prints");
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source,
                ability_index,
            },
        )
        .expect("the two white already floating pay for it");
    assert!(!stack_is_empty(&engine), "the pump is no mana ability");
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        pt(&engine, bird),
        (3, 1),
        "+2/+0 on the creature the ability names"
    );
    assert_eq!(
        pt(&engine, bystander),
        bystander_pt,
        "and nothing at all on the creature beside it"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "the {{W}}{{W}} came out of the pool"
    );

    // The printed limit, read with the price still covered: two white is
    // exactly the cost, so an absent line is the once-a-turn rule and not an
    // ability the seat could not have paid for.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.abilities.iter().any(|(src, _)| *src == bird),
        "\"Activate only once each turn\" — {{W}}{{W}} is still in the pool, so \
         the cost is payable and the missing line is the limit: {:?}",
        legal.abilities
    );

    // A turn later the pump is gone and the line is back: `PerTurn(1)`, and
    // `Duration::UntilEndOfTurn` rather than a counter.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert_eq!(
        pt(&engine, bird),
        (1, 1),
        "\"until end of turn\": the printed body is back on the Aesthir's next turn"
    );
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.iter().any(|(src, _)| *src == bird),
        "the limit is per turn and not once for the game: {:?}",
        legal.abilities
    );
}
