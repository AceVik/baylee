//! `cards/lands/utility/takenuma_abandoned_mire.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Takenuma, Abandoned Mire is a legendary land with no enter modifier, and
/// the half of it that is written is `{T}: Add {B}`. The card prints no basic
/// land type, so this tap is a *printed* ability and `tap_all_mana` — the
/// CR 305.6 intrinsic list — never touches it, which is why it is pressed by
/// index instead. A Forest stands untapped beside it as the control: black
/// mana in the pool while the only other land on the board has not moved can
/// have come from nowhere else. The channel line is the `Coverage::Partial`
/// gap, so it is asserted as an absence — nothing discards the card, and it
/// is still on the battlefield in nobody's graveyard afterwards.
#[test]
fn takenuma_abandoned_mire_taps_for_black_and_offers_no_channel() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(97, forest())
        .battlefield(0, &[forest()])
        .hand(0, &[takenuma_abandoned_mire()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let forest_land = on_battlefield(&engine, p0, forest()).expect("the Forest is out");
    let land = play_land(&mut engine, p0, takenuma_abandoned_mire());
    assert!(
        !entered_tapped(&engine, land),
        "the card prints no enter modifier, so it lands untapped"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "playing a land makes no mana by itself"
    );

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!(
            "the land drop leaves the seat holding priority: {:?}",
            engine.pending()
        )
    };
    assert!(
        legal.abilities.contains(&(land, 0)),
        "the land's own {{T}}: Add {{B}} is offered: {:?}",
        legal.abilities
    );
    assert!(
        !legal.mana_abilities.contains(&land),
        "a printed tap is no basic land type (CR 305.6), so `tap_all_mana` \
         would never press it: {:?}",
        legal.mana_abilities
    );

    activate(&mut engine, p0, takenuma_abandoned_mire(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Black),
        1,
        "{{B}}, off the land's own tap"
    );
    assert_eq!(pool.total(), 1, "one mana and nothing else came with it");
    assert!(is_tapped(&engine, land), "the land paid its own {{T}}");
    assert!(
        !is_tapped(&engine, forest_land),
        "and the untapped Forest is the control: the black mana has no other \
         source on this board"
    );
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack"
    );
    assert!(
        on_battlefield(&engine, p0, takenuma_abandoned_mire()).is_some(),
        "the channel line is the gap, so nothing discarded the card for it"
    );
    assert!(
        in_graveyard(&engine, p0, takenuma_abandoned_mire()).is_none(),
        "and no three cards were milled into anybody's graveyard"
    );
}
