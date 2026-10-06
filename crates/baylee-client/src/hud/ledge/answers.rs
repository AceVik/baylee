//! What the shelf says: the rows of answers for each kind of question.

#[allow(clippy::wildcard_imports)] // the ledge's shared vocabulary
use super::*;

/// What one button in the middle sends.
///
/// Two mechanisms wearing one shape, which is the honest way round: an
/// [`Answer`](Says::Answer) replies to the question the engine asked and rides
/// a [`PromptButton`]; a [`Command`](Says::Command) states a condition and
/// rides a [`MenuButton`], reaching the game by the road that button's key
/// already takes.
///
/// §10.1 item 7 of the design is about the one command there is: "resolve the
/// stack" is a condition rather than a reply, and stands in the row of replies
/// anyway, because while there *is* a stack it answers the question above it —
/// no, to none of that.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(in crate::hud) enum Says {
    /// An answer to the engine's question.
    Answer(PromptAction),
    /// Something done to the game beside answering it.
    Command(super::MenuAction),
}

/// Which answers this question takes, in the order they are offered.
///
/// Lifted out of `sync_overlay` with the two suppressions that are easy to
/// read as bugs intact. `cast_menu` takes the answers away because the
/// engine's window *behind* the client's own chooser is an ordinary priority,
/// and "Pass priority" under "Choose how it is cast" is two primary answers
/// saying opposite things. `elsewhere` takes the Confirm away because the zone
/// browser's footer already draws one, and a player ticking a fetchland's
/// target saw the same word twice on one screen.
///
/// What did not come from `sync_overlay` is the middle button of a priority,
/// which had no button at all before the shelf: see [`Says`].
/// The three answers to a combat declaration, with `commit` named for the
/// side of it this seat is on.
///
/// Its own function because both arms of [`answers_for`] need the same
/// guard and differ only in one word. The guard is
/// [`crate::input::empty_combat_declaration`] rather than a second reading
/// of the same two fields, so the button and the key cannot disagree about
/// when the answer exists.
fn combat_row(duel: &Duel, lang: Lang, commit: Phrase) -> Vec<(Says, String)> {
    let mut row = vec![(
        Says::Answer(PromptAction::AimNext),
        Phrase::AimNext.text(lang).to_string(),
    )];
    if !duel
        .interaction
        .as_ref()
        .is_some_and(crate::input::empty_combat_declaration)
    {
        row.push((
            Says::Answer(PromptAction::Confirm),
            commit.text(lang).to_string(),
        ));
    }
    row.push((
        Says::Answer(PromptAction::DeclareNothing),
        Phrase::DeclareNone.text(lang).to_string(),
    ));
    row
}

/// The answers to a payment window (CR 605.3a): pay, don't, and the granted
/// actions that could make the mana.
fn payment_row(duel: &Duel, lang: Lang) -> Vec<(Says, String)> {
    let say = |action, phrase: Phrase| (Says::Answer(action), phrase.text(lang).to_string());
    // With no plan (nothing this client can tap pays it, or a
    // granted action has to make the mana first) "Pay" is the plain
    // pass, and the engine settles with whatever is floating.
    let mut row = vec![match duel.owed_plan.as_ref() {
        Some(plan) if !plan.is_empty() => (
            Says::Answer(PromptAction::Confirm),
            Phrase::PayRemainder.fill(lang, &[&plan.taps().to_string()]),
        ),
        _ => say(PromptAction::Confirm, Phrase::PayNow),
    }];
    row.push((
        Says::Command(super::MenuAction::DeclinePayment),
        Phrase::DeclinePayment.text(lang).to_string(),
    ));
    if !super::granted_offers(duel).is_empty() {
        row.push((
            Says::Command(super::MenuAction::ToggleGrantedActions),
            Phrase::GrantedActions.text(lang).to_string(),
        ));
    }
    row
}

pub(super) fn answers_for(
    duel: &Duel,
    lang: Lang,
    over: bool,
    waiting: bool,
    elsewhere: bool,
) -> Vec<(Says, String)> {
    use baylee_engine::choice::Pending;
    // Above the suppression and not below it: the hold exists **only** while
    // this seat is not being asked, so a branch under that `return` would be
    // dead code that looked like a feature.
    if holding(duel, over, waiting) {
        return vec![(
            Says::Command(super::MenuAction::ReleaseHold),
            Phrase::HoldRelease.text(lang).to_string(),
        )];
    }
    if over || waiting || duel.cast_menu.is_some() {
        return Vec::new();
    }
    let say = |action: PromptAction, phrase: Phrase| {
        (Says::Answer(action), phrase.text(lang).to_string())
    };
    match duel
        .interaction
        .as_ref()
        .map(baylee_client_core::Interaction::pending)
    {
        // No further mulligan once the hand would open with zero cards
        // (CR 103.5): the question says so, and the button goes with it.
        Some(Pending::Mulligan { can_take, .. }) => {
            let mut row = vec![say(PromptAction::Keep, Phrase::KeepHand)];
            if *can_take {
                row.push(say(PromptAction::Mulligan, Phrase::TakeMulligan));
            }
            row
        }
        Some(pending @ Pending::YesNo { .. }) => {
            let mut row = vec![
                say(PromptAction::Yes, Phrase::ActAnswerYes),
                say(PromptAction::No, Phrase::ActAnswerNo),
            ];
            let count = duel
                .view
                .as_ref()
                .map_or(0, |v| crate::yes_batch::candidates(v, pending).len());
            if count > 1 {
                row.push((
                    Says::Answer(PromptAction::YesBatch),
                    Phrase::AnswerYesBatch.fill(lang, &[&count.to_string()]),
                ));
            }
            row
        }
        // Priority is not confirmed, it is *passed*, and the two words are
        // not interchangeable on a button: "OK" acknowledges something that
        // has already happened. "Skip turn" beside it is the same decision at
        // the larger size, and is where the phase rail's fast-forward went.
        //
        // Between them, and only while there is a stack to let go: three
        // buttons, three mechanisms — the engine's own answer, an engine hold,
        // and a client-side autopilot that never reaches the wire at all. They
        // look alike on purpose; what they have in common is that each of them
        // is a way of saying "not now".
        // A payment window (CR 605.3a) is answered "pay" or "don't": the
        // confirm taps whatever is still owed and settles it
        // (`Duel::pay_owed`), and the other passes it unpaid. Lands can
        // still be tapped one by one before either; the plan is for what is
        // left.
        Some(Pending::Priority { .. }) if duel.paying() => payment_row(duel, lang),
        Some(Pending::Priority { .. }) => {
            let mut row = vec![say(PromptAction::Confirm, Phrase::PassPriority)];
            if duel.can_hold_for_stack() {
                row.push((
                    Says::Command(super::MenuAction::HoldForStack),
                    Phrase::ResolveTheStack.text(lang).to_string(),
                ));
            }
            row.push(say(PromptAction::SkipTurn, Phrase::SkipTheTurn));
            if !super::granted_offers(duel).is_empty() {
                row.push((
                    Says::Command(super::MenuAction::ToggleGrantedActions),
                    Phrase::GrantedActions.text(lang).to_string(),
                ));
            }
            row
        }
        // Combat offers three answers and shows the middle one only once
        // there is something to send. "None" is a real answer and the step
        // does not end without one, so both of the others stand whatever is
        // declared — but the confirm key is the key a player is already
        // pressing to walk the turn forward, and while nothing is declared
        // "Attack" and "None" do the same thing by two different names, one
        // of them wearing that key. A row that offered both was the legend
        // that taught the mistake; `crate::input::committed_answer` is the
        // half that stops the key, and this is the half that stops
        // advertising it.
        Some(Pending::ChooseAttackers { .. }) => combat_row(duel, lang, Phrase::Attack),
        Some(Pending::ChooseBlockers { .. }) => combat_row(duel, lang, Phrase::Block),
        Some(Pending::DiscardChoice { count, .. })
            if duel
                .interaction
                .as_ref()
                .is_some_and(baylee_client_core::Interaction::can_confirm) =>
        {
            vec![(
                Says::Answer(PromptAction::Confirm),
                Phrase::counted(
                    usize::from(*count),
                    Phrase::DiscardCard,
                    Phrase::DiscardCards,
                )
                .fill(lang, &[&count.to_string()]),
            )]
        }
        Some(_)
            if !elsewhere
                && duel
                    .interaction
                    .as_ref()
                    .is_some_and(baylee_client_core::Interaction::can_confirm) =>
        {
            confirmation_row(duel, lang)
        }
        _ => Vec::new(),
    }
}

/// Retarget identity and instructions live in the wrapping drawer above the shelf.
pub(super) fn shelf_headline(
    duel: &Duel,
    lang: Lang,
    texts: &crate::cardtext::CardTexts,
) -> Option<String> {
    let headline = base_shelf_headline(duel, lang, texts)?;
    if let Some(view) = duel.view.as_ref() {
        let subject = baylee_client_core::decision::resource_player(view);
        if subject != view.seat {
            let who = baylee_client_core::i18n::seat_name(lang, duel.statics.as_ref(), subject);
            return Some(Phrase::DecidingFor.fill(lang, &[&who]));
        }
    }
    Some(headline)
}

pub(super) fn base_shelf_headline(
    duel: &Duel,
    lang: Lang,
    texts: &crate::cardtext::CardTexts,
) -> Option<String> {
    if !super::granted_offers(duel).is_empty()
        && duel
            .view
            .as_ref()
            .is_some_and(|view| matches!(view.owed, Some(baylee_core::mana::ManaPayment::Fixed(_))))
    {
        return Some(Phrase::GrantedPaymentHint.text(lang).to_string());
    }

    if duel.interaction.as_ref().is_some_and(|i| {
        matches!(
            i.pending(),
            baylee_engine::choice::Pending::ChooseTargets {
                reason: baylee_engine::choice::TargetPrompt::Retarget { .. },
                ..
            }
        )
    }) {
        Some(Phrase::ChooseNewTarget.text(lang).to_string())
    } else {
        duel.headline(lang, texts)
    }
}

/// The manual answer and explicit batch choices for the current decision.
pub(super) fn confirmation_row(duel: &Duel, lang: Lang) -> Vec<(Says, String)> {
    use baylee_engine::choice::Pending;
    let Some(interaction) = duel.interaction.as_ref() else {
        return Vec::new();
    };
    let say = |action, phrase: Phrase| (Says::Answer(action), phrase.text(lang).to_string());
    let confirm = if matches!(
        interaction.pending(),
        Pending::ChooseTargets {
            reason: baylee_engine::choice::TargetPrompt::Retarget { .. },
            ..
        }
    ) {
        if interaction.selected().next().is_none()
            && interaction.selected_players().next().is_none()
        {
            Phrase::KeepCurrentTarget
        } else {
            Phrase::ChangeTarget
        }
    } else {
        Phrase::ConfirmOk
    };
    let mut answers = vec![say(PromptAction::Confirm, confirm)];
    if matches!(
        interaction.pending(),
        Pending::ChooseNumber {
            reason: baylee_engine::choice::NumberPrompt::CombatDamage { .. },
            ..
        }
    ) {
        answers.push(say(PromptAction::AutoDamage, Phrase::AutoCombatDamage));
    }
    if let Some((i, v)) = duel.interaction.as_ref().zip(duel.view.as_ref())
        && let Some(baylee_engine::choice::PlayerAction::ChooseTargetBatch { count, .. }) =
            baylee_client_core::targeting::batch_answer(i, v)
    {
        answers.push((
            Says::Answer(PromptAction::TargetBatch),
            Phrase::TargetingBatch.fill(lang, &[&count.to_string()]),
        ));
    }
    answers
}

/// Whether the shelf is showing a running hold instead of a question.
///
/// A hold is the one game state with **no other symptom**: the middle is
/// empty precisely *because* the seat is not being asked, which is exactly
/// what an idle shelf looks like. A player who set one two turns ago and
/// forgot would watch the game play itself with nothing on screen to blame.
///
/// Two mechanisms, one picture (§4.4): the engine's own hold, which a view
/// reports, and the client's autopilot, which never reaches the wire at all.
/// They differ in one thing only and it is the keycap — see [`keys_for`].
///
/// `cast_menu` takes it away for the reason it takes the answers away: the
/// engine's window behind the client's own chooser is an ordinary priority,
/// and the chooser is a question this seat is very much being asked.
pub(super) fn holding(duel: &Duel, over: bool, waiting: bool) -> bool {
    !over
        && waiting
        && duel.cast_menu.is_none()
        && (duel.priority_held() || duel.autopilot.is_some())
}

/// The legend on each answer's keycap, or `None` where the answer has no key.
///
/// **Always out of the keymap**, never out of a string in the code: a player
/// who rebinds `Space` sees the new chord on the next frame.
/// [`baylee_client_core::ledge::shortcut_for`] is the bridge from an answer to
/// the action that sends it, and `chords` is the account's own binding of
/// that action. `first` and not `[0]`, because an action a player has unbound
/// is an answer with no cap rather than a panic.
///
/// A [`Says::Command`] names its action here rather than through that bridge,
/// which reaches `PromptAction` alone. That is one line per command and the
/// alternative is worse: `shortcut_for` lives in client-core, where
/// `MenuAction` is a renderer type it does not know and should not learn.
///
/// `held` is the one cap that depends on the *game* and not on the button:
/// the way out of an engine hold wears `F6`, because `F6` is what cancels a
/// running hold, and the way out of the **autopilot** wears nothing, because
/// no key ends that one (§4.4). One button, two mechanisms behind it, and a
/// cap that promised what its key does not do would be worse than no cap.
pub(super) fn keys_for(
    prefs: &crate::prefs::Prefs,
    answers: &[(Says, String)],
    armed: bool,
    held: bool,
) -> Vec<Option<String>> {
    let legend = |action| {
        prefs
            .keymap()
            .chords(action)
            .first()
            .map(baylee_client_core::prefs::Chord::display)
    };
    if armed {
        // Not a `PromptAction` between them: an armed deed is the client's
        // own two-stage commit, fired by `Action::Primary` and taken back by
        // `Action::Cancel` (`input::armed_keys`), so the bridge does not
        // reach it and these two are named directly.
        return vec![
            legend(baylee_client_core::prefs::Action::Primary),
            legend(baylee_client_core::prefs::Action::Cancel),
        ];
    }
    answers
        .iter()
        .map(|(says, _)| match says {
            Says::Answer(action) => baylee_client_core::ledge::shortcut_for(*action),
            // The same key the button's own press takes: `menu_click` and the
            // key handler both go through `Duel::hold_action`, so the cap is
            // the truth about what the button does and not merely about what
            // else would do it.
            Says::Command(super::MenuAction::ToggleGrantedActions) => {
                Some(baylee_client_core::prefs::Action::GrantedActions)
            }
            Says::Command(super::MenuAction::HoldForStack) => {
                Some(baylee_client_core::prefs::Action::HoldForStack)
            }
            // The same key again, and the same reason: `Duel::hold_action`
            // answers `PriorityHold::Always` while a hold is running, so F6
            // and this button send one thing. The autopilot has no key and
            // gets no cap.
            Says::Command(super::MenuAction::ReleaseHold) => {
                held.then_some(baylee_client_core::prefs::Action::HoldForStack)
            }
            Says::Command(_) => None,
        })
        .map(|action| action.and_then(legend))
        .collect()
}
