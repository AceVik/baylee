//! `cards/creatures/mv_5/ward_sliver.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

/// Ward Sliver (`7c259e83-1d85-4267-a2e6-1a0fbacae3fe`) — {4}{W} Creature — Sliver 2/2:
/// "As this creature enters, choose a color.
/// All Slivers have protection from the chosen color."
#[test]
fn ward_sliver_grants_all_slivers_protection_from_chosen_color() {
    let ward_card = card_index("7c259e83-1d85-4267-a2e6-1a0fbacae3fe");
    let p0 = PlayerId::new(0);

    let mut engine = Duel::new(SEED, plains())
        .battlefield(
            0,
            &[
                plains(),
                plains(),
                plains(),
                plains(),
                plains(),
                mountain(),
                index::METALLIC_SLIVER,
                grizzly_bears(),
            ],
        )
        .hand(0, &[ward_card, lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let plains_ids = all_on_battlefield(&engine, p0, plains());
    tap_mana_where(&mut engine, p0, |id| plains_ids.contains(&id));
    cast_with_floating(&mut engine, p0, ward_card);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseColor { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::ChooseColor(baylee_core::mana::ManaColor::Red),
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    let ward = on_battlefield(&engine, p0, ward_card).unwrap();
    let metallic = on_battlefield(&engine, p0, index::METALLIC_SLIVER).unwrap();
    let bear = on_battlefield(&engine, p0, grizzly_bears()).unwrap();

    cast_from_hand(&mut engine, p0, lightning_bolt());
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected target choice for Lightning Bolt");
    };
    assert!(!options.contains(&ward));
    assert!(!options.contains(&metallic));
    assert!(options.contains(&bear));
}
