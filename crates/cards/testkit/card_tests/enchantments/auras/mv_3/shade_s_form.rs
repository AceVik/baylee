//! `cards/enchantments/auras/mv_3/shade_s_form.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

fn shade_s_form() -> CardIndex {
    card_index("682a6d77-e8cd-4bf2-b777-7793101e0fd3")
}

/// Enchanted creature gets "{B}: +1/+1 until end of turn" for its controller,
/// and returns under Shade's Form's controller's control if it dies (not if exiled).
#[test]
fn shade_s_form_grants_pump_and_reanimates_under_your_control_on_death() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = lk::game(
        SEED,
        &[],
        &[grizzly_bears()],
        &[shade_s_form(), lightning_bolt()],
        &[],
    );
    let bear = lk::obj(&e, p1, grizzly_bears());

    lk::float(
        &mut e,
        p0,
        &[(ManaColor::Black, 2), (ManaColor::Colorless, 1)],
    );
    lk::cast_at(&mut e, p0, shade_s_form(), &[bear], &[]);
    assert!(!lk::offered(
        &e,
        grizzly_bears(),
        p0,
        baylee_engine::choice::GRANTED_ABILITY
    ));

    // P1 activates {B} twice -> 4/4 until EOT, then reverts to 2/2 next turn.
    reach_their_main_phase(&mut e, p1);
    lk::float(&mut e, p1, &[(ManaColor::Black, 2)]);
    for _ in 0..2 {
        lk::use_ability(
            &mut e,
            p1,
            grizzly_bears(),
            baylee_engine::choice::GRANTED_ABILITY,
            &[],
            &[],
        );
    }
    assert_eq!(pt(&e, bear), (4, 4));

    reach_their_main_phase(&mut e, p0);
    assert_eq!(pt(&e, bear), (2, 2));
    lk::float(&mut e, p0, &[(ManaColor::Red, 1)]);
    lk::cast_at(&mut e, p0, lightning_bolt(), &[bear], &[]);

    let stolen = lk::obj(&e, p0, grizzly_bears());
    assert_eq!(e.state().object(stolen).unwrap().controller, p0);
    assert_eq!(e.state().object(stolen).unwrap().owner, p1);
}
