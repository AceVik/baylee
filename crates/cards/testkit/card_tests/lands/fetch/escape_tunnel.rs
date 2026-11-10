//! `cards/lands/fetch/escape_tunnel.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Escape Tunnel prints `{{T}}, Sacrifice this land: Search your library for a basic land card, put it onto the battlefield tapped, then shuffle` and an unblockability evasion ability.
/// The card is marked `Coverage::Partial` because targeting creatures by power comparison is unsupported and omitted.
/// Activating the fetch ability sacrifices Escape Tunnel and puts a searched basic land onto the battlefield tapped.
#[test]
fn escape_tunnel_sacrifices_to_fetch_basic_land_tapped() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[escape_tunnel()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let tunnel = play_land(&mut engine, p0, escape_tunnel());
    assert!(!is_tapped(&engine, tunnel));

    activate(&mut engine, p0, escape_tunnel(), 0);
    assert!(in_graveyard(&engine, p0, escape_tunnel()).is_some());

    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::ChooseCards {
                prompt: ChoicePrompt::SearchLibrary,
                ..
            }
        )
    });

    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        panic!("expected library search prompt");
    };
    assert!(!options.is_empty());
    let chosen = options[0];
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![chosen],
            },
        )
        .unwrap();

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().object(chosen).unwrap().zone,
        Zone::Battlefield
    );
    assert!(is_tapped(&engine, chosen));
}
