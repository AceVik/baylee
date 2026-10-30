//! A paid effect opens mana production before the new cast chooses X.
use crate::engine::testkit::{Duel, card_index, keep_mulligans, reach_main_phase};
use crate::{
    choice::{Pending, PlayerAction},
    zone::{Zone, ZoneLocation},
};
use baylee_core::{ids::PlayerId, mana::ManaColor};

#[test]
fn effect_precast_payment_does_not_misreport_stored_x_as_selected() {
    let owner = PlayerId::new(0);
    let other = PlayerId::new(1);
    let forest = card_index("b34bb2dc-c1af-4d77-b0b3-a0fb342a5fc6");
    let fireball = card_index("aa7714b0-2bfb-458a-8ebf-37ec2c53383e");
    let mut engine = Duel::new(601_272, forest).hand(0, &[fireball]).start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, owner);
    let source = engine.state.zones.list(ZoneLocation::Hand(owner))[0];
    let version = engine.state.object(source).unwrap().version;
    engine.state.object_mut(source).unwrap().x_value = 9;
    engine.state.players[0].mana_pool.add(ManaColor::Red, 1);
    engine.state.players[0]
        .mana_pool
        .add(ManaColor::Colorless, 2);
    assert!(engine.open_cast_payment(owner, source, version, false, false));
    assert_eq!(engine.announced_spell_context(owner), Some((source, None)));
    assert_eq!(engine.announced_spell_context(other), None);
    engine.apply(owner, PlayerAction::PassPriority).unwrap();
    assert!(matches!(engine.pending(), Pending::ChooseNumber { .. }));
    assert_eq!(engine.announced_spell_context(owner), Some((source, None)));
    engine.apply(owner, PlayerAction::ChooseNumber(1)).unwrap();
    assert!(matches!(engine.pending(), Pending::ChooseTargets { .. }));
    assert_eq!(
        engine.announced_spell_context(owner),
        Some((source, Some(1)))
    );
    engine
        .apply(
            owner,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![other],
            },
        )
        .unwrap();
    assert_eq!(engine.state.object(source).unwrap().zone, Zone::Stack);
    assert_eq!(engine.announced_spell_context(owner), None);
}
