//! `cards/lands/triome/ziatora_s_proving_ground.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Ziatora's Proving Ground is a triome land under `Coverage::Implemented` that enters tapped and produces {B}, {R}, or {G}.
/// Playing the land from hand puts it onto the battlefield tapped.
/// After advancing through an opponent's turn to its controller's next main phase, the land untaps.
/// Activating its mana ability opens a color choice among Black, Red, and Green, adding the chosen mana to the pool and tapping the land.
#[test]
fn ziatoras_proving_ground_enters_tapped_and_produces_three_colors() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[ziatora_s_proving_ground()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let land = play_land(&mut engine, p0, ziatora_s_proving_ground());
    assert!(
        entered_tapped(&engine, land),
        "Ziatora's Proving Ground enters tapped"
    );

    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);

    assert!(
        !is_tapped(&engine, land),
        "the land untaps on its controller's untap step"
    );

    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: land,
                ability_index: 0,
            },
        )
        .expect("activating mana ability is legal");

    let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
        panic!("expected color choice, got {:?}", engine.pending())
    };
    assert_eq!(options.len(), 3, "offers exactly three colors");
    assert!(options.contains(&ManaColor::Black), "offers Black");
    assert!(options.contains(&ManaColor::Red), "offers Red");
    assert!(options.contains(&ManaColor::Green), "offers Green");

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Green))
        .expect("choosing Green is legal");

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Green),
        1,
        "one green mana added to pool"
    );
    assert!(
        is_tapped(&engine, land),
        "the land is tapped after producing mana"
    );
}
