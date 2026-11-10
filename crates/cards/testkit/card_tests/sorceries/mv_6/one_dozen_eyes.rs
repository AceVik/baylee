//! One Dozen Eyes, both individual modes and entwine.

use super::*;
use baylee_core::generated::subtypes;

fn one_dozen_eyes() -> CardIndex {
    card_index("b1fbf818-6699-4f05-9a91-19aa296526bf")
}

fn count(e: &Engine<RegistryLookup>, subtype: baylee_core::ids::SubtypeId) -> usize {
    e.state
        .battlefield_seen()
        .filter(|id| {
            e.state.object(*id).is_some_and(|object| {
                object.controller == PlayerId::new(0)
                    && object.characteristics().subtypes.contains(subtype)
            })
        })
        .count()
}

fn cast_with_modes(mask: u8) -> Engine<RegistryLookup> {
    let p0 = PlayerId::new(0);
    let lands = if mask == 3 { 9 } else { 6 };
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &vec![forest(); lands])
        .hand(0, &[one_dozen_eyes()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    cast_from_hand(&mut e, p0, one_dozen_eyes());
    let Pending::ChooseCastMode { options, .. } = e.pending().clone() else {
        panic!("expected modes")
    };
    let requested = match mask {
        1 => CastModeKind::Mode(0),
        2 => CastModeKind::Mode(1),
        3 => CastModeKind::Modes(3),
        _ => unreachable!("the test only casts the two modes or entwine"),
    };
    let mode = options
        .iter()
        .position(|option| option.kind == requested)
        .expect("the requested mode is offered");
    e.apply(p0, PlayerAction::ChooseMode(mode)).unwrap();
    pass_until(&mut e, stack_is_empty);
    e
}

#[test]
fn one_dozen_eyes_creates_each_mode_and_both_when_entwined() {
    let e = cast_with_modes(1);
    assert_eq!(count(&e, subtypes::creature::BEAST), 1);
    assert_eq!(count(&e, subtypes::creature::INSECT), 0);
    let e = cast_with_modes(2);
    assert_eq!(count(&e, subtypes::creature::BEAST), 0);
    assert_eq!(count(&e, subtypes::creature::INSECT), 5);
    let e = cast_with_modes(3);
    assert_eq!(count(&e, subtypes::creature::BEAST), 1);
    assert_eq!(count(&e, subtypes::creature::INSECT), 5);
}
