use super::*;
use crate::netplay4::{CatalogPolicy4, NetPlayer4};
use baylee_engine::choice::CastModeKind;
use std::path::PathBuf;

/// With four mana already floated, either land pays the printed cost. Only
/// the Swamp also allows the seat to pay four life. The checkpoint itself
/// answers the engine's cast and targeting questions.
fn play(net: &mut NetPlayer4, black_land: &str, alternative: bool) {
    let preset = game_preset(
        [&["Snuff Out"], &[]],
        [
            &[black_land, "Mountain", "Mountain", "Mountain"],
            &["Grizzly Bears"],
        ],
    );
    let mut session = driven_session(&preset);
    net.init_game(&preset);
    main_phase(&mut session);
    tap_lands(&mut session);
    let card = hand_object(&session, P0, "Snuff Out");
    apply_encoded(&mut session, &PlayerAction::CastSpell { card });
    let mut saw_alternative = false;
    let mut paid_life = false;
    for _ in 0..32 {
        if in_graveyard(&session, P0, "Snuff Out") {
            break;
        }
        let player = session.pending().asked().expect("the engine asks a seat");
        let (pending, view) = session.view_for(player).expect("the deciding seat's view");
        net.observe(&session.observe_view(P0)).unwrap();
        let action = if player == P0 && !matches!(pending, Pending::Priority { .. }) {
            net.act(&view, &pending, &[None, None])
        } else {
            PlayerAction::PassPriority
        };
        assert!(
            pending.answer_fault(&action).is_none(),
            "{action:?} for {pending:?}"
        );
        if let Pending::ChooseCastMode { options, .. } = &pending {
            let has = |mode| options.iter().any(|o| o.kind == mode);
            assert!(has(CastModeKind::Normal));
            assert_eq!(has(CastModeKind::Alternative(0)), alternative);
            saw_alternative |= has(CastModeKind::Alternative(0));
            let PlayerAction::ChooseMode(index) = action else {
                panic!("the seat did not choose an offered cost: {action:?}")
            };
            paid_life = options[index].kind == CastModeKind::Alternative(0);
        }
        session
            .act(player, action)
            .expect("the engine accepts the answer");
    }
    assert_eq!(saw_alternative, alternative);
    assert!(in_graveyard(&session, P0, "Snuff Out"));
    assert!(in_graveyard(&session, P1, "Grizzly Bears"));
    assert_eq!(
        session.state().players[0].life,
        if paid_life { 16 } else { 20 }
    );
}

#[test]
#[ignore = "requires an exported checkpoint in BAYLEE_TEST_MODEL"]
fn netplay4_answers_snuff_out_with_and_without_a_swamp() {
    let model = PathBuf::from(std::env::var_os("BAYLEE_TEST_MODEL").expect("BAYLEE_TEST_MODEL"));
    let catalog = if std::env::var("BAYLEE_TEST_CATALOG_DRIFT").as_deref() == Ok("1") {
        CatalogPolicy4::Drift
    } else {
        CatalogPolicy4::Exact
    };
    let mut net = NetPlayer4::load_with(&model, catalog)
        .unwrap()
        .with_temperature(0.0);
    play(&mut net, "Swamp", true);
    play(&mut net, "Vault of Whispers", false);
}
