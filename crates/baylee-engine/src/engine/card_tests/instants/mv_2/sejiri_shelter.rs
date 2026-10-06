//! `cards/instants/mv_2/sejiri_shelter.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Sejiri Shelter // Sejiri Glacier — `{1}{W}` Instant // Land.
/// The instant face reads "Target creature you control gains protection
/// from the color of your choice until end of turn." The land face enters
/// tapped and taps for `{W}`.
///
/// The card is `Coverage::Partial`: the land face taps for `{W}` and the
/// instant face has no ability at all, because protection from a colour of
/// your choice is not something the DSL can say. So the instant face casts
/// as a vanilla spell, and this test confirms only that the card reaches
/// the stack and resolves into the graveyard. The protection grant and the
/// `ChooseColor` question that must follow the target choice are both
/// absent; this scenario proves nothing about those clauses.
///
/// SKIP: the instant effect — "target creature you control gains
/// protection from the color of your choice" — requires a `ChooseTargets`
/// step for the creature and a `ChooseColor` step for the protection
/// color. Both depend on the targeting and protection-grant ability being
/// present in the card definition.
#[test]
fn sejiri_shelter_stub_casts_and_resolves_into_graveyard() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(302, plains())
        .battlefield(0, &[plains(), plains()])
        .hand(0, &[sejiri_shelter()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // The stub declares no target, so cast_from_hand succeeds. This only
    // confirms the card is registered and the engine can advance past it.
    // The real test must be written once the protection-grant ability and
    // the color-choice prompt are implemented.
    cast_from_hand(&mut engine, p0, sejiri_shelter());
    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p0, sejiri_shelter()).is_some(),
        "a resolved instant goes to its owner's graveyard"
    );
}

/// Sejiri Glacier, which is the half of Sejiri Shelter that is implemented.
///
/// The card's own test above can only watch the instant face resolve into a
/// graveyard, because "target creature you control gains protection from the
/// colour of your choice" is not something the DSL can say and the front
/// face carries no ability at all. That leaves the **land** face carrying
/// everything this card actually does — enters tapped, taps for `{W}` — and
/// nothing was playing it. A `Coverage::Partial` card is exactly where that
/// happens: the refusal is written down, so the half that works stops being
/// looked at.
#[test]
fn sejiri_glacier_enters_tapped_and_makes_white() {
    let seat = PlayerId::new(0);
    let (mut engine, land) =
        play_land_face(sejiri_shelter(), 1).expect("the back face is a land and may be played");

    assert!(
        is_tapped(&engine, land),
        "Sejiri Glacier prints \"this land enters tapped\""
    );

    // A land untaps in its controller's own untap step (CR 502.1), which is
    // the next turn but one — and then it makes white.
    pass_until(&mut engine, |e| {
        e.state().turn.number >= 3 && e.state().turn.active == seat && !is_tapped(e, land)
    });
    engine
        .apply(
            seat,
            PlayerAction::ActivateAbility {
                source: land,
                ability_index: 0,
            },
        )
        .expect("a printed mana ability is offered on the land it is printed on");
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::White),
        1,
        "`{{T}}: Add {{W}}` puts one white mana in the pool"
    );
    assert!(is_tapped(&engine, land), "and the land is tapped for it");
}
