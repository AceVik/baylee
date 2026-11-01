//! A mana run: the taps of a plan, sent one by one, and then the cast or
//! activation they pay for; with the plans a click works out.

#[allow(clippy::wildcard_imports)] // the crate root's own vocabulary
use super::*;

/// Tapping permanents for mana, one action at a time.
///
/// The plan is decided in one go (`manaplan::plan`) and then spent one step
/// per engine round trip, because that is how the engine works: every
/// activation is an action, and each one comes back as a fresh `Pending` with
/// a fresh `LegalActions`. That round trip is also the safety property — each
/// step is re-checked against what the engine is offering *now*, so a plan
/// that has gone stale stops instead of guessing.
///
/// Usually the mana is *for* something and [`RunEnd`] says what. It is not
/// always: [`RunEnd::Float`] is a run of one tap made because the player
/// asked for that mana and nothing else, which is what a mana bubble sends.
#[derive(Debug)]
pub struct ManaRun {
    /// Taps still to make.
    pub(super) steps: std::collections::VecDeque<baylee_client_core::manaplan::Step>,
    /// The colour to answer with while an ability is asking for one.
    asking: Option<baylee_core::mana::ManaColor>,
    /// The card all of this is for — the spell being paid for, or, under
    /// [`RunEnd::Float`], the permanent whose own mana was asked for.
    card: ObjectId,
    /// What the mana is spent on once every tap is made.
    then: RunEnd,
    /// Set for a cast made before its mana (CR 601.2g): the card was
    /// `LegalActions::payable`, so the cast is sent first, its own questions
    /// are answered as they come, and the plan is worked out against the
    /// payment window it opens (`PlayerView::owed`). Its mana's triggers
    /// then wait until the spell is cast (CR 601.2i). `None` for a run that
    /// taps first.
    cast_first: Option<CastFirst>,
}

/// Where a cast-first run stands.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum CastFirst {
    /// The cast is still to be sent.
    Unsent,
    /// The cast was sent; the run waits for its payment window.
    Sent,
}

impl ManaRun {
    /// Starts a run for `card`.
    #[must_use]
    pub fn new(plan: baylee_client_core::manaplan::Plan, card: ObjectId, then: RunEnd) -> Self {
        Self {
            steps: plan.steps.into(),
            asking: None,
            card,
            then,
            cast_first: None,
        }
    }

    /// Starts a run that casts `card` first and pays for it in the window
    /// the cast opens (CR 601.2g), for a card in `LegalActions::payable`.
    #[must_use]
    pub fn cast_first(card: ObjectId) -> Self {
        Self {
            steps: std::collections::VecDeque::new(),
            asking: None,
            card,
            then: RunEnd::Cast,
            cast_first: Some(CastFirst::Unsent),
        }
    }

    /// The action a completed plan is paying for.
    #[cfg(test)]
    #[must_use]
    pub(crate) const fn then(&self) -> RunEnd {
        self.then
    }

    /// The spell being paid for — the HUD says so while it happens.
    #[must_use]
    pub const fn card(&self) -> ObjectId {
        self.card
    }

    /// The spell a cast-first run has sent and is answering the questions
    /// of before it pays (CR 601.2c–g): the decision sheet says the cast and
    /// the question in one sentence, and that the mana comes after.
    #[must_use]
    pub fn casting_first(&self) -> Option<ObjectId> {
        (self.cast_first == Some(CastFirst::Sent)).then_some(self.card)
    }

    /// The same run once its cast has been sent, for a test that stands in
    /// that moment.
    #[cfg(test)]
    #[must_use]
    pub(crate) const fn sent(mut self) -> Self {
        self.cast_first = Some(CastFirst::Sent);
        self
    }
}

/// Spends a mana plan, one action per frame the engine asks us something.
///
/// Aborting is a first-class outcome and not an error path: anything the
/// engine offers that is not the next step of the plan ends the run and hands
/// the player back their turn, with whatever was already tapped left tapped.
/// That is the honest failure — mana in the pool is a thing the player can
/// see and spend — and it is much better than the alternative of pushing an
/// action the engine will refuse.
///
/// Asked before the duel is borrowed: `&mut duel` through the `ResMut` is a
/// write, and with no run under way this would mark the duel changed on
/// every frame.
pub(super) fn run_mana_plan(mut duel: ResMut<Duel>) {
    if duel.mana_run.is_some() {
        advance_mana_run(&mut duel);
    }
}

/// One step of a run, against whatever the engine is asking right now.
///
/// The system above is the wiring; this is the decision, and it is `pub` so a
/// test can drive a whole run against a real `LocalHost` the way the frame
/// loop does — press, send, take the answer, press again. A run asserted on
/// through its `ManaRun` alone would prove the plan and not the spending, and
/// spending is where every one of this function's failure modes lives.
pub fn advance_mana_run(duel: &mut Duel) {
    if duel.mana_run.is_none() {
        return;
    }
    // Between sending and the next snapshot there is nothing to decide; the
    // run is not stale, it is simply waiting.
    let Some(interaction) = duel.interaction.as_ref() else {
        return;
    };
    let seat = duel.seat().unwrap_or(PlayerId::new(0));
    let pending = interaction.pending().clone();
    if let Some(phase) = duel.mana_run.as_ref().and_then(|r| r.cast_first)
        && advance_cast_first(duel, phase, &pending, seat)
    {
        return;
    }
    let mut action = None;
    let mut finished = false;
    let mut abort = None;

    match &pending {
        Pending::ChooseColor { player, options } if *player == seat => {
            let asked = duel.mana_run.as_ref().and_then(|r| r.asking);
            match asked.filter(|c| options.contains(c)) {
                Some(color) => {
                    action = Some(PlayerAction::ChooseColor(color));
                    if let Some(run) = duel.mana_run.as_mut() {
                        run.asking = None;
                    }
                }
                None => abort = Some(Phrase::PlanColourGone),
            }
        }
        Pending::Priority { player, legal } if *player == seat => {
            let step = duel.mana_run.as_mut().and_then(|r| r.steps.pop_front());
            if let Some(step) = step {
                action = tap_action(&step, legal);
                if action.is_none() {
                    abort = Some(Phrase::PlanLandGone);
                } else if let Some(run) = duel.mana_run.as_mut() {
                    run.asking = step.color;
                }
            } else {
                // Every tap is made; the mana is floating and the engine is
                // offering the thing it was floated for. Which list to look
                // in was decided when the run was armed, not here — a card
                // that is castable *and* suspendable is two different deeds
                // and neither is undoable.
                let run = duel.mana_run.as_ref().map(|r| (r.card, r.then));
                match run {
                    Some((source, RunEnd::Ability(ability_index))) => {
                        if legal.abilities.contains(&(source, ability_index)) {
                            action = Some(PlayerAction::ActivateAbility {
                                source,
                                ability_index,
                            });
                        } else {
                            abort = Some(Phrase::DeedWithdrawn);
                        }
                    }
                    Some((card, RunEnd::Cast)) if legal.castable.contains(&card) => {
                        action = Some(PlayerAction::CastSpell { card });
                    }
                    Some((card, RunEnd::Suspend)) if legal.suspendable.contains(&card) => {
                        action = Some(PlayerAction::Suspend { card });
                    }
                    Some((card, RunEnd::Foretell)) if legal.foretellable.contains(&card) => {
                        action = Some(PlayerAction::Foretell { card });
                    }
                    // Nothing is owed at the end of a pour, and there is
                    // nothing to check either: the mana is in the pool, which
                    // is a thing the player can see and spend.
                    Some((_, RunEnd::Float)) => {}
                    Some((_, RunEnd::Cast)) => {
                        abort = Some(Phrase::PlanSpellRefused);
                    }
                    Some((_, RunEnd::Suspend)) => {
                        abort = Some(Phrase::PlanSuspendRefused);
                    }
                    Some((_, RunEnd::Foretell)) => {
                        abort = Some(Phrase::PlanForetellRefused);
                    }
                    None => abort = Some(Phrase::PlanCardGone),
                }
                finished = true;
            }
        }
        // Mana abilities do not use the stack, so priority never leaves the
        // seat in the middle of a plan. Anything else means the game moved on
        // without us and the plan is void.
        _ => abort = Some(Phrase::PlanQuestionChanged),
    }

    if let Some(reason) = abort {
        duel.last_error = Some(Refusal::Said(reason));
        duel.mana_run = None;
        return;
    }
    if finished {
        duel.mana_run = None;
    }
    if let Some(action) = action {
        duel.submit(action);
    }
}

/// The cast-first half of a run (CR 601.2g); returns whether it handled this
/// question.
///
/// The cast goes out on this seat's priority. Every question after it that
/// is not the window is the cast's own (its mode, X, targets) and is the
/// player's to answer: the run waits. When the window opens, the plan is
/// worked out the way "Pay now" works it out ([`Duel::compute_owed_plan`]),
/// against the pool as it stands, staged, and sent as one
/// `PlayerAction::PayWindow`: the click that confirmed the armed card was
/// the Go, and its lit lands were the staging. A plan the engine refuses
/// comes back as one sentence with the window and the staging standing,
/// and a window nothing plans stays open for the player — never passed on
/// their behalf, which would reverse the cast in silence. A priority that is
/// not a window means the cast is over: on the stack, or reversed.
fn advance_cast_first(
    duel: &mut Duel,
    phase: CastFirst,
    pending: &Pending,
    seat: PlayerId,
) -> bool {
    let Some(card) = duel.mana_run.as_ref().map(|r| r.card) else {
        return true;
    };
    let Pending::Priority { player, legal } = pending else {
        // The cast's own questions, answered by the player (or the cast
        // chooser's answer, `take_the_chosen_cast_mode`).
        if phase == CastFirst::Unsent {
            duel.last_error = Some(Refusal::Said(Phrase::PlanQuestionChanged));
            duel.mana_run = None;
        }
        return true;
    };
    if *player != seat {
        return true;
    }
    if phase == CastFirst::Unsent {
        if legal.castable.contains(&card) || legal.payable.contains(&card) {
            if let Some(run) = duel.mana_run.as_mut() {
                run.cast_first = Some(CastFirst::Sent);
            }
            duel.submit(PlayerAction::CastSpell { card });
        } else {
            duel.last_error = Some(Refusal::Said(Phrase::DeedWithdrawn));
            duel.mana_run = None;
        }
        return true;
    }
    if !duel.paying() {
        let cast = duel
            .view
            .as_ref()
            .is_some_and(|v| v.stack.iter().any(|o| o.id == card));
        duel.mana_run = None;
        if !cast {
            duel.last_error = Some(Refusal::Said(Phrase::PlanSpellRefused));
        }
        return true;
    }
    duel.mana_run = None;
    duel.refresh_owed_plan();
    if !duel.staged_covers() && !duel.stage_owed_plan() {
        duel.last_error = Some(Refusal::Said(Phrase::CardCostsUnavailable));
        return true;
    }
    duel.send_payment();
    true
}

use baylee_client_core::paystage::tap_action;

/// Answers the engine's `ChooseCastMode` with the way the player already
/// chose, when they chose one.
///
/// Asked before the duel is borrowed, as [`run_mana_plan`] is.
pub(super) fn answer_the_chosen_cast_mode(mut duel: ResMut<Duel>) {
    if duel.cast_answer.is_some() {
        take_the_chosen_cast_mode(&mut duel);
    }
}

/// The decision behind that system, `pub` for the same reason
/// [`advance_mana_run`] is: a test drives the whole cast through the same
/// door the frame loop does.
///
/// It is the far end of [`CastMenu`]. The player answered this client's
/// question before any mana was floated; the engine asks its own once the
/// mana is up, and this is where the two are joined. The match is on
/// [`baylee_engine::choice::CastModeKind`] and never on an index, because the
/// engine's indices are positions in a list it builds against the pool at the
/// instant it asks — and the run that just finished is what changed that pool.
///
/// A way the engine does not offer is reported rather than substituted. It is
/// reachable: the pitch card this client counted can have left the hand
/// between the chooser and the cast (a Force of Will countering the very
/// spell), and casting the *other* way instead would be the silence this
/// whole repair exists to end.
pub fn take_the_chosen_cast_mode(duel: &mut Duel) {
    let Some((card, kind)) = duel.cast_answer else {
        return;
    };
    let seat = duel.seat().unwrap_or(PlayerId::new(0));
    let Some(interaction) = duel.interaction.as_ref() else {
        return;
    };
    let Pending::ChooseCastMode {
        player,
        object,
        options,
        ..
    } = interaction.pending()
    else {
        return;
    };
    if *player != seat || *object != card {
        return;
    }
    let at = options.iter().position(|option| option.kind == kind);
    duel.cast_answer = None;
    let Some(at) = at else {
        duel.last_error = Some(Refusal::Said(Phrase::CastModeWithdrawn));
        return;
    };
    // Through `choose_index` and `confirm` rather than building the action
    // here, so this answers the question the same way a press on the row
    // would — one door, and a client that cannot express an answer fails
    // rather than sending one nothing on screen could have produced.
    let action = duel
        .interaction
        .as_mut()
        .and_then(|i| i.choose_index(at).then(|| i.confirm())?);
    if let Some(action) = action {
        duel.submit(action);
    }
}

/// The taps that would make `card` castable, if any.
///
/// `None` both when the spell needs no help and when nothing here can pay for
/// it — the caller has already asked the engine the first question.
#[must_use]
pub fn mana_for(duel: &Duel, card: ObjectId) -> Option<baylee_client_core::manaplan::Plan> {
    let view = duel.view.as_ref()?;
    let legal = duel.interaction.as_ref()?.legal_actions()?;
    // A hand card, a card in the seat's own graveyard it may flash back, or a
    // commander standing in the command zone. The three are the only places
    // this client offers to tap lands *for*, and they have to be the same
    // three [`reachable`] admits — a card in one set and not the other is a
    // card that lights up and then does nothing when it is clicked. The
    // graveyard's price is the flashback cost, for the reason given there.
    if let Some(hand_card) = baylee_client_core::decision::hand(view)
        .iter()
        .find(|c| c.id == card)
        && baylee_cards::by_index(hand_card.card.index)
            .is_some_and(|def| def.faces[0].kicked_targets.is_some())
    {
        return castmodes::reachable_modes(view, legal, card)
            .first()
            .map(|mode| mode.plan.clone());
    }
    let cost = reach::least_x(spell_cost(view, legal, card)?);
    let seat = view.seat(baylee_client_core::decision::resource_player(view))?;
    if legal.castable.contains(&card) {
        manasources::plan_for_spell(&cost, &seat.mana_pool, view, legal, card)
    } else {
        manasources::plan_with_life_for_spell(&cost, &seat.mana_pool, view, legal, card, seat.life)
    }
}

/// What casting `card` the ordinary way costs this seat: a hand card's
/// printed cost, a graveyard card's flashback cost, an exiled card's printed
/// cost, or a commander's cost plus its tax, each with what effects add to
/// the spell.
pub(super) fn spell_cost(
    view: &PlayerView,
    legal: &baylee_engine::choice::LegalActions,
    card: ObjectId,
) -> Option<baylee_core::mana::ManaCost> {
    let cost = if let Some(hand_card) = baylee_client_core::decision::hand(view)
        .iter()
        .find(|c| c.id == card)
    {
        manasources::hand_cost(hand_card)?
    } else if let Some(buried) = view
        .graveyards
        .get(view.seat.get() as usize)
        .and_then(|zone| zone.iter().find(|o| o.id == card))
    {
        buried.flashback?
    } else if let Some(exiled) = view.exile.iter().flatten().find(|o| o.id == card) {
        reach::printed_face(exiled.card?)?.mana_cost
    } else {
        let commander = view
            .seat(view.seat)?
            .commanders
            .iter()
            .find(|c| c.object == card)?;
        commander_cost(commander)?.with_more_generic(2 * commander.casts)
    };
    Some(
        cost.with_more_generic(
            legal.spell_increase(card, baylee_engine::choice::CastModeKind::Normal),
        ),
    )
}

/// The taps that would make `card` suspendable, if any.
///
/// [`mana_for`] for the suspend cost. Separate rather than a flag on it,
/// because the two costs are different numbers on the same card — Ancestral
/// Vision prints no mana cost at all and suspends for `{U}` — and a caller
/// that took the wrong one would tap the wrong lands.
#[must_use]
pub fn suspend_mana_for(duel: &Duel, card: ObjectId) -> Option<baylee_client_core::manaplan::Plan> {
    let view = duel.view.as_ref()?;
    let legal = duel.interaction.as_ref()?.legal_actions()?;
    let hand_card = baylee_client_core::decision::hand(view)
        .iter()
        .find(|c| c.id == card)?;
    let cost = suspend_cost(hand_card.card)?;
    let pool = view
        .seat(baylee_client_core::decision::resource_player(view))?
        .mana_pool;
    baylee_client_core::manaplan::plan(&cost, &pool, &manasources::sources(view, legal))
}

/// The taps for foretell's fixed {2} special action, never its later cast cost.
#[must_use]
pub fn foretell_mana_for(
    duel: &Duel,
    card: ObjectId,
) -> Option<baylee_client_core::manaplan::Plan> {
    let view = duel.view.as_ref()?;
    let legal = duel.interaction.as_ref()?.legal_actions()?;
    let hand_card = baylee_client_core::decision::hand(view)
        .iter()
        .find(|c| c.id == card)?;
    let def = baylee_cards::by_index(hand_card.card.index)?;
    let face = def
        .faces
        .get(usize::from(hand_card.card.face))
        .or_else(|| def.faces.first())?;
    face.foretell?;
    let pool = view
        .seat(baylee_client_core::decision::resource_player(view))?
        .mana_pool;
    baylee_client_core::manaplan::plan(
        &baylee_core::mana::ManaCost::parse("{2}"),
        &pool,
        &manasources::sources(view, legal),
    )
}
