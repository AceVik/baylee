//! `cards/creatures/mv_5/pinnacle_monk.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Pinnacle Monk ({3}{R}{R}, 2/2): "When this creature enters, return target
/// instant or sorcery card from your graveyard to your hand."
///
/// The graveyard is stocked so that the *filter* is what the test reads:
/// two Llanowar Elves are milled into it first and a Dark Ritual is cast on
/// top of them, so "target instant or sorcery card" has one legal answer out
/// of three cards lying in the same zone. Creature cards rather than lands,
/// because they are the control that rejects the two filters a card like this
/// drifts into — `Filter::Any`, which is what the neighbouring Eternal
/// Witness prints, *and* a nonland filter, which land bystanders would let
/// through.
///
/// The minimum is struck too: the printed line has no "may" in it, so the
/// choice is one-of-one rather than the up-to-one the Witness gets, and a
/// player cannot decline the buy-back.
#[test]
fn an_entering_pinnacle_monk_buys_back_the_instant_and_leaves_the_creature_cards_lying() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(23, llanowar_elves())
        .battlefield(
            0,
            &[
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                swamp(),
            ],
        )
        .hand(0, &[pinnacle_monk(), dark_ritual()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");
    // Two cards off an Elf deck: the controls that say the filter is read.
    seed_graveyard(&mut engine, p0, 2);

    // The Swamp alone pays for the Ritual, so the five Mountains are still
    // standing for the {3}{R}{R} the Monk costs.
    let swamp_land = on_battlefield(&engine, p0, swamp()).expect("a Swamp of its own");
    engine
        .apply(p0, PlayerAction::ActivateManaAbility { source: swamp_land })
        .expect("the Swamp taps for {B}");
    let ritual = in_hand(&engine, p0, dark_ritual()).expect("the Ritual is in hand");
    engine
        .apply(p0, PlayerAction::CastSpell { card: ritual })
        .expect("{B} is in the pool");
    pass_until(&mut engine, stack_is_empty);
    let buried = in_graveyard(&engine, p0, dark_ritual()).expect("the Ritual resolved and died");

    tap_all_mana(&mut engine, p0);
    let monk = in_hand(&engine, p0, pinnacle_monk()).expect("the Monk is in hand");
    engine
        .apply(p0, PlayerAction::CastSpell { card: monk })
        .expect("five Mountains pay {3}{R}{R}");
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });

    let Pending::ChooseTargets {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        unreachable!("the walk stopped at the trigger's target choice")
    };
    assert_eq!(player, p0, "its own controller points the trigger");
    assert_eq!(
        (min, max),
        (1, 1),
        "the printed line says \"return target instant or sorcery card\" with \
         no \"may\" anywhere in it, so the choice cannot be declined",
    );
    assert_eq!(
        options,
        vec![buried],
        "the Ritual alone: the two creature cards in the same graveyard are \
         neither instants nor sorceries",
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![buried],
            },
        )
        .expect("the one legal target");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p0, pinnacle_monk()).is_some(),
        "the Monk finished entering, so this was its own enters-trigger",
    );
    assert!(
        in_hand(&engine, p0, dark_ritual()).is_some(),
        "\"to your hand\" — the Ritual is castable again",
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Graveyard(p0)).len(),
        2,
        "the Ritual left; the two creature cards stayed where they lay",
    );
}

/// Prowess: "Whenever you cast a noncreature spell, this creature gets
/// +1/+1 **until end of turn**."
///
/// The Monk is seated rather than cast, which is the point of a second test
/// rather than a longer first one: prowess is about the spell cast *after*
/// it is already standing, and seeding the board skips the enters-trigger
/// the test above is entirely about.
///
/// Both halves of the printed sentence are struck. Dark Ritual is an instant
/// and therefore a noncreature spell, so the 2/2 is a 3/3 once the trigger
/// has resolved — and the turn is then handed over, where a pump written as
/// a permanent effect rather than an until-end-of-turn one (CR 702.108)
/// would leave a 3/3 standing.
#[test]
fn a_noncreature_spell_grows_the_monk_and_the_next_turn_takes_it_back() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(31, forest())
        .battlefield(0, &[pinnacle_monk(), swamp()])
        .hand(0, &[dark_ritual()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");
    let monk = on_battlefield(&engine, p0, pinnacle_monk()).expect("the Monk is standing");
    assert_eq!(
        pt(&engine, monk),
        (2, 2),
        "the printed 2/2, before anything has been cast",
    );

    let swamp_land = on_battlefield(&engine, p0, swamp()).expect("a Swamp of its own");
    engine
        .apply(p0, PlayerAction::ActivateManaAbility { source: swamp_land })
        .expect("the Swamp taps for {B}");
    let ritual = in_hand(&engine, p0, dark_ritual()).expect("the Ritual is in hand");
    engine
        .apply(p0, PlayerAction::CastSpell { card: ritual })
        .expect("{B} is in the pool");
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        pt(&engine, monk),
        (3, 3),
        "Dark Ritual is an instant, which is a noncreature spell, and prowess \
         answered it with +1/+1",
    );

    reach_their_main_phase(&mut engine, p1);
    assert_eq!(
        pt(&engine, monk),
        (2, 2),
        "\"until end of turn\" — the turn the spell was cast in is over and \
         the Monk is the 2/2 it prints again",
    );
}
