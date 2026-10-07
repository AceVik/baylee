//! `cards/enchantments/auras/mv_2/inertia_bubble.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Inertia Bubble prints two lines: "Enchant artifact" and "Enchanted
/// artifact doesn't untap during its controller's untap step." The board
/// holds two Sol Rings — the one the Bubble ends up holding and one it does
/// not — and both are tapped for mana in the same turn, so the untethered
/// Ring coming back in p0's next untap step is what tells a rule (CR 502.3)
/// from a game that simply never reached it. The filter is read off the
/// target menu as well: the Islands beside them are permanents and are not
/// offered, which is the mistake a `Filter::Any` would make. Nothing but
/// playing the Aura separates `Modifier::DoesNotUntap` on
/// `Filter::AttachedToBySource` from a static that lost its filter and
/// pinned every artifact in play.
#[test]
fn inertia_bubble_pins_the_artifact_it_enchants_while_the_ring_beside_it_untaps() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(4711, island())
        .battlefield(0, &[island(), island(), quiet_artifact(), quiet_artifact()])
        .hand(0, &[inertia_bubble()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // `cast_from_hand` is `tap_all_mana` plus the cast: both Rings are tapped
    // by the same helper that floats the {1}{U}, so the untap step below has
    // two tapped artifacts of the same printing to answer for.
    cast_from_hand(&mut engine, p0, inertia_bubble());

    let Pending::ChooseTargets {
        options, min, max, ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"enchant artifact\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!((min, max), (1, 1), "an Aura has exactly one host");
    assert_eq!(
        options.len(),
        2,
        "the two artifacts on the table and nothing else: {options:?}"
    );
    let island = on_battlefield(&engine, p0, island()).expect("the Island is out");
    assert!(
        !options.contains(&island),
        "\"artifact\" is read: a land is a permanent and not on this menu: {options:?}"
    );
    let (host, bystander) = (options[0], options[1]);

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![host],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| at_rest(e, p0));

    let bubble = on_battlefield(&engine, p0, inertia_bubble()).expect("the Bubble resolved");
    assert_eq!(
        engine.state().object(bubble).and_then(|o| o.attached_to),
        Some(host),
        "the Aura landed on the artifact that was named and not on the other one"
    );
    assert!(is_tapped(&engine, host), "the host paid for the cast");
    assert!(
        is_tapped(&engine, bystander),
        "and so did the Ring beside it, so the untap step has two answers to give"
    );

    // A whole turn cycle, so CR 502.3 is what happens between the two readings.
    reach_their_main_phase(&mut engine, PlayerId::new(1));
    assert!(walk_to_own_main(&mut engine, p0), "p0 takes another turn");
    assert!(
        !is_tapped(&engine, bystander),
        "the untap step ran: the Ring the Bubble does not hold came back"
    );
    assert!(
        is_tapped(&engine, host),
        "\"enchanted artifact doesn't untap\" — the step passed over the Ring \
         the Bubble holds and over no other"
    );
}
