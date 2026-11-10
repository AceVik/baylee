//! Printed behavior and adversarial resolution cases.
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

use baylee_core::color::{Color, ColorSet};
const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);
fn setup() -> (Engine<RegistryLookup>, ObjectId, ObjectId) {
    let mut e = Duel::new(SEED, forest())
        .hand(0, &[index::DREAM_COAT])
        .battlefield(1, &[index::GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    let host = on_battlefield(&e, P1, index::GRIZZLY_BEARS).unwrap();
    let aura = in_hand(&e, P0, index::DREAM_COAT).unwrap();
    e.dev_state_mut(P0).unwrap().players[0]
        .mana_pool
        .add(ManaColor::Blue, 1);
    e.refresh_offer();
    cast_with_floating(&mut e, P0, index::DREAM_COAT);
    aim_at(&mut e, P0, host);
    pass_until(&mut e, stack_is_empty);
    (e, aura, host)
}
fn choose(e: &mut Engine<RegistryLookup>, wanted: ColorSet) {
    pass_until(e, |e| matches!(e.pending(), Pending::ChooseCastMode { .. }));
    let Pending::ChooseCastMode { options, .. } = e.pending() else {
        unreachable!()
    };
    assert_eq!(options.len(), 31);
    let index = options
        .iter()
        .position(|o| o.kind == CastModeKind::Colors(wanted))
        .unwrap();
    let before = e.snapshot_hash();
    assert!(e.apply(P0, PlayerAction::ChooseMode(31)).is_err());
    assert_eq!(e.snapshot_hash(), before);
    e.apply(P0, PlayerAction::ChooseMode(index)).unwrap();
    pass_until(e, stack_is_empty);
}
#[test]
fn dream_coat_all_nonempty_color_combinations_replace_color_indefinitely() {
    for mask in 1_u8..32 {
        let colors = [
            Color::White,
            Color::Blue,
            Color::Black,
            Color::Red,
            Color::Green,
        ]
        .into_iter()
        .enumerate()
        .filter(|(bit, _)| mask & (1 << bit) != 0)
        .fold(ColorSet::EMPTY, |set, (_, color)| {
            set.union(ColorSet::of(color))
        });
        let (mut e, aura, host) = setup();
        activate(&mut e, P0, index::DREAM_COAT, 1);
        choose(&mut e, colors);
        assert_eq!(
            e.state().object(host).unwrap().characteristics().colors,
            colors
        );
        pass_until(&mut e, |e| at_rest(e, P0));
        assert!(!priority_offer(&e).abilities.contains(&(aura, 1)));
        reach_their_main_phase(&mut e, P1);
        assert_eq!(
            e.state().object(host).unwrap().characteristics().colors,
            colors
        );
        pass_until(
            &mut e,
            |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == P0),
        );
        assert!(
            priority_offer(&e).abilities.contains(&(aura, 1)),
            "once each turn, including the opponent's"
        );
    }
}
#[test]
fn dream_coat_departed_aura_uses_its_last_attachment() {
    let (mut e, aura, host) = setup();
    activate(&mut e, P0, index::DREAM_COAT, 1);
    bury(&mut e, &[aura]);
    choose(&mut e, ColorSet::of(Color::Black));
    assert_eq!(
        e.state().object(host).unwrap().characteristics().colors,
        ColorSet::of(Color::Black)
    );
}
#[test]
fn dream_coat_does_not_color_a_returned_host() {
    let (mut e, _, host) = setup();
    activate(&mut e, P0, index::DREAM_COAT, 1);
    let state = e.dev_state_mut(P0).unwrap();
    state
        .move_object(
            host,
            ZoneLocation::Exile(P1),
            ZonePosition::Top,
            Cause::Effect,
        )
        .unwrap();
    state
        .move_object(
            host,
            ZoneLocation::Battlefield,
            ZonePosition::Top,
            Cause::Effect,
        )
        .unwrap();
    pass_until(&mut e, stack_is_empty);
    assert_eq!(
        e.state().object(host).unwrap().characteristics().colors,
        ColorSet::of(Color::Green)
    );
}
