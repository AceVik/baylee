//! `cards/instants/mv_2/razorgrass_ambush.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Razorgrass Ambush // Razorgrass Field — `{1}{W}` Instant // Land.
/// The instant face reads "Razorgrass Ambush deals 3 damage to target
/// attacking or blocking creature." The land face has a life-payment
/// enter trigger and `{T}: Add {W}`.
///
/// Both faces are a GENERATED STUB — no abilities are implemented. The
/// instant face declares no targeting, so the stub casts as a vanilla
/// spell and the test can only confirm that the card reaches the stack and
/// resolves into the graveyard. The 3-damage effect, the mandatory combat
/// target, and the land-face life-payment trigger are all absent; this
/// scenario proves nothing about those clauses.
///
/// SKIP: the instant effect — "deals 3 damage to target attacking or
/// blocking creature" — needs a creature in combat as a target, which
/// requires a full combat phase and a helper to read the damage counter.
/// The land-face enter trigger (pay 3 life or enter tapped) needs
/// Razorgrass Ambush // Razorgrass Field (`Coverage::Partial`): "Razorgrass
/// Ambush deals 3 damage to target attacking or blocking creature."
///
/// The blocking half is the partial — the DSL has no filter for a blocking
/// creature — so the implemented sentence is the attacking one, and reaching
/// it needs a real combat. That is the whole point of the test: the spell is
/// **not castable** with nothing attacking, which is how this card first
/// read as an unimplemented stub to a reader that only tried to cast it on
/// an empty board.
#[test]
fn razorgrass_ambush_burns_a_creature_that_is_attacking() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(301, plains())
        .battlefield(0, &[plains(), plains()])
        .hand(0, &[razorgrass_ambush()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // Nothing is attacking yet, so there is no legal target and the spell
    // cannot be cast at all — the engine refuses it rather than offering an
    // empty target list. The mana is tapped first on purpose: without it the
    // refusal would be affordability and this assertion would prove nothing.
    let spell = in_hand(&engine, p0, razorgrass_ambush()).expect("the Ambush is in hand");
    tap_all_mana(&mut engine, p0);
    assert!(
        engine.state().players[0].mana_pool.total() >= 2,
        "two Plains pay {{1}}{{W}}, so what is refused below is the target"
    );
    assert!(
        engine
            .apply(p0, PlayerAction::CastSpell { card: spell })
            .is_err(),
        "with no attacking creature the Ambush has nothing to target"
    );

    // A second game for the combat, because those two Plains are tapped now
    // and a seat's lands untap in its **own** untap step (CR 502.1) — on p1's
    // turn p0 would have had no mana, and "not castable" would have meant
    // something else entirely.
    let mut engine = Duel::new(302, plains())
        .battlefield(0, &[plains(), plains()])
        .hand(0, &[razorgrass_ambush()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // p1's turn, and the Elf swings.
    let elf = on_battlefield(&engine, p1, llanowar_elves()).expect("the Elf is deployed");
    pass_until(&mut engine, |e| {
        e.state().turn.active == p1 && matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers { attackers, .. } = engine.pending().clone() else {
        unreachable!("the walk waited for exactly this")
    };
    assert!(
        attackers.contains(&elf),
        "a creature that has been on the battlefield since before the game \
         may attack (CR 302.6): {attackers:?}"
    );
    engine
        .apply(
            p1,
            PlayerAction::DeclareAttackers {
                attackers: vec![(elf, Defender::Player(p0))],
            },
        )
        .expect("attacking the other seat is legal");

    // Now p0 has a target, and three damage on a 1/1 is lethal.
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p0),
    );
    cast_from_hand(&mut engine, p0, razorgrass_ambush());
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "the Ambush asks for an attacking creature, got {:?}",
            engine.pending()
        )
    };
    assert!(
        options.contains(&elf),
        "the attacking Elf is the legal target: {options:?}"
    );
    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![elf] })
        .expect("the attacking Elf is a legal target");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p1, llanowar_elves()).is_some(),
        "three damage on a 1/1 is lethal (CR 704.5g)"
    );
}
