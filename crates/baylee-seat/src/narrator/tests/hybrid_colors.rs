//! Choosing the colours that pay a hybrid cost when the spell reads the
//! colours of mana spent to cast it (CR 107.4e, 601.2g, 601.2h): Catharsis
//! evoked or cast from dual lands offers both `{R}{R}` and `{W}{W}`, while an
//! ordinary hybrid spell keeps one option per cast way.

use super::carbon::{picked, send};
use super::carbon_b4::{asked_with_note, told};
use super::*;
use crate::wake::tests::played::position;
use baylee_cards_dsl::KeywordSet;
use baylee_engine::choice::{ArrangePrompt, PlayerAction};
use baylee_gamehost::Session;

/// The option lines of `text` that start with `label`, trimmed of their ids.
fn cast_options<'a>(text: &'a str, needle: &str) -> Vec<&'a str> {
    text.lines()
        .map(str::trim_start)
        .filter_map(|line| line.split_once("  ").map(|(_, rest)| rest))
        .filter(|rest| rest.starts_with("Cast ") && rest.contains(needle))
        .collect()
}

/// Resolves the stack after casting Catharsis: answers any trigger-order
/// question with the evoke sacrifice first, and passes priority until the
/// stack is empty and seat 0 holds priority again.
fn resolve_catharsis(session: &mut Session) {
    for _ in 0..32 {
        let pending = session.pending().clone();
        let player = pending.asked().expect("the table stays playable");
        match pending {
            Pending::Priority { .. } => {
                if player == ME && session.observe_view(ME).stack.is_empty() {
                    return;
                }
                session
                    .act_slice(player, PlayerAction::PassPriority, 0)
                    .expect("pass priority");
            }
            Pending::Arrange {
                prompt: ArrangePrompt::TriggerOrder,
                ..
            } => {
                let ask = asked_with_note(session);
                let wake = Narrator::new(&ask.context).wake(&ask, &[], None);
                let trigger_ids: Vec<String> = wake
                    .text
                    .lines()
                    .map(str::trim_start)
                    .filter(|line| line.starts_with("#T"))
                    .filter_map(|line| line.split_whitespace().next().map(str::to_string))
                    .collect();
                let sacrifice = wake
                    .text
                    .lines()
                    .map(str::trim_start)
                    .find(|line| {
                        line.starts_with("#T") && line.to_ascii_lowercase().contains("sacrifice")
                    })
                    .and_then(|line| line.split_whitespace().next())
                    .unwrap_or_else(|| panic!("evoke sacrifice trigger in:\n{}", wake.text))
                    .to_string();
                let mut ordered = vec![sacrifice.clone()];
                for id in trigger_ids {
                    if id != sacrifice {
                        ordered.push(id);
                    }
                }
                let act = wake
                    .menu
                    .resolve(&Decision {
                        ask: Some(format!("q{}", ask.question)),
                        piles: vec![ordered],
                        ..Decision::default()
                    })
                    .expect("trigger order resolves")
                    .act;
                send(session, act);
            }
            other => panic!("unexpected question while resolving Catharsis: {other:?}"),
        }
    }
    panic!("Catharsis did not finish resolving");
}

/// Catharsis evoked (`{R/W}{R/W}`) with two Sacred Foundries: both `{R}{R}` and
/// `{W}{W}` tap plans are offered. Picking `{R}{R}` and ordering the evoke
/// sacrifice first pumps our existing 2/2 creature to 3/3 with haste; picking
/// `{W}{W}` creates two 1/1 Kithkin tokens instead.
#[test]
fn catharsis_evoke_offers_both_red_and_white_hybrid_payments() {
    // Case 1: pick the {R}{R} evoke option.
    let (mut session, _, _) = position(
        &["Sacred Foundry", "Sacred Foundry", "Grizzly Bears"],
        &["Catharsis"],
        &[],
    );
    let ask = asked_with_note(&session);
    let text = told(&ask);
    let evokes = cast_options(&text, "Catharsis");
    let rr = evokes
        .iter()
        .copied()
        .find(|line| line.matches("for {R}").count() == 2)
        .unwrap_or_else(|| panic!("expected a {{R}}{{R}} evoke option:\n{text}"));
    let ww = evokes
        .iter()
        .copied()
        .find(|line| line.matches("for {W}").count() == 2)
        .unwrap_or_else(|| panic!("expected a {{W}}{{W}} evoke option:\n{text}"));
    assert_ne!(rr, ww);

    let act = picked(&ask, rr);
    send(&mut session, act);
    resolve_catharsis(&mut session);

    let after = session.observe_view(ME);
    let bears = after
        .battlefield
        .iter()
        .find(|o| o.controller == ME && o.name.as_ref() == "Grizzly Bears")
        .expect("Grizzly Bears stays on the battlefield");
    assert_eq!(bears.power, Some(3), "pumped to 3/3 by {{R}}{{R}} trigger");
    assert_eq!(
        bears.toughness,
        Some(3),
        "pumped to 3/3 by {{R}}{{R}} trigger"
    );
    assert!(
        bears.keywords & KeywordSet::HASTE.bits() != 0,
        "gains haste from {{R}}{{R}} trigger"
    );
    assert!(
        !after
            .battlefield
            .iter()
            .any(|o| o.controller == ME && o.name.as_ref() == "Catharsis"),
        "evoked Catharsis was sacrificed"
    );

    // Case 2: pick the {W}{W} evoke option.
    let (mut session, _, _) = position(
        &["Sacred Foundry", "Sacred Foundry", "Grizzly Bears"],
        &["Catharsis"],
        &[],
    );
    let ask = asked_with_note(&session);
    let text = told(&ask);
    let evokes = cast_options(&text, "Catharsis");
    let ww = evokes
        .iter()
        .copied()
        .find(|line| line.matches("for {W}").count() == 2)
        .unwrap_or_else(|| panic!("expected a {{W}}{{W}} evoke option:\n{text}"));

    let act = picked(&ask, ww);
    send(&mut session, act);
    resolve_catharsis(&mut session);

    let after = session.observe_view(ME);
    let kithkin = after
        .battlefield
        .iter()
        .filter(|o| o.controller == ME && o.name.as_ref() == "Kithkin")
        .count();
    assert_eq!(kithkin, 2, "{{W}}{{W}} trigger creates two Kithkin tokens");
    let bears = after
        .battlefield
        .iter()
        .find(|o| o.controller == ME && o.name.as_ref() == "Grizzly Bears")
        .expect("Grizzly Bears stays on the battlefield");
    assert_eq!(
        bears.power,
        Some(2),
        "not pumped when only {{W}}{{W}} spent"
    );
    assert!(
        bears.keywords & KeywordSet::HASTE.bits() == 0,
        "no haste when only {{W}}{{W}} spent"
    );
}

/// Catharsis cast for its printed `{4}{R/W}{R/W}` with four Plains and two
/// Sacred Foundries offers an option tapping both Sacred Foundries for `{R}`,
/// reaching both the `{W}{W}` and `{R}{R}` triggers on the same cast.
#[test]
fn catharsis_hardcast_offers_red_hybrid_payment_alongside_plains() {
    let (mut session, _, _) = position(
        &[
            "Sacred Foundry",
            "Sacred Foundry",
            "Plains",
            "Plains",
            "Plains",
            "Plains",
            "Grizzly Bears",
        ],
        &["Catharsis"],
        &[],
    );
    let ask = asked_with_note(&session);
    let text = told(&ask);
    let hardcasts = cast_options(&text, "{4}{R/W}{R/W}");
    let rr = hardcasts
        .iter()
        .copied()
        .find(|line| line.matches("for {R}").count() == 2)
        .unwrap_or_else(|| panic!("expected a {{4}}{{R}}{{R}} hardcast option:\n{text}"));

    let act = picked(&ask, rr);
    send(&mut session, act);
    // Casting from the pool with 6 mana asks whether to cast normally or evoke;
    // pick the printed cost.
    if let Pending::ChooseCastMode { options, .. } = session.pending().clone() {
        let normal = options
            .iter()
            .position(|o| o.kind == baylee_engine::choice::CastModeKind::Normal)
            .expect("normal cast mode offered");
        session
            .act_slice(ME, PlayerAction::ChooseMode(normal), 0)
            .expect("normal mode chosen");
    }
    // Resolve stack (both {W}{W} and {R}{R} triggers fire, no evoke sacrifice).
    for _ in 0..32 {
        let pending = session.pending().clone();
        let player = pending.asked().expect("the table stays playable");
        match pending {
            Pending::Priority { .. } => {
                if player == ME && session.observe_view(ME).stack.is_empty() {
                    break;
                }
                session
                    .act_slice(player, PlayerAction::PassPriority, 0)
                    .expect("pass priority");
            }
            Pending::Arrange { cards, .. } => {
                session
                    .act_slice(ME, PlayerAction::Arrange { piles: vec![cards] }, 0)
                    .expect("order triggers");
            }
            other => panic!("unexpected question while hardcasting Catharsis: {other:?}"),
        }
    }

    let after = session.observe_view(ME);
    let kithkin = after
        .battlefield
        .iter()
        .filter(|o| o.controller == ME && o.name.as_ref() == "Kithkin")
        .count();
    assert_eq!(kithkin, 2, "{{W}}{{W}} spent creates two Kithkin tokens");
    let bears = after
        .battlefield
        .iter()
        .find(|o| o.controller == ME && o.name.as_ref() == "Grizzly Bears")
        .expect("Grizzly Bears stays on the battlefield");
    assert_eq!(bears.power, Some(3), "{{R}}{{R}} spent pumps to 3/3");
    assert!(
        bears.keywords & KeywordSet::HASTE.bits() != 0,
        "{{R}}{{R}} spent grants haste"
    );
}

/// An ordinary hybrid spell whose abilities do not read the colours of mana
/// spent to cast it keeps a single cast option (no menu growth).
#[test]
fn ordinary_hybrid_spell_keeps_one_option() {
    let (session, _, _) = position(
        &["Sacred Foundry", "Sacred Foundry", "Sacred Foundry"],
        &["Hearthfire Hobgoblin"],
        &[],
    );
    let ask = asked_with_note(&session);
    let text = told(&ask);
    let casts = cast_options(&text, "Hearthfire Hobgoblin");
    assert_eq!(
        casts.len(),
        1,
        "ordinary hybrid spell must keep one cast option:\n{text}"
    );
}
