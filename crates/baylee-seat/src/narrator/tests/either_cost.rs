//! An either-or additional cost, as the seat is asked it and pays it, on a
//! real engine: Redirect Lightning's "pay 5 life or pay {2}" (CR 601.2b,
//! 601.2g, 118.8).

use super::carbon::{picked, send};
use super::carbon_b4::{asked_with_note, told, untapped};
use super::*;
use crate::wake::tests::played::position;
use baylee_engine::choice::{PlayerAction, YesNoPrompt};

/// gpt-6 Astra, ten ranked games of 27 and 28.11.2026: the option "Cast
/// Redirect Lightning {R} (taps Mountain)" tapped the {R} and cast off the
/// pool, and the 5 life was paid without a question although two more
/// Mountains could pay the {2}. The seat is asked which to pay now; told
/// "the second", it pays the {2} in the window that follows and keeps its
/// life.
#[test]
#[allow(clippy::too_many_lines)] // the whole cast, question by question, as the seat meets it
fn redirect_lightning_off_the_pool_asks_the_cost_and_pays_the_two_in_the_window() {
    let (mut session, view, _) = position(
        &["Mountain", "Mountain", "Mountain", "Mountain"],
        &["Redirect Lightning", "Shock"],
        &["Grizzly Bears"],
    );
    let held = |name: &str| {
        view.hand
            .iter()
            .find(|card| card.name == name)
            .unwrap_or_else(|| panic!("{name} in hand"))
            .id
    };
    let (redirect, shock) = (held("Redirect Lightning"), held("Shock"));
    let bears = view
        .battlefield
        .iter()
        .find(|o| o.controller != ME && o.name == "Grizzly Bears")
        .expect("their Bears")
        .id;

    // Shock at the Bears, the spell Redirect Lightning will point at.
    let mountain = untapped(&session, "Mountain")[0];
    session
        .act_slice(
            ME,
            PlayerAction::ActivateManaAbility { source: mountain },
            0,
        )
        .expect("a Mountain's red");
    session
        .act_slice(ME, PlayerAction::CastSpell { card: shock }, 0)
        .expect("{R} pays Shock");
    session
        .act_slice(
            ME,
            PlayerAction::ChooseTargets {
                objects: vec![bears],
                players: Vec::new(),
            },
            0,
        )
        .expect("the Bears are a legal target");
    let life = session.observe_view(ME).seats[0].life;

    // The seat's own option: it taps one Mountain for the printed {R}.
    let priority = asked_with_note(&session);
    assert!(
        matches!(priority.pending, Pending::Priority { .. }),
        "seat 0 holds priority over its Shock: {:?}",
        priority.pending
    );
    send(
        &mut session,
        picked(
            &priority,
            &format!("Cast Redirect Lightning {}", tag(redirect)),
        ),
    );

    let mut asked_cost = false;
    let mut paid_window = false;
    for _ in 0..8 {
        let asked = asked_with_note(&session);
        match asked.pending.clone() {
            Pending::YesNo {
                prompt: YesNoPrompt::ChooseFirstCostBranch { index: 0 },
                ..
            } => {
                let text = told(&asked);
                assert!(
                    text.contains("pay 5 life or pay {2}"),
                    "the question names the spell's cost: {text}"
                );
                send(
                    &mut session,
                    picked(&asked, "No: pay the second cost option"),
                );
                asked_cost = true;
            }
            Pending::ChooseTargets { options, .. } => {
                assert!(options.contains(&shock), "Shock is the target: {options:?}");
                session
                    .act_slice(
                        ME,
                        PlayerAction::ChooseTargets {
                            objects: vec![shock],
                            players: Vec::new(),
                        },
                        0,
                    )
                    .expect("Shock has a single target");
            }
            Pending::Priority { .. } if asked.view.owed.is_some() => {
                let text = told(&asked);
                assert!(
                    text.contains("Pay {2}"),
                    "the window offers the {{2}}: {text}"
                );
                send(&mut session, picked(&asked, "Pay {2}"));
                paid_window = true;
            }
            _ => break,
        }
    }

    assert!(asked_cost, "the seat was asked which cost to pay");
    assert!(paid_window, "the {{2}} was paid in the window");
    let after = session.observe_view(ME);
    assert!(
        after.stack.iter().any(|o| o.id == redirect),
        "Redirect Lightning is cast"
    );
    assert_eq!(after.seats[0].life, life, "no life was paid");
    assert!(
        untapped(&session, "Mountain").is_empty(),
        "all four Mountains paid: Shock's {{R}}, Redirect's {{R}} and the {{2}}"
    );
}
