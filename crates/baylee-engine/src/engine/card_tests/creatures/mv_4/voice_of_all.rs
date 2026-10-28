//! `cards/creatures/mv_4/voice_of_all.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

/// Voice of All — {2}{W}{W} Creature — Angel 2/2
/// "Flying
/// As this creature enters, choose a color.
/// This creature has protection from the chosen color."
#[test]
fn voice_of_all_enters_choosing_color_and_gains_protection_from_it() {
    let voice = card_index("b1631e9c-a993-4d44-b009-b2624d79560c");
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(
            0,
            &[
                plains(),
                plains(),
                plains(),
                plains(),
                mountain(),
                grizzly_bears(),
            ],
        )
        .hand(0, &[voice, index::LIGHTNING_BOLT])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, voice);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseColor { .. })
    });
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Red))
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    let voice_id = on_battlefield(&engine, p0, voice).expect("Voice of All on board");
    assert!(keywords(&engine, voice_id).contains(KeywordSet::FLYING));
    let bears_id = on_battlefield(&engine, p0, grizzly_bears()).expect("Grizzly Bears on board");

    // Red Lightning Bolt cannot target Voice of All (protection from red), only Grizzly Bears.
    cast_with_floating(&mut engine, p0, index::LIGHTNING_BOLT);
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected target choice for Lightning Bolt");
    };
    assert!(!options.contains(&voice_id));
    assert!(options.contains(&bears_id));
}
