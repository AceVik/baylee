//! `cards/enchantments/auras/mv_2/ward_of_lights.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn ward_of_lights() -> CardIndex {
    card_index("edc05b33-2bfa-4692-bb3b-26b7cf111ce8")
}

/// Ward of Lights (`{W}{W}`, `Coverage::Partial`): an Aura that asks for a
/// color as it enters and gives the enchanted creature protection from it.
/// Red is chosen: Lightning Bolt (red) cannot name the Bears, and naming it is
/// refused; Unsummon (blue) is another color and still can.
#[test]
fn ward_of_lights_protects_from_the_chosen_color_only() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[plains(), plains(), grizzly_bears()])
        .hand(0, &[ward_of_lights()])
        .battlefield(1, &[mountain(), island(), grizzly_bears()])
        .hand(1, &[lightning_bolt(), unsummon()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let my_bear = on_battlefield(&engine, p0, grizzly_bears()).unwrap();
    let their_bear = on_battlefield(&engine, p1, grizzly_bears()).unwrap();
    cast_from_hand(&mut engine, p0, ward_of_lights());
    aim_at(&mut engine, p0, my_bear);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseColor { .. })
    });
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Red))
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(
        on_battlefield(&engine, p0, ward_of_lights()).is_some(),
        "the Aura is on the battlefield"
    );

    // On p1's turn the red Bolt cannot target my_bear.
    reach_their_main_phase(&mut engine, p1);
    cast_from_hand(&mut engine, p1, lightning_bolt());
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("a target question, got {:?}", engine.pending())
    };
    assert!(!options.contains(&my_bear), "protection from red");
    assert!(options.contains(&their_bear));
    assert!(
        engine
            .apply(
                p1,
                PlayerAction::ChooseTargets {
                    objects: vec![my_bear],
                    players: vec![],
                },
            )
            .is_err(),
        "the protected Bears cannot be named"
    );
    aim_at(&mut engine, p1, their_bear);
    pass_until(&mut engine, stack_is_empty);

    // The blue Unsummon is not red: the same Bears is a legal target.
    cast_with_floating(&mut engine, p1, unsummon());
    let menu = aim_at(&mut engine, p1, my_bear);
    assert!(menu.contains(&my_bear), "protection is from red only");
    pass_until(&mut engine, stack_is_empty);
    assert!(
        in_hand(&engine, p0, grizzly_bears()).is_some(),
        "the Bears went back to its owner's hand"
    );
}

/// "This effect doesn't remove this Aura": Ward of Lights is white, and with
/// white chosen the Bears has protection from white yet the Aura stays
/// attached after state-based actions.
#[test]
fn ward_of_lights_stays_attached_when_its_own_color_is_chosen() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[plains(), plains(), grizzly_bears()])
        .hand(0, &[ward_of_lights()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let bear = on_battlefield(&engine, p0, grizzly_bears()).unwrap();
    cast_from_hand(&mut engine, p0, ward_of_lights());
    aim_at(&mut engine, p0, bear);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseColor { .. })
    });
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::White))
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    let aura = on_battlefield(&engine, p0, ward_of_lights())
        .expect("a white Aura on a creature with protection from white stays");
    assert_eq!(
        engine.state().object(aura).and_then(|o| o.attached_to),
        Some(bear)
    );
}

/// Only the sorcery-speed half is played: the optional flash comes with a
/// sacrifice the DSL cannot say, so the Aura cannot be cast on the opponent's
/// turn even with the mana untapped and priority in hand.
#[test]
fn ward_of_lights_cannot_be_cast_on_the_opponents_turn() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[plains(), plains(), grizzly_bears()])
        .hand(0, &[ward_of_lights()])
        .battlefield(1, &[grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    pass_until(&mut engine, |e| {
        e.state().turn.active == p1
            && e.state().turn.phase == Phase::FirstMain
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p0)
    });
    tap_all_mana(&mut engine, p0);
    let aura = in_hand(&engine, p0, ward_of_lights()).expect("still in hand");
    assert!(
        engine
            .apply(p0, PlayerAction::CastSpell { card: aura })
            .is_err(),
        "no flash, no instant-speed cast"
    );
    assert!(in_hand(&engine, p0, ward_of_lights()).is_some());
}
