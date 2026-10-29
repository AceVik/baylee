//! `cards/creatures/mv_3/glowrider.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

/// Glowrider (`aaf825b0-af0f-47a0-8b00-85f0614cef5d`) — {2}{W} Creature — Human Cleric 2/1:
/// "Noncreature spells cost {1} more to cast."
#[test]
fn glowrider_taxes_noncreature_spells_by_one() {
    let glowrider_card = card_index("aaf825b0-af0f-47a0-8b00-85f0614cef5d");
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);

    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(), plains(), plains(), mountain(), mountain()])
        .hand(0, &[glowrider_card, lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let plains_ids = all_on_battlefield(&engine, p0, plains());
    tap_mana_where(&mut engine, p0, |id| plains_ids.contains(&id));
    cast_with_floating(&mut engine, p0, glowrider_card);
    pass_until(&mut engine, stack_is_empty);

    let bolt_id = in_hand(&engine, p0, lightning_bolt()).unwrap();
    let mountains = all_on_battlefield(&engine, p0, mountain());
    tap_mana_where(&mut engine, p0, |id| id == mountains[0]);
    assert!(
        !priority_offer(&engine).castable.contains(&bolt_id),
        "Lightning Bolt costs {{1}}{{R}} under Glowrider, so {{R}} is not enough"
    );

    tap_mana_where(&mut engine, p0, |id| id == mountains[1]);
    assert!(priority_offer(&engine).castable.contains(&bolt_id));
    cast_with_floating(&mut engine, p0, lightning_bolt());
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().players[1].life, 17);
}
