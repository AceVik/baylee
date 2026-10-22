//! `cards/instants/mv_2/honorable_passage.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn honorable_passage() -> CardIndex {
    card_index("66805334-1015-4b41-ba9b-116de94b0744")
}

/// `Honorable Passage` ({1}{W}, Instant, `Coverage::Partial`) prevents the next
/// damage a chosen source would deal to you this turn.
#[test]
fn honorable_passage_prevents_next_damage_from_chosen_source_to_you() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(), plains()])
        .battlefield(1, &[grizzly_bears()])
        .hand(0, &[honorable_passage()])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, p1);

    let bear = on_battlefield(&engine, p1, grizzly_bears()).unwrap();
    engine.apply(p1, PlayerAction::PassPriority).unwrap();

    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, honorable_passage());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseDamageSource { .. })
    });
    let Pending::ChooseDamageSource {
        choice, options, ..
    } = engine.pending().clone()
    else {
        unreachable!();
    };
    let source = options
        .into_iter()
        .find(|s| s.object == bear)
        .expect("Grizzly Bears is a damage source option");
    engine
        .apply(p0, PlayerAction::ChooseDamageSource { choice, source })
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });

    // Let Grizzly Bears attack P0 unblocked; the prevention shield absorbs the 2 combat damage.
    engine
        .apply(
            p1,
            PlayerAction::DeclareAttackers {
                attackers: vec![(bear, Defender::Player(p0))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| e.state().turn.phase == Phase::SecondMain);
    assert_eq!(engine.state().players[0].life, 20);
}
