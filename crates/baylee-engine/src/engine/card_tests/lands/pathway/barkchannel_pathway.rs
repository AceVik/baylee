//! `cards/lands/pathway/barkchannel_pathway.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Barkchannel Pathway // Tidechannel Pathway: Modal double-faced land.
/// Front face taps for {G}; back face Tidechannel Pathway taps for {U}.
/// Playing the card from hand and choosing face 1 puts Tidechannel Pathway onto the battlefield.
/// The land enters untapped and taps to add {U} to the mana pool.
#[test]
fn barkchannel_pathway_back_face_taps_for_blue() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(101, forest())
        .hand(0, &[barkchannel_pathway()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let card = in_hand(&engine, p0, barkchannel_pathway()).expect("pathway in hand");
    engine.apply(p0, PlayerAction::PlayLand { card }).unwrap();

    let Pending::ChooseCastMode {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!("expected face choice");
    };
    let slot = options
        .iter()
        .position(|o| matches!(o.kind, CastModeKind::PlayLandFace(1)))
        .expect("face 1 offered");
    engine
        .apply(player, PlayerAction::ChooseMode(slot))
        .unwrap();

    let land = on_battlefield(&engine, p0, barkchannel_pathway()).expect("land deployed");
    assert_eq!(engine.state().object(land).unwrap().face_index, 1);
    assert!(!is_tapped(&engine, land));

    activate(&mut engine, p0, barkchannel_pathway(), 0);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Blue),
        1
    );
    assert!(is_tapped(&engine, land));
}
