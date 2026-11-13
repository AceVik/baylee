//! What the Carbon seat was offered wrong in block 7 (12.11.2026), on a real
//! engine: a spell's X was capped at what the unrestricted mana pays, where
//! a restricted mana ability's mana may pay the spell too (CR 106.6).

use super::carbon_b4::{asked_with_note, told, untapped};
use super::*;
use crate::wake::tests::played::position;
use baylee_engine::choice::{PlayerAction, Spend};
use baylee_gamehost::Session;

/// Taps each of seat 0's `lands` for its mana, and `creature`'s first
/// ability where one is named, before anything is cast.
fn float(session: &mut Session, lands: &[ObjectId], creature: Option<ObjectId>) {
    for &land in lands {
        session
            .act_slice(ME, PlayerAction::ActivateManaAbility { source: land }, 0)
            .expect("a land's mana");
    }
    if let Some(source) = creature {
        session
            .act_slice(
                ME,
                PlayerAction::ActivateAbility {
                    source,
                    ability_index: 0,
                },
                0,
            )
            .expect("the creature's mana");
    }
}

/// Casts `card` from seat 0's hand, takes its first mode where it asks one
/// (Invoke the Firemind's "Draw X cards"), and returns the line of the X
/// question as the seat is told it.
fn x_line(session: &mut Session, card: ObjectId) -> String {
    session
        .act_slice(ME, PlayerAction::CastSpell { card }, 0)
        .expect("the spell is castable at X = 0");
    if matches!(session.pending(), Pending::ChooseCastMode { .. }) {
        session
            .act_slice(ME, PlayerAction::ChooseMode(0), 0)
            .expect("the first mode");
    }
    let asked = asked_with_note(session);
    assert!(
        matches!(asked.pending, Pending::ChooseNumber { .. }),
        "X is asked: {:?}",
        asked.pending
    );
    let text = told(&asked);
    text.lines()
        .find(|line| line.contains("value of X"))
        .unwrap_or_else(|| panic!("X is asked: {text}"))
        .to_string()
}

/// Seat 0's permanent named `name`, tapped or not.
fn permanent(session: &Session, name: &str) -> ObjectId {
    session
        .observe_view(ME)
        .battlefield
        .iter()
        .find(|o| o.controller == ME && &*o.name == name)
        .unwrap_or_else(|| panic!("{name} is on seat 0's battlefield"))
        .id
}

/// Announces `x` and pays the window it opens with every mana ability on
/// offer, the restricted taps the engine names for it included, then passes
/// to spend the mana. Returns the restricted taps the engine named, and
/// asserts that `spell` is on the stack at that X.
fn pay_x(session: &mut Session, spell: ObjectId, x: u32) -> Vec<(ObjectId, u32)> {
    session
        .act_slice(ME, PlayerAction::ChooseNumber(x), 0)
        .expect("the X is on offer");
    let mut named = Vec::new();
    for _ in 0..8 {
        let (pending, view) = session.view_for(ME).expect("seat 0 is asked");
        let Pending::Priority { legal, .. } = pending else {
            panic!("the window asks for mana: {pending:?}");
        };
        if view.owed.is_none() {
            break;
        }
        let restricted = legal.restricted_taps_for(Spend::Owed);
        for tap in &restricted {
            if !named.contains(tap) {
                named.push(*tap);
            }
        }
        let action = if let Some(&source) = legal.mana_abilities.first() {
            PlayerAction::ActivateManaAbility { source }
        } else if let Some(&(source, ability_index)) = restricted.first() {
            PlayerAction::ActivateAbility {
                source,
                ability_index,
            }
        } else {
            PlayerAction::PassPriority
        };
        session
            .act_slice(ME, action, 0)
            .expect("the window's answer");
    }
    let cast = session
        .observe_view(ME)
        .stack
        .iter()
        .find(|o| o.id == spell)
        .map(|o| o.chosen_x);
    assert_eq!(
        cast,
        Some(Some(x)),
        "cast at X = {x}: {:?}",
        session.pending()
    );
    named
}

/// Carbon b7, segment 3, attempt 22, seat 0, q213-q214: Invoke the Firemind
/// (`{X}{U}{U}{R}`, a sorcery) cast off a floating `{U}{U}{R}` beside two
/// untapped lands and Vodalian Arcanist, whose `{C}` may be spent "only to
/// cast an instant or sorcery spell". The lands pay two and the Arcanist the
/// third, so X = 3 is paid (CR 106.6, 107.3a, 601.2b); the seat was asked
/// "from 0 to 2", what the lands pay alone, and the plan's `n 3` was
/// refused. X = 3 is then paid through the window, the Arcanist's mana
/// among it, so the 3 the question offers is one the cast completes.
#[test]
fn a_restricted_mana_ability_that_may_pay_the_spell_counts_toward_its_x() {
    let (mut session, view, _) = position(
        &[
            "Island",
            "Island",
            "Island",
            "Island",
            "Mountain",
            "Vodalian Arcanist",
        ],
        &["Invoke the Firemind"],
        &[],
    );
    let firemind = view.hand[0].id;
    let islands = untapped(&session, "Island");
    let mountain = untapped(&session, "Mountain")[0];
    float(&mut session, &[islands[0], islands[1], mountain], None);
    let line = x_line(&mut session, firemind);
    assert!(line.contains("(from 0 to 3)"), "{line}");

    let arcanist = permanent(&session, "Vodalian Arcanist");
    let named = pay_x(&mut session, firemind, 3);
    assert!(
        named.contains(&(arcanist, 0)),
        "the engine names the Arcanist's mana for the window: {named:?}"
    );
    assert!(
        session
            .observe_view(ME)
            .object(arcanist)
            .is_some_and(|o| o.status.contains(baylee_view::ObjectStatus::TAPPED)),
        "the Arcanist paid"
    );
}

/// The same board with Ancient Ziggurat in the Arcanist's place: its mana may
/// be spent only on a creature spell, so it pays nothing of a sorcery and the
/// lands' X = 2 stands (CR 106.6).
#[test]
fn a_restricted_mana_ability_that_may_not_pay_the_spell_is_not_counted() {
    let (mut session, view, _) = position(
        &[
            "Island",
            "Island",
            "Island",
            "Island",
            "Mountain",
            "Ancient Ziggurat",
        ],
        &["Invoke the Firemind"],
        &[],
    );
    let firemind = view.hand[0].id;
    let islands = untapped(&session, "Island");
    let mountain = untapped(&session, "Mountain")[0];
    float(&mut session, &[islands[0], islands[1], mountain], None);
    let line = x_line(&mut session, firemind);
    assert!(line.contains("(from 0 to 2)"), "{line}");
}

/// The Arcanist's mana for a creature spell: Endless One (`{X}`) beside two
/// Islands and the Arcanist is X = 2, the Islands' (CR 106.6).
#[test]
fn instant_and_sorcery_mana_is_not_counted_for_a_creature_spell() {
    let (mut session, view, _) = position(
        &["Island", "Island", "Vodalian Arcanist"],
        &["Endless One"],
        &[],
    );
    let line = x_line(&mut session, view.hand[0].id);
    assert!(line.contains("(from 0 to 2)"), "{line}");
}

/// Invoke the Firemind cast after two Islands, a Mountain and Vodalian
/// Arcanist were tapped, with `spare` more Islands untapped: `{U}{U}{R}` and
/// the Arcanist's restricted `{C}` float. Returns the X question's line, the
/// session and the spell.
fn arcanist_floated(spare: usize) -> (String, Session, ObjectId) {
    let mut lands = vec!["Island", "Island", "Mountain", "Vodalian Arcanist"];
    lands.extend(std::iter::repeat_n("Island", spare));
    let (mut session, view, _) = position(&lands, &["Invoke the Firemind"], &[]);
    let firemind = view.hand[0].id;
    let tapped: Vec<ObjectId> = untapped(&session, "Island")[..2]
        .iter()
        .copied()
        .chain(untapped(&session, "Mountain"))
        .collect();
    let arcanist = permanent(&session, "Vodalian Arcanist");
    float(&mut session, &tapped, Some(arcanist));
    let line = x_line(&mut session, firemind);
    (line, session, firemind)
}

/// The Arcanist's `{C}` floating counts toward the X: the view says it is
/// restricted colourless mana and not what it may pay for, which no offer
/// states at an X question, and mana floating then was made before the cast
/// began (CR 601.2b, 601.2g), here for this spell. With an Island untapped
/// X = 2 is offered and paid. Counted with the unrestricted mana alone it
/// was "from 0 to 1"; left to the engine, whose range is open while the
/// Island is untapped, "from 0 to 4294967295".
#[test]
fn restricted_mana_floating_counts_toward_the_x() {
    let (line, mut session, firemind) = arcanist_floated(1);
    assert!(line.contains("(from 0 to 2)"), "{line}");
    pay_x(&mut session, firemind, 2);
}

/// With nothing left untapped the engine's own range is what the floating
/// mana pays (X = 1, the `{C}` included), and the seat's bound is the same;
/// the unrestricted mana alone asked "from 0 to 0".
#[test]
fn restricted_mana_floating_with_nothing_untapped_agrees_with_the_engine() {
    let (line, mut session, firemind) = arcanist_floated(0);
    assert!(line.contains("(from 0 to 1)"), "{line}");
    pay_x(&mut session, firemind, 1);
}
