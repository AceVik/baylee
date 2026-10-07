//! `cards/creatures/mv_4/wartime_protestors.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Aang and Katara make X Allies at once; Wartime Protestors says
/// "whenever **another Ally** you control enters, put a +1/+1 counter on
/// that creature and it gains haste". Every one of them is an Ally
/// entering, so the rally fires once for each.
///
/// It fired **once for the whole batch**, because the trigger scan broke
/// out of the event loop after the first match. Six tokens arrived, one of
/// them was answered, and the other five were invisible to everything on
/// the board — which is how it was reported: "4 of the tokens disappeared
/// and only one of the two was handled correctly".
///
/// The counted assertion is the counter-test in both directions. One
/// counter on each token fails on the old code (five have none) and would
/// also fail if the loop were made to fire per *permanent* per event, which
/// is the shape that gives N² triggers for N tokens.
#[test]
fn a_rally_trigger_fires_once_for_every_ally_that_entered() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(37, forest())
        .battlefield(
            0,
            &[
                wartime_protestors(),
                forest(),
                plains(),
                island(),
                quiet_artifact(),
                quiet_artifact(),
                quiet_artifact(),
            ],
        )
        .hand(0, &[aang_and_katara()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    assert!(tokens_of(&engine, p0).is_empty(), "no tokens yet");

    // Tapping everything for mana is what sets X. The lands go through
    // `mana_abilities` and the Sol Rings through `abilities` — an intrinsic
    // land tap and a printed mana ability are two different offers — and
    // `tap_all_mana` takes both (#159), so the three artifacts are tapped by
    // the same call that taps the lands. The Protestors is a creature with
    // no mana ability and stays untapped, which is why X is three and not
    // four.
    tap_all_mana(&mut engine, p0);
    let spell = in_hand(&engine, p0, aang_and_katara()).expect("the spell is in hand");
    engine
        .apply(p0, PlayerAction::CastSpell { card: spell })
        .expect("six mana is on the table");
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::Priority { .. }) && tokens_of(e, p0).len() == 3
    });

    let tokens = tokens_of(&engine, p0);
    assert_eq!(tokens.len(), 3, "one Ally per tapped artifact");
    // Let the three rally triggers resolve.
    pass_until(&mut engine, |e| {
        e.state().zones.list(ZoneLocation::Stack).is_empty()
            && matches!(e.pending(), Pending::Priority { .. })
    });

    for (n, token) in tokens.iter().copied().enumerate() {
        let obj = engine
            .state()
            .object(token)
            .expect("the token is still here");
        assert_eq!(
            obj.counters.get(CounterKind::P1P1),
            1,
            "token {n} was answered exactly once"
        );
        assert!(
            keywords(&engine, token).contains(KeywordSet::HASTE),
            "token {n} gained haste"
        );
    }

    let protestors = on_battlefield(&engine, p0, wartime_protestors()).expect("still there");
    assert_eq!(
        engine
            .state()
            .object(protestors)
            .expect("the source is on the battlefield")
            .counters
            .get(CounterKind::P1P1),
        0,
        "the trigger says `another Ally`",
    );
}
