//! `cards/lands/check/ba_sing_se.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Ba Sing Se: "This land enters tapped unless you control a basic land." / "{T}: Add {G}." / "{2}{G}, {T}: Earthbend 2. Activate only as a sorcery."
/// Controlling a basic land allows Ba Sing Se to enter untapped.
/// Activating ability 1 earthbends itself: a 2/2 land creature with haste,
/// two `+1/+1` counters on it. A Lightning Bolt kills it, and earthbend's
/// delayed trigger returns it to the battlefield tapped, a land again.
#[test]
fn ba_sing_se_enters_untapped_and_animates_land() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(206, forest())
        .battlefield(0, &[forest(), forest(), forest()])
        .hand(0, &[ba_sing_se()])
        .battlefield(1, &[mountain()])
        .hand(1, &[lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, ba_sing_se());
    assert!(!entered_tapped(&engine, land));

    // The Forests pay; Ba Sing Se stays up, because `tap_all_mana` takes its
    // printed `{T}: Add` too (#159) and the animation is its other ability.
    tap_mana_except(&mut engine, p0, land);
    activate(&mut engine, p0, ba_sing_se(), 1);

    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected target choice, got {:?}", engine.pending());
    };
    assert!(options.contains(&land));

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![land],
            },
        )
        .unwrap();

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, land), (2, 2));
    assert!(keywords(&engine, land).contains(KeywordSet::HASTE));
    assert!(is_tapped(&engine, land));
    assert!(
        engine
            .state()
            .object(land)
            .unwrap()
            .characteristics()
            .types
            .contains(TypeSet::CREATURE)
    );
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p1),
    );
    tap_all_mana(&mut engine, p1);
    let bolt = in_hand(&engine, p1, lightning_bolt()).expect("the Bolt");
    engine
        .apply(p1, PlayerAction::CastSpell { card: bolt })
        .expect("a Mountain pays for the Bolt");
    aim_at(&mut engine, p1, land);
    pass_until(&mut engine, |e| {
        stack_is_empty(e)
            && e.journal().entries().iter().any(|entry| {
                matches!(entry.event, crate::event::GameEvent::ZoneChanged {
                    object, from: crate::zone::Zone::Graveyard, to: crate::zone::Zone::Battlefield, ..
                } if object == land)
            })
    });
    let back = engine.state().object(land).expect("Ba Sing Se");
    assert_eq!(back.zone, crate::zone::Zone::Battlefield, "it came back");
    assert!(is_tapped(&engine, land), "tapped");
    assert!(
        !back.characteristics().types.contains(TypeSet::CREATURE),
        "a land and no creature: a new object (CR 400.7)"
    );
    assert_eq!(counters_on(&engine, land, CounterKind::P1P1), 0);
}
