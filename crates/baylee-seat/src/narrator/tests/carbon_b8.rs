//! What the Carbon seat was told wrong in block 8 (12.11.2026), on a real
//! engine: a band question that offered more than a band may hold.

use super::carbon_b4::{asked_with_note, told};
use super::*;
use crate::wake::tests::played::position;
use baylee_core::ids::Defender;
use baylee_engine::choice::PlayerAction;

/// Carbon b8, segment 3, attempt 37, seat 1, q408: Kjeldoran Skycaptain
/// (banding) attacking beside two Ashen Ghouls (no banding) was told
/// `pick=[up to 2 ids from the list]`, and a band holds one creature without
/// banding at most (CR 702.22c). The question offers one, and says why.
#[test]
fn a_band_question_offers_one_creature_without_banding_and_says_so() {
    let (mut session, _, _) = position(
        &["Kjeldoran Skycaptain", "Ashen Ghoul", "Ashen Ghoul"],
        &[],
        &[],
    );
    for _ in 0..12 {
        match session.pending().clone() {
            Pending::ChooseAttackers { player, .. } if player == ME => break,
            Pending::Priority { player, .. } => {
                session
                    .act_slice(player, PlayerAction::PassPriority, 0)
                    .expect("a pass");
            }
            other => panic!("on the way to the attack: {other:?}"),
        }
    }
    let attackers: Vec<(ObjectId, Defender)> = session
        .observe_view(ME)
        .battlefield
        .iter()
        .filter(|o| o.controller == ME && o.power.is_some())
        .map(|o| (o.id, Defender::Player(THEM)))
        .collect();
    assert_eq!(attackers.len(), 3);
    session
        .act_slice(ME, PlayerAction::DeclareAttackers { attackers }, 0)
        .expect("the three attack");
    let asked = asked_with_note(&session);
    assert!(
        matches!(asked.pending, Pending::ChooseCards { max: 1, .. }),
        "{:?}",
        asked.pending
    );
    let text = told(&asked);
    assert!(
        text.contains("any number with banding and at most one without"),
        "{text}"
    );
    assert!(
        text.contains("pick=[one id from the list], or pick=[] for none"),
        "{text}"
    );
    assert!(!text.contains("up to 2"), "{text}");
}
