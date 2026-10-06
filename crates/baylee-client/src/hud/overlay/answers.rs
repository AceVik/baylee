//! The words on an armed deed and the answer buttons.

#[allow(clippy::wildcard_imports)] // the overlay's shared vocabulary
use super::*;

/// What an armed deed calls itself: the words, and the mana those words name.
///
/// The two are separate because a cost is **drawn** and not spelled. A `Text`
/// node can hold the characters `{4}{U}{U}`, and that is what the deck
/// builder deliberately does not do — `manaui::spawn_pip` sets each symbol on
/// its own coloured disc, and this row now says the price the same way every
/// other price in this client is said.
pub(in crate::hud) struct ArmedWords {
    /// The phrase. When [`Self::cost`] is `Some`, its `{0}` marks where the
    /// pips go and the two halves are laid out either side of them; a phrase
    /// with no `{0}` simply takes the pips after its last word.
    pub(in crate::hud) text: String,
    /// The mana this deed spends, or `None` when it spends none.
    pub(in crate::hud) cost: Option<baylee_core::mana::ManaCost>,
}

/// What an armed deed calls itself, or `None` when the engine no longer
/// offers it.
///
/// Resolved against the *current* `LegalActions` here as well as at the two
/// places that fire it, which is what stops a row drawn a frame ago from
/// offering something that has since been withdrawn. The row simply
/// disappears; the state itself is cleared by the next key or tap, both of
/// which run the same resolution.
pub(in crate::hud) fn armed_label(
    duel: &Duel,
    lang: Lang,
    armed: &crate::Armed,
) -> Option<ArmedWords> {
    match &armed.deed {
        crate::Deed::Play => duel
            .interaction
            .as_ref()
            .and_then(|i| i.play_card(armed.object))
            .map(|_| ArmedWords {
                text: Phrase::ArmedPlay.text(lang).to_string(),
                // The engine offered this cast, so the mana is already
                // floating (`casting::can_cast` checks the pool): there is no
                // price left to quote.
                cost: None,
            }),
        // The retained abilities dialog shows the complete printed sentence
        // and costs. Confirmation names the action without repeating that text.
        crate::Deed::Ability(action) => super::ability_options(duel, lang, armed.object)?
            .into_iter()
            .any(|option| option.action == *action)
            .then(|| ArmedWords {
                text: Phrase::ArmedActivate.text(lang).to_string(),
                cost: None,
            }),
        // The owner's report: this said "Tap 3, then cast", which is a fact
        // about the client's plan and not about the spell. What a player
        // needs to read before spending a turn's lands is the *price* —
        // `{4}{U}{U}` — so the plan carries the cost it was built for and
        // the row draws it.
        crate::Deed::Run { plan, then } => {
            if matches!(then, crate::RunEnd::Cast)
                && duel
                    .cast_answer
                    .is_some_and(|(card, _)| card == armed.object)
            {
                let current = crate::input::chosen_cast_plan(duel, armed.object)?;
                return Some(ArmedWords {
                    text: Phrase::ArmedPayAndCast.text(lang).to_string(),
                    cost: Some(current.cost),
                });
            }
            if let crate::RunEnd::Ability(index) = then {
                let view = duel.view.as_ref()?;
                let legal = duel.interaction.as_ref()?.legal_actions()?;
                let cost = crate::abilities::mana_for(view, legal, armed.object, *index)
                    .map(|current| current.cost)
                    .or_else(|| {
                        legal
                            .abilities
                            .contains(&(armed.object, *index))
                            .then_some(plan.cost)
                    })?;
                return Some(ArmedWords {
                    text: Phrase::ArmedPayAndActivate.text(lang).to_string(),
                    cost: Some(cost),
                });
            }
            let offered = match then {
                crate::RunEnd::Cast => Some((&duel.reachable, Phrase::ArmedPayAndCast)),
                crate::RunEnd::Suspend => Some((&duel.suspend_reach, Phrase::ArmedSuspend)),
                // A pour is never armed: [`crate::input::arm_ability`] starts
                // its run on the press that picks the colour, which is the
                // whole of "the card taps once the mana has been chosen". So
                // there is no row to draw here, and no price to quote either
                // — the mana *is* the point, and it costs a tap.
                // A settle is never armed either: `Duel::pay_owed` runs it on
                // the press that asks for it.
                crate::RunEnd::Float | crate::RunEnd::Ability(_) | crate::RunEnd::Settle => None,
            };
            offered
                .filter(|(offered, _)| offered.contains(&armed.object))
                .map(|(_, phrase)| ArmedWords {
                    text: phrase.text(lang).to_string(),
                    cost: Some(plan.cost),
                })
        }
        // The engine is already offering the suspend, so the cost is floating
        // and there is nothing left to quote — the same reason `Play` quotes
        // nothing.
        crate::Deed::Suspend => duel
            .interaction
            .as_ref()
            .and_then(|i| i.suspend(armed.object))
            .map(|_| ArmedWords {
                text: Phrase::ArmedSuspendNow.text(lang).to_string(),
                cost: None,
            }),
    }
}

/// One arm of the number stepper.
/// Where the prompt slip stands.
///
/// A full-width row that centres its one child, rather than a panel pinned to
/// a corner. The slip is the one thing on screen that *must* be answered, and
/// it used to sit in the bottom-right — the corner furthest from the two
/// things a player is already looking at, their own board and the hand under
/// it. Centred over the near edge of that board, the question and the cards
/// that answer it are one place to look.
///
/// The row itself is `Pickable::IGNORE` and paints nothing: it spans the
/// window so that its child can be centred in it, and takes no click away
/// from the table it lies over.
/// One answer on the prompt slip.
///
/// `flex_grow: 1` with a `flex_basis` of **zero** is the whole promise: grow
/// alone divides the *slack* left over after the labels, so "Aim next" and
/// "Declare none" would still come out different widths. A basis of zero
/// takes the labels out of the sum, and the row is cut into equal parts.
pub(in crate::hud) fn answer_node() -> Node {
    Node {
        flex_grow: 1.0,
        flex_basis: px(0),
        flex_direction: FlexDirection::Row,
        align_items: AlignItems::Center,
        justify_content: JustifyContent::Center,
        padding: UiRect::axes(px(10), px(7)),
        border: UiRect::all(px(1)),
        border_radius: btn_radius(),
        ..default()
    }
}

/// One answer on a parchment sheet.
///
/// `lead` is the answer the sheet is *for* — keep, confirm, pass, play again
/// — and is the only one drawn in brass; the rest are the same button in the
/// sheet's own colour, because two equally loud answers make a player read
/// both before finding out which one the sheet meant. A lone answer is a lead
/// answer: there is nothing left for it to be quieter than.
///
/// It carries no marker of its own, because two sheets use it and they name
/// their answers differently: the prompt slip adds a [`PromptButton`] with a
/// `PlayerAction` on it, and the end screen's exits carry the lobby's `Press`
/// — a way *out* of a duel is not a move in one. A widget that knew which of
/// those it was would be a widget only one of them could use.
pub(crate) fn answer_button(
    commands: &mut Commands,
    fonts: &UiFonts,
    label: &str,
    lead: bool,
) -> Entity {
    let rest = if lead {
        palette::BRASS
    } else {
        palette::SLIP_GHOST
    };
    commands
        .spawn((
            answer_node(),
            BackgroundColor(rest),
            BorderColor::all(if lead {
                palette::BRASS
            } else {
                palette::PARCHMENT_EDGE
            }),
            soft_shadow(),
            // The same component every other button in the client carries:
            // `ambience::feel` leans it towards the pointer, sinks it under a
            // press and owns its `BackgroundColor` from the first frame on.
            Feel::new(rest),
            children![(
                Text::new(label),
                tf_bold(fonts, ANSWER_PT),
                TextColor(if lead {
                    palette::PARCHMENT_INK
                } else {
                    palette::PARCHMENT_SOFT
                }),
                // A label is a `Node`, and a node under the pointer is what
                // the pointer is *over*: without this the button only ever lit
                // up when the pointer was in its padding, and went dead the
                // moment it crossed the word it is named after. Measured, not
                // guessed — hovering the padding moved 155 levels and hovering
                // the word moved none.
                Pickable::IGNORE,
            )],
        ))
        .id()
}
