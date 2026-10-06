//! `cards/creatures/mv_1/dragon_s_rage_channeler.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Dragon's Rage Channeler` prints `Whenever you cast a noncreature spell, surveil 1.` and `Delirium — As long as there are four or more card types among cards in your graveyard, this creature gets +2/+2, has flying, and attacks each combat if able.`
///
/// Marked `Coverage::Partial`, casting a noncreature spell like `lightning_greaves()` triggers surveil 1 via `Trigger::SpellCast`.
/// The trigger prompts through `Pending::Arrange` with `ArrangePrompt::Surveil`, moving the top card into `ZoneLocation::Graveyard`, while the Delirium stat bonus and `KeywordSet::FLYING` are omitted.
#[test]
fn dragon_s_rage_channeler_surveils_on_noncreature_cast_and_omits_delirium() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[dragon_s_rage_channeler(), plains(), plains()])
        .hand(0, &[lightning_greaves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let drc = on_battlefield(&engine, p0, dragon_s_rage_channeler()).expect("drc seated");
    assert_eq!(pt(&engine, drc), (1, 1));
    assert!(!keywords(&engine, drc).contains(KeywordSet::FLYING));

    cast_from_hand(&mut engine, p0, lightning_greaves());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::Arrange { .. })
    });

    let Pending::Arrange {
        player,
        cards,
        prompt,
        piles,
    } = engine.pending().clone()
    else {
        panic!("expected a surveil arrangement, got {:?}", engine.pending());
    };
    assert_eq!(player, p0);
    assert_eq!(prompt, crate::choice::ArrangePrompt::Surveil);
    assert_eq!(piles, surveil_piles(1));
    assert!(!cards.is_empty());

    let binned = cards[0];
    engine.apply(p0, look_answer(&cards, &[binned])).unwrap();

    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p0, forest()).is_some(),
        "surveiled card was placed into the graveyard"
    );
    assert!(
        on_battlefield(&engine, p0, lightning_greaves()).is_some(),
        "lightning greaves resolved"
    );
    assert_eq!(
        pt(&engine, drc),
        (1, 1),
        "under `Coverage::Partial` delirium is omitted, body remains 1/1"
    );
    assert!(!keywords(&engine, drc).contains(KeywordSet::FLYING));
}
