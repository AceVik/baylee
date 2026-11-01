//! An-Zerrin Ruins — `{2}{R}{R}` Enchantment (Homelands)
//!
//! As An-Zerrin Ruins enters, choose a creature type.
//! Creatures of the chosen type don't untap during their controllers' untap
//! steps.

#[allow(clippy::wildcard_imports)]
use super::*;

fn an_zerrin_ruins() -> CardIndex {
    card_index("f805b100-49b8-4d23-a768-e8de8bb0daa3")
}

#[test]
fn an_zerrin_ruins_keeps_only_creatures_of_chosen_type_from_untapping() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .hand(0, &[an_zerrin_ruins()])
        .battlefield(
            0,
            &[
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                grizzly_bears(), // Bear -> chosen type, stays tapped
                gray_ogre(),     // Ogre -> untaps normally
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bear = on_battlefield(&engine, p0, grizzly_bears()).unwrap();
    let ogre = on_battlefield(&engine, p0, gray_ogre()).unwrap();

    // Cast An-Zerrin Ruins ({2}{R}{R}) and choose Bear as it enters.
    let ruins_hand = in_hand(&engine, p0, an_zerrin_ruins()).unwrap();
    tap_all_mana(&mut engine, p0);
    engine
        .apply(p0, PlayerAction::CastSpell { card: ruins_hand })
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseSubtype { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::ChooseSubtype(baylee_core::generated::subtypes::creature::BEAR),
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    // Attack with both Grizzly Bears and Gray Ogre so both tap.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(bear, Defender::Player(p1)), (ogre, Defender::Player(p1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });
    assert!(is_tapped(&engine, bear));
    assert!(is_tapped(&engine, ogre));

    // Advance through p1's turn to p0's next FirstMain.
    pass_until(&mut engine, |e| {
        e.state().turn.active == p1 && e.state().turn.phase == Phase::FirstMain
    });
    pass_until(&mut engine, |e| {
        e.state().turn.active == p0 && e.state().turn.phase == Phase::FirstMain
    });

    assert!(
        is_tapped(&engine, bear),
        "Bear does not untap during p0's untap step while An-Zerrin Ruins names Bear"
    );
    assert!(
        !is_tapped(&engine, ogre),
        "Ogre is not the chosen type and untaps normally"
    );
}
