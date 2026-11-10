//! Printed behavior and adversarial resolution cases.
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

use baylee_core::color::{Color, ColorSet};
#[test]
fn alchors_tomb_offers_one_color_at_resolution_and_the_change_survives_source_removal() {
    let p0 = PlayerId::new(0);
    for color in [
        Color::White,
        Color::Blue,
        Color::Black,
        Color::Red,
        Color::Green,
    ] {
        let mut e = Duel::new(SEED, forest())
            .battlefield(0, &[index::ALCHOR_S_TOMB, forest()])
            .start();
        keep_mulligans(&mut e);
        reach_main_phase(&mut e, p0);
        let target = on_battlefield(&e, p0, forest()).unwrap();
        let tomb = on_battlefield(&e, p0, index::ALCHOR_S_TOMB).unwrap();
        e.dev_state_mut(p0).unwrap().players[0]
            .mana_pool
            .add(ManaColor::Colorless, 2);
        e.refresh_offer();
        activate(&mut e, p0, index::ALCHOR_S_TOMB, 0);
        aim_at(&mut e, p0, target);
        assert!(
            e.state()
                .object(tomb)
                .unwrap()
                .status
                .contains(Status::TAPPED)
        );
        bury(&mut e, &[tomb]);
        pass_until(&mut e, |e| {
            matches!(e.pending(), Pending::ChooseCastMode { .. })
        });
        let Pending::ChooseCastMode { options, .. } = e.pending() else {
            unreachable!()
        };
        assert_eq!(options.len(), 5);
        let choice = options
            .iter()
            .position(|o| o.kind == CastModeKind::Colors(ColorSet::of(color)))
            .unwrap();
        e.apply(p0, PlayerAction::ChooseMode(choice)).unwrap();
        pass_until(&mut e, stack_is_empty);
        reach_their_main_phase(&mut e, PlayerId::new(1));
        assert_eq!(
            e.state().object(target).unwrap().characteristics().colors,
            ColorSet::of(color)
        );
        assert!(
            e.state()
                .object(target)
                .unwrap()
                .characteristics()
                .types
                .contains(TypeSet::LAND)
        );
    }
}
#[test]
fn alchors_tomb_target_leaving_prevents_the_color_choice() {
    let p0 = PlayerId::new(0);
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[index::ALCHOR_S_TOMB, forest()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let target = on_battlefield(&e, p0, forest()).unwrap();
    e.dev_state_mut(p0).unwrap().players[0]
        .mana_pool
        .add(ManaColor::Colorless, 2);
    e.refresh_offer();
    activate(&mut e, p0, index::ALCHOR_S_TOMB, 0);
    aim_at(&mut e, p0, target);
    bury(&mut e, &[target]);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(
        e.state().object(target).unwrap().characteristics().colors,
        baylee_core::color::ColorSet::EMPTY
    );
}
