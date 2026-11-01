//! What a question says: its headline, and the lines that build it.

#[allow(clippy::wildcard_imports)] // the interaction's own vocabulary
use super::*;

/// Whether the roster says the house is answering for that chair.
///
/// Read off the flag rather than from anything the seat is doing, for the
/// reason `SeatRole` gives: a held chair keeps its player's name, life total
/// and hand, so there is nothing else about it to recognise. A client with no
/// roster yet says no and prints the name alone, which is what it said
/// before this existed.
fn held_by_the_house(statics: Option<&GameStatic>, player: PlayerId) -> bool {
    statics.is_some_and(|s| {
        s.seats
            .iter()
            .find(|seat| seat.player == player)
            .is_some_and(|seat| crate::board::SeatRole::of(seat) == crate::board::SeatRole::Away)
    })
}

fn waiting_line(lang: Lang, statics: Option<&GameStatic>, player: PlayerId) -> String {
    let name = seat_name(lang, statics, player);
    let phrase = if held_by_the_house(statics, player) {
        Phrase::WaitingForHeldSeat
    } else {
        Phrase::WaitingForPlayer
    };
    phrase.fill(lang, &[&name])
}

impl Prompt {
    /// What the bar says to a seat that has kept its opening hand while
    /// others are still deciding theirs (#257), or `None`.
    ///
    /// Read off the view because there is no question to read it off: a host
    /// sends each seat only its own, so from its keep until turn 1 this seat
    /// holds none, and the shelf stood empty over a hand that could not be
    /// played. `None` while this seat is still deciding (its own question is
    /// the sentence then) and from turn 1 on, when `deciding` is empty.
    #[must_use]
    pub fn after_keeping(view: &PlayerView) -> Option<Self> {
        let others = view.deciding;
        (!others.is_empty() && !others.contains(view.seat)).then_some(Self::Deciding { others })
    }

    /// A short line for the prompt bar.
    ///
    /// `statics` is the roster, and it is here for the two lines that are
    /// about **somebody else**: the one that says whose answer the table is
    /// waiting for, and the one that says who offered a draw. Both had the
    /// seat and printed it as a number — "Warte auf Platz 1", "ein Remis
    /// wurde angeboten" — which is answerable at a duel by knowing there is
    /// only one other chair, and is not a question at all at a table of four.
    /// [`crate::i18n::seat_name`] numbers a seat the roster does not describe,
    /// so a frame drawn before `GameStatic` arrives says what it used to.
    #[must_use]
    pub fn headline(
        &self,
        lang: Lang,
        turn: Turn,
        statics: Option<&GameStatic>,
        owing: bool,
    ) -> String {
        self.headline_naming(lang, turn, statics, owing, &|_| None)
    }

    /// [`Self::headline`], naming the creatures a question is about where
    /// the line has room for them: the two a share of combat damage passes
    /// between, and the attacker a band forms around. Without their names
    /// both are counts ("creature 1 of 2", "the band"), which is no
    /// question at a table with two Bears on it.
    ///
    /// `name` is the renderer's: what an object on the table is called in
    /// the reader's language, `None` for one it cannot name, which keeps
    /// the count.
    #[must_use]
    pub fn headline_naming(
        &self,
        lang: Lang,
        turn: Turn,
        statics: Option<&GameStatic>,
        owing: bool,
        name: &dyn Fn(ObjectId) -> Option<String>,
    ) -> String {
        self.headline_naming_targets(lang, turn, statics, owing, name, &|_| None)
    }

    /// Names historical retargets only through an exact-incarnation callback.
    #[must_use]
    pub fn headline_naming_targets(
        &self,
        lang: Lang,
        turn: Turn,
        statics: Option<&GameStatic>,
        owing: bool,
        name: &dyn Fn(ObjectId) -> Option<String>,
        target_name: &dyn Fn(baylee_core::ids::DamageSourceRef) -> Option<String>,
    ) -> String {
        let name = |id: ObjectId| name(id).filter(|n| !n.trim().is_empty());
        match self {
            Self::TextReplacement { .. }
            | Self::ChooseManaAbility { .. }
            | Self::ChooseDamageSource { .. }
            | Self::ChooseDamageEffect { .. }
            | Self::AllocatePrevention { .. }
            | Self::ChooseCards {
                reason: ChoicePrompt::CastFaceDown { .. } | ChoicePrompt::CommandCard,
                ..
            } => self.resolution_headline(lang),
            Self::Waiting { on: Some(p) } => waiting_line(lang, statics, *p),
            Self::Waiting { on: None } => Phrase::JustWaiting.text(lang).to_string(),
            // One seat by name, as any other wait; several by count, because
            // eight names do not fit the shelf and the carets already point
            // at each of them.
            Self::Deciding { others } => match others.len() {
                0 => Phrase::JustWaiting.text(lang).to_string(),
                1 => Self::Waiting {
                    on: others.iter().next(),
                }
                .headline_naming(lang, turn, statics, owing, &name),
                n => Phrase::WaitingForPlayers.fill(lang, &[&n.to_string()]),
            },
            Self::Mulligan { taken, free } => {
                if *free {
                    Phrase::MulliganFree.text(lang).to_string()
                } else {
                    Phrase::MulliganTaken.fill(lang, &[&taken.to_string()])
                }
            }
            Self::BottomCards { count } => {
                Phrase::PutOnBottom.fill_count(lang, usize::from(*count), &[])
            }
            // Not "You have priority". A priority window is an invitation, and
            // stating the rules term for it — next to a button labelled OK —
            // read as a modal that had to be dismissed before play could go on.
            //
            // And not "Your move" on an opponent's turn, which is the same
            // invitation and a different sentence: a player reads "Du bist
            // dran" over the Pass button as *it is your turn*, and then reads
            // the phase rail and the seat bars as disagreeing with it.
            //
            // A payment window is the one priority round where neither of
            // those is true. It is deliberately shaped like nothing — mana
            // abilities and no plays — so it arrives here as an ordinary
            // `Priority` and said "Your move" over a board with nothing to
            // move. `owing` is the caller reading `PlayerView::owed`, which
            // exists precisely because this is not derivable: an offer of
            // mana abilities with nothing castable is also every quiet
            // window in the game.
            Self::Priority { .. } if owing => Phrase::PayOrPass.text(lang).to_string(),
            Self::Priority { .. } => priority_line(lang, turn),
            Self::DeclareAttackers => Phrase::DeclareAttackers.text(lang).to_string(),
            Self::DeclareBlockers { .. } => Phrase::DeclareBlockers.text(lang).to_string(),
            Self::Discard { count } => {
                Phrase::DiscardCards.fill_count(lang, usize::from(*count), &[])
            }
            Self::LegendRule => Phrase::LegendRule.text(lang).to_string(),
            // Delve and convoke are not selections of cards or of targets:
            // both are "spend what you have to help pay", and a line saying
            // so is the difference between a question and a puzzle.
            Self::ChooseCards {
                reason: ChoicePrompt::Delve,
                ..
            } => Phrase::DelveToHelpPay.text(lang).to_string(),
            // Crew is not "choose between one and three": the number that
            // decides is the creatures' total power, and the engine refuses
            // an answer short of it, so the line says the total.
            Self::ChooseCards {
                reason: ChoicePrompt::CostCrew { power },
                ..
            } => Phrase::CrewWithPower.fill(lang, &[&power.to_string()]),
            Self::ChooseCards {
                reason: ChoicePrompt::CostSaddle { power },
                ..
            } => Phrase::SaddleWithPower.fill(lang, &[&power.to_string()]),
            // One card type at a time, and the type is the whole question:
            // the menu is that type's cards, and the next question is the
            // next type's.
            Self::ChooseCards {
                reason: ChoicePrompt::OneOfType { card_type },
                ..
            } => Phrase::TakeOneOfType.fill(lang, &[card_type_name(*card_type).text(lang)]),
            // Every other reason is said by the noun that is counted, which is
            // the one place in this sentence where it fits: "Wähle bis zu 2
            // Karten, die nach unten gehen". Without it a tutor, a scry, a
            // put-back and a wish all read "Wähle 1 Karte", and two of those
            // four decide the turn.
            Self::ChooseCards { reason, min, max } => {
                cards_line(lang, *reason, *min, *max, statics, &name)
            }
            Self::ChooseTargets { reason, min, max } => {
                targets_line(lang, reason, *min, *max, statics, target_name)
            }
            // Landwalk choices may offer nonbasic land subtypes too. Keep
            // their question distinct from creature-type choices.
            Self::ChooseSubtype { options }
                if !options.is_empty()
                    && options
                        .iter()
                        .all(|s| baylee_core::types::SubtypeSet::BASIC_LANDS.contains(*s)) =>
            {
                Phrase::ChooseBasicLandType.text(lang).to_string()
            }
            Self::ChooseSubtype { options }
                if !options.is_empty()
                    && options
                        .iter()
                        .all(|s| baylee_core::generated::subtypes::ALL_LAND_TYPES.contains(*s)) =>
            {
                Phrase::ChooseLandType.text(lang).to_string()
            }
            Self::ChooseSubtype { .. } => Phrase::ChooseCreatureType.text(lang).to_string(),
            Self::ChooseCardName { .. } => Phrase::ChooseCardName.text(lang).to_string(),
            Self::ChooseColor { .. } => Phrase::ChooseColour.text(lang).to_string(),
            Self::ChooseNumber { min, max, reason } => {
                number_line(lang, *min, *max, *reason, &name)
            }
            Self::ChoosePlayer { .. } => Phrase::ChoosePlayer.text(lang).to_string(),
            Self::CastMode { options, .. } => cast_choice_headline(options, lang),
            Self::ChoosePile { label: None, .. } => {
                Phrase::ChoosePileForHand.text(lang).to_string()
            }
            // Raging River asks once per attacker, and the question can only
            // be answered when it says which attacker it is about.
            Self::ChoosePile {
                label: Some(attacker),
                ..
            } => name(*attacker).map_or_else(
                || Phrase::ChooseRiverLabel.text(lang).to_string(),
                |attacker| Phrase::ChooseRiverLabelFor.fill(lang, &[&attacker]),
            ),
            Self::Arrange { reason, onto } => arrange_line(lang, *reason, *onto),
            Self::YesNo { question } => yes_no_line(lang, *question, statics),
            Self::GameOver => Phrase::TheGameIsOver.text(lang).to_string(),
        }
    }
}

/// The headline of an arrangement: why the cards are being placed, and for
/// a plain order, where.
fn arrange_line(lang: Lang, reason: ArrangePrompt, onto: Option<ArrangePlace>) -> String {
    let phrase = match (reason, onto) {
        (ArrangePrompt::Order, Some(ArrangePlace::LibraryTop)) => Phrase::OrderOnTop,
        (ArrangePrompt::Order, Some(ArrangePlace::LibraryBottom)) => Phrase::OrderOnBottom,
        (ArrangePrompt::Order, Some(ArrangePlace::Graveyard)) => Phrase::OrderGraveyard,
        (ArrangePrompt::Order, None) => Phrase::PutInOrder,
        (ArrangePrompt::Scry, _) => Phrase::ScryPrompt,
        (ArrangePrompt::Surveil, _) => Phrase::SurveilPrompt,
        (ArrangePrompt::DiscardToLibrary, _) => Phrase::DiscardToLibraryPrompt,
    };
    phrase.text(lang).to_string()
}

/// What a card choice is *for*, as the noun it counts — both forms.
///
/// [`ChoicePrompt`] has twelve variants and the prompt bar used to read one of
/// them. A library search, a scry, a put-back and a wish are four different
/// decisions and were four copies of the same sentence, so a player could not
/// tell whether they were fetching something, burying it or bringing it in
/// from outside. `Delve` is answered a line earlier (it is part of a cost,
/// not a selection) and `Generic` is the plain noun, which is honest: the
/// engine did not say what it was for either.
///
/// These prompts describe what happens to each selected payment object: `CostSacrifice`,
/// `CostDiscard`, `CostTap`, `CostReturn` and `CostExile` arrive while
/// CR 601.2h is being paid, so each
/// gets a noun that says what happens to the card rather than sharing delve's
/// "spend what you have" line. A player who is told only "choose 1 card"
/// while paying for Survival of the Fittest cannot tell the discard from the
/// creature it fetches — and the tap and the return are the two whose card
/// *survives*, which nobody would guess from a sacrifice's wording. Those
/// two are not one noun either: Quirion Ranger's Forest may already be
/// tapped, and a player reading "untapped permanent to tap" over their own
/// lands would look for the wrong one. `CostExile` names its pile for the
/// same kind of reason: delve exiles from the graveyard too, but a player
/// told "card to discard" over Moorland Haunt's menu would look in their
/// hand.
///
/// `LeaveTapped` is the one that is neither: the untap step asking the
/// active player which of their permanents stay tapped (CR 502.3). Its noun
/// says what *not* choosing does, because the empty answer is the whole
/// board untapping and a player shown "permanent to untap" over a menu of
/// one would read the question backwards. `Untap` is the same step under a
/// limit (Smoke, Winter Orb), where the menu is what may untap and at least
/// one must, so its noun says what choosing does.
///
/// `Band` is the attacking player naming the creatures that attack in a
/// band with one that has banding (CR 702.22c). Its noun says what choosing
/// does; naming none attacks without a band.
///
/// `Dredge` and `DrawReplacement` are asked about one card draw at a time
/// (CR 121.2) and are whole sentences in [`cards_line`], as the entry
/// replacements are; their arm here only keeps the match exhaustive. The
/// question a draw can also raise through `Discard` (Chains of Mephistopheles)
/// and `PutOnBottom` (Aladdin's Lamp's top cards, the one left is drawn) is
/// said by those prompts' own nouns, which read true there.
///
/// `RevealOrEnterTapped` is a reveal land asking which card from hand to
/// show. Its noun says what happens to the card and not what declining
/// costs, which is the opposite of `LeaveTapped`'s reasoning and right for
/// the opposite reason: the menu holds only matching cards, so a player who
/// sees "card to reveal" over their two Faeries already knows what the land
/// wanted, and the price of naming nothing is on the land rather than on
/// any of the cards listed.
fn choice_noun(reason: ChoicePrompt) -> Phrase {
    match reason {
        ChoicePrompt::RemoveLandCounters => Phrase::NounLandsToRemoveCounters,
        ChoicePrompt::Keep => Phrase::NounCardsToKeep,
        ChoicePrompt::KeepLands => Phrase::NounLandsToKeep,
        ChoicePrompt::KeepCreatures => Phrase::NounCreaturesToKeep,
        ChoicePrompt::KeepPermanents => Phrase::NounPermanentsToKeep,
        ChoicePrompt::SearchLibrary => Phrase::NounCardsFromLibrary,
        ChoicePrompt::PutBackOnTop => Phrase::NounCardsToTop,
        ChoicePrompt::Wish => Phrase::NounCardsOutside,
        ChoicePrompt::CostSacrifice | ChoicePrompt::SacrificeFor { .. } => {
            Phrase::NounPermanentsToSacrifice
        }
        ChoicePrompt::CostDiscard | ChoicePrompt::Discard => Phrase::NounCardsToDiscard,
        ChoicePrompt::CostTap
        | ChoicePrompt::CostTapExactlyOrNone { .. }
        | ChoicePrompt::CostCrew { .. }
        | ChoicePrompt::CostSaddle { .. } => Phrase::NounPermanentsToTap,
        ChoicePrompt::CostReturn => Phrase::NounPermanentsToReturn,
        ChoicePrompt::CostExile => Phrase::NounCardsToExile,
        ChoicePrompt::LeaveTapped => Phrase::NounPermanentsToLeaveTapped,
        ChoicePrompt::Untap => Phrase::NounPermanentsToUntap,
        ChoicePrompt::Band { .. } => Phrase::NounAttackersToBand,
        ChoicePrompt::BlockWith { .. } => Phrase::NounAttackersToBlock,
        ChoicePrompt::LeftPile => Phrase::NounCreaturesForLeft,
        ChoicePrompt::CamouflagePile { .. } => Phrase::NounCreaturesForPile,
        ChoicePrompt::TappedForCounters { .. } => Phrase::NounTappedForCounters,
        ChoicePrompt::Amass => Phrase::NounArmiesToAmass,
        ChoicePrompt::Proliferate => Phrase::NounProliferatePermanents,
        ChoicePrompt::RevealDiscard => Phrase::NounDiscardsToReveal,
        ChoicePrompt::RevealOrEnterTapped => Phrase::NounCardsToReveal,
        ChoicePrompt::PutIntoHand => Phrase::NounCardsToHand,
        ChoicePrompt::PutOnBottom => Phrase::NounCardsToBottom,
        ChoicePrompt::PlayFromExile => Phrase::NounCardsToPlay,
        ChoicePrompt::PutOntoBattlefield => Phrase::NounCardsToBattlefield,
        ChoicePrompt::PutIntoGraveyard => Phrase::NounCardsToGraveyard,
        ChoicePrompt::FirstPile => Phrase::NounCardsForFirstPile,
        ChoicePrompt::FromGraveyard => Phrase::NounCardsFromGraveyard,
        ChoicePrompt::NewHost { .. } => Phrase::NounNewHostsFor,
        ChoicePrompt::Behold
        | ChoicePrompt::LookAtLibrary
        | ChoicePrompt::LookAtHand
        | ChoicePrompt::Delve
        | ChoicePrompt::OneOfType { .. }
        | ChoicePrompt::CastFaceDown { .. }
        | ChoicePrompt::CommandCard
        | ChoicePrompt::EntryOrder
        | ChoicePrompt::EntryReplacement
        | ChoicePrompt::Dredge
        | ChoicePrompt::DrawReplacement
        | ChoicePrompt::Generic => Phrase::NounCards,
    }
}

/// The line a choice of cards gets, said by the noun that is counted.
///
/// The band is the leader's, and a table of banding creatures is a table of
/// them: that line says which one it forms around, where it can be named.
fn cards_line(
    lang: Lang,
    reason: ChoicePrompt,
    min: u8,
    max: u8,
    statics: Option<&GameStatic>,
    name: &dyn Fn(ObjectId) -> Option<String>,
) -> String {
    // Whole sentences, not counted nouns: the question is which of several
    // rules applies (CR 616.1) or whether a draw is replaced at all
    // (CR 702.52a), and "up to 1 card" says neither.
    let sentence = match reason {
        ChoicePrompt::EntryOrder => Some(Phrase::ChooseEntryOrder),
        ChoicePrompt::EntryReplacement => Some(Phrase::ChooseEntryReplacement),
        ChoicePrompt::Dredge => Some(Phrase::ChooseDredge),
        ChoicePrompt::DrawReplacement => Some(Phrase::ChooseDrawReplacement),
        _ => None,
    };
    if let Some(phrase) = sentence {
        return phrase.text(lang).to_owned();
    }
    if let ChoicePrompt::CostTapExactlyOrNone { count } = reason {
        return Phrase::ChooseExactlyOrNoTaps.fill(lang, &[&count.to_string()]);
    }
    if reason == ChoicePrompt::LookAtLibrary {
        return Phrase::InspectLibrary.text(lang).to_owned();
    }
    if reason == ChoicePrompt::Behold {
        return Phrase::BeholdInstructions.text(lang).to_owned();
    }
    if reason == ChoicePrompt::LookAtHand {
        return Phrase::InspectHand.text(lang).to_owned();
    }
    if let ChoicePrompt::Band { with } = reason
        && let Some(leader) = name(with)
    {
        let noun = Phrase::NounAttackersToBandWith.fill_count(lang, usize::from(max), &[&leader]);
        return choose_line_of(lang, &noun, u32::from(min), u32::from(max));
    }
    if let ChoicePrompt::CamouflagePile { pile, of } = reason {
        let noun = Phrase::NounCreaturesForPileOf.fill_count(
            lang,
            usize::from(max),
            &[&pile.to_string(), &of.to_string()],
        );
        return choose_line_of(lang, &noun, u32::from(min), u32::from(max));
    }
    // An Aura's new host names the Aura (09.10.: Kudzu asked for "up to 1
    // card").
    if let ChoicePrompt::NewHost { aura } = reason {
        let aura = name(aura).unwrap_or_else(|| Phrase::NounTheAura.text(lang).to_owned());
        let noun = Phrase::NounNewHostsFor.fill_count(lang, usize::from(max), &[&aura]);
        return choose_line_of(lang, &noun, u32::from(min), u32::from(max));
    }
    if let ChoicePrompt::BlockWith { blocker } = reason
        && let Some(blocker) = name(blocker)
    {
        let noun = Phrase::NounAttackersForBlocker.fill_count(lang, usize::from(max), &[&blocker]);
        return choose_line_of(lang, &noun, u32::from(min), u32::from(max));
    }
    let selection = choose_line(lang, choice_noun(reason), u32::from(min), u32::from(max));
    if let ChoicePrompt::SacrificeFor { player } = reason {
        Phrase::SacrificeForPlayer.fill(lang, &[&seat_name(lang, statics, player), &selection])
    } else {
        selection
    }
}

/// Payment instructions distinguish a fixed debt from an optional amount.
#[must_use]
pub fn payment_line(lang: Lang, payment: baylee_core::mana::ManaPayment) -> String {
    match payment {
        baylee_core::mana::ManaPayment::Fixed(_) => Phrase::PayOrPass.text(lang).to_owned(),
        baylee_core::mana::ManaPayment::AnyAmount { preventable_damage } => {
            Phrase::PrepareManaPayment.fill(lang, &[&preventable_damage.to_string()])
        }
    }
}

/// A target question, including which existing target a retarget may replace.
fn targets_line(
    lang: Lang,
    reason: &TargetPrompt,
    min: u32,
    max: u32,
    statics: Option<&GameStatic>,
    name: &dyn Fn(baylee_core::ids::DamageSourceRef) -> Option<String>,
) -> String {
    match reason {
        TargetPrompt::Convoke => Phrase::TapToHelpPay.text(lang).to_string(),
        // Entering as a copy chooses; it does not target.
        TargetPrompt::CopyOf => choose_line(lang, Phrase::NounPermanentsToCopy, min, max),
        TargetPrompt::Targets => choose_line(lang, Phrase::NounTargets, min, max),
        TargetPrompt::DistinctManaValues(_) => Phrase::DifferentManaValues.text(lang).to_string(),
        TargetPrompt::Retarget { current, index, of } => {
            let target = match current {
                baylee_engine::choice::TargetRef::Object(id) => {
                    name(*id).unwrap_or_else(|| Phrase::PreviousTarget.text(lang).to_string())
                }
                baylee_engine::choice::TargetRef::Player(id) => seat_name(lang, statics, *id),
            };
            let question = if min == 0 {
                Phrase::ChooseNewTargetOrKeep
            } else {
                Phrase::ChooseNewTarget
            };
            Phrase::RetargetContext.fill(
                lang,
                &[
                    &(u64::from(*index) + 1).to_string(),
                    &of.to_string(),
                    &target,
                    question.text(lang),
                ],
            )
        }
    }
}

/// The line a number question gets: the range to choose from, or, for one
/// target's share of a division, that target by its place in the order the
/// player chose them (the order the stack shows) and what is still to give.
fn number_line(
    lang: Lang,
    min: u32,
    max: u32,
    reason: NumberPrompt,
    name: &dyn Fn(ObjectId) -> Option<String>,
) -> String {
    let (min, max) = (min.to_string(), max.to_string());
    // Which creature's damage, and which creature this share is for: the
    // attacker's controller divides it among its blockers (CR 510.1c), and
    // "creature 1 of 2" names neither.
    if let NumberPrompt::CombatDamage {
        source,
        recipient,
        index,
        of,
        left,
    } = reason
        && let (Some(source), Some(recipient)) = (name(source), name(recipient))
    {
        return Phrase::CombatDamageShareNamed.fill(
            lang,
            &[
                &source,
                &recipient,
                &(u64::from(index) + 1).to_string(),
                &of.to_string(),
                &left.to_string(),
                &min,
                &max,
            ],
        );
    }
    match reason {
        NumberPrompt::Sacrifices => Phrase::ChooseSacrificeCount.fill(lang, &[&min, &max]),
        NumberPrompt::EnergyPayment { candidates } => {
            Phrase::ChooseEnergyPayment.fill(lang, &[&min, &max, &candidates.to_string()])
        }
        NumberPrompt::TextReplacement { .. } => {
            Phrase::ChooseTextReplacement.text(lang).to_string()
        }
        NumberPrompt::Counters { target, kind } => {
            let label = counter_label(kind, lang);
            name(target).map_or_else(
                || Phrase::ChooseCounters.fill(lang, &[&label, &min, &max]),
                |target| Phrase::ChooseCountersNamed.fill(lang, &[&label, &target, &min, &max]),
            )
        }
        NumberPrompt::X | NumberPrompt::Choice => Phrase::ChooseNumberIn.fill(lang, &[&min, &max]),
        NumberPrompt::ManaPayment { preventable_damage } => {
            Phrase::ChooseManaPayment.fill(lang, &[&min, &max, &preventable_damage.to_string()])
        }
        NumberPrompt::Replicate { cost } => {
            Phrase::ReplicateHowOften.fill(lang, &[&cost.to_string(), &min, &max])
        }
        NumberPrompt::CombatDamage {
            index, of, left, ..
        } => Phrase::CombatDamageShare.fill(
            lang,
            &[
                &(u64::from(index) + 1).to_string(),
                &of.to_string(),
                &left.to_string(),
                &min,
                &max,
            ],
        ),
        NumberPrompt::DivideDamage {
            index, of, left, ..
        } => Phrase::DamageShare.fill(
            lang,
            &[
                &(u64::from(index) + 1).to_string(),
                &of.to_string(),
                &left.to_string(),
                &min,
                &max,
            ],
        ),
    }
}

/// Localized plural name for counters offered by an engine choice.
#[must_use]
pub fn counter_label(kind: baylee_engine::object::CounterKind, lang: Lang) -> String {
    counters_named(kind, lang, None)
}

/// As [`counter_label`], in the form `n` counters take ("1 +1/+1 counter").
#[must_use]
pub fn counter_label_counted(
    kind: baylee_engine::object::CounterKind,
    lang: Lang,
    n: usize,
) -> String {
    counters_named(kind, lang, Some(n))
}

/// A counter kind's name; with no count, the plural.
fn counters_named(
    kind: baylee_engine::object::CounterKind,
    lang: Lang,
    n: Option<usize>,
) -> String {
    use baylee_engine::object::CounterKind;
    let say = |phrase: Phrase, args: &[&str]| match n {
        Some(n) => phrase.fill_count(lang, n, args),
        None => phrase.fill(lang, args),
    };
    match kind {
        CounterKind::Plus { power, toughness } => say(
            Phrase::LogCountersPlus,
            &[&power.to_string(), &toughness.to_string()],
        ),
        CounterKind::Minus { power, toughness } => say(
            Phrase::LogCountersMinus,
            &[&power.to_string(), &toughness.to_string()],
        ),
        CounterKind::Loyalty => say(Phrase::LogCountersLoyalty, &[]),
        CounterKind::Lore => say(Phrase::LogCountersLore, &[]),
        CounterKind::Bore => say(Phrase::LogCountersBore, &[]),
        CounterKind::Shield => say(Phrase::LogCountersShield, &[]),
        CounterKind::Defense => say(Phrase::LogCountersDefense, &[]),
        CounterKind::Time => say(Phrase::LogCountersTime, &[]),
        CounterKind::Charge => say(Phrase::LogCountersCharge, &[]),
        CounterKind::Poison => say(Phrase::LogCountersPoison, &[]),
        CounterKind::Energy => say(Phrase::LogCountersEnergy, &[]),
        CounterKind::Rad => say(Phrase::LogCountersRad, &[]),
        CounterKind::Lifelink => say(Phrase::LogCountersLifelink, &[]),
        CounterKind::Level => say(Phrase::LogCountersLevel, &[]),
        CounterKind::Hone => say(Phrase::LogCountersHone, &[]),
        CounterKind::Stun => say(Phrase::LogCountersStun, &[]),
        CounterKind::Finality => say(Phrase::LogCountersFinality, &[]),
        CounterKind::Flying => say(Phrase::LogCountersFlying, &[]),
        CounterKind::Shadow => say(Phrase::LogCountersShadow, &[]),
        CounterKind::Indestructible => say(Phrase::LogCountersIndestructible, &[]),
        CounterKind::Custom(id) => {
            let noun = say(Phrase::LogCountersOther, &[]);
            baylee_cards_dsl::counters::ASSIGNED
                .iter()
                .find(|(_, assigned)| *assigned == kind)
                .map_or_else(
                    || format!("{noun} ({id})"),
                    |(name, _)| format!("{name} {noun}"),
                )
        }
    }
}

/// The name of one card type, for a question asked about it.
fn card_type_name(card_type: baylee_core::types::TypeSet) -> Phrase {
    use baylee_core::types::TypeSet;
    [
        (TypeSet::ARTIFACT, Phrase::KindArtifact),
        (TypeSet::BATTLE, Phrase::KindBattle),
        (TypeSet::CREATURE, Phrase::KindCreature),
        (TypeSet::ENCHANTMENT, Phrase::KindEnchantment),
        (TypeSet::INSTANT, Phrase::KindInstant),
        (TypeSet::LAND, Phrase::KindLand),
        (TypeSet::PLANESWALKER, Phrase::KindPlaneswalker),
        (TypeSet::SORCERY, Phrase::KindSorcery),
    ]
    .into_iter()
    .find_map(|(t, name)| card_type.contains(t).then_some(name))
    .unwrap_or(Phrase::KindOther)
}

/// "Choose two cards", with the noun as an argument rather than glued on.
///
/// A count and a noun agree differently in different languages, so the whole
/// sentence has to be one phrase — pasting a translated noun onto a
/// translated "choose up to" is how a translation ends up ungrammatical.
///
/// The noun is a counted phrase and **`max` picks its form**, because
/// `max` is the number the noun stands next to in all three frames: "up to
/// 2 cards", "1 card", "1–3 cards". A range whose top is more than one is
/// plural however low it starts.
fn choose_line(lang: Lang, noun: Phrase, min: u32, max: u32) -> String {
    choose_line_of(
        lang,
        &noun.fill_count(lang, usize::try_from(max).unwrap_or(usize::MAX), &[]),
        min,
        max,
    )
}

/// [`choose_line`] with the counted noun already written.
fn choose_line_of(lang: Lang, noun: &str, min: u32, max: u32) -> String {
    match (min, max) {
        (0, m) => Phrase::ChooseUpTo.fill(lang, &[&m.to_string(), noun]),
        (a, b) if a == b => Phrase::ChooseExactly.fill(lang, &[&a.to_string(), noun]),
        (a, b) => Phrase::ChooseBetween.fill(lang, &[&a.to_string(), &b.to_string(), noun]),
    }
}

/// The roster is taken for the one question that is about another player.
fn yes_no_line(lang: Lang, question: YesNoPrompt, statics: Option<&GameStatic>) -> String {
    match question {
        YesNoPrompt::SacrificeToEnter { count } => {
            Phrase::SacrificeToEnter.fill(lang, &[&count.to_string()])
        }
        YesNoPrompt::EnterTappedForCounters { kind, amount } => Phrase::EnterTappedForCounters
            .fill(
                lang,
                &[
                    &amount.to_string(),
                    &counter_label_counted(kind, lang, usize::from(amount)),
                ],
            ),
        YesNoPrompt::PayLifeOrEnterTapped { amount } => {
            Phrase::PayLifeOrTapped.fill(lang, &[&amount.to_string()])
        }
        YesNoPrompt::PayEnergy { amount } => Phrase::PayEnergy.fill(lang, &[&amount.to_string()]),
        YesNoPrompt::ProliferatePlayer { player } => {
            Phrase::ProliferatePlayer.fill(lang, &[&seat_name(lang, statics, player)])
        }
        YesNoPrompt::ManaAnyType => Phrase::UseManaAnyType.text(lang).to_string(),
        YesNoPrompt::Kicker => Phrase::PayAdditionalCost.text(lang).to_string(),
        YesNoPrompt::PayLife { amount } => Phrase::PayLife.fill(lang, &[&amount.to_string()]),
        YesNoPrompt::PayTax { mana } => Phrase::PayTax.fill(lang, &[&mana.to_string()]),
        YesNoPrompt::PayMana { cost } => Phrase::PayMana.fill(lang, &[&cost.to_string()]),
        YesNoPrompt::PayManaAndLife { cost, life } => {
            Phrase::PayManaAndLife.fill(lang, &[&cost.to_string(), &life.to_string()])
        }
        YesNoPrompt::PayPact { cost } => Phrase::PayPact.fill(lang, &[&cost.to_string()]),
        YesNoPrompt::Miracle { .. } => Phrase::CastForMiracle.text(lang).to_string(),
        YesNoPrompt::CastWithoutPaying { .. } => Phrase::CastWithoutPaying.text(lang).to_string(),
        YesNoPrompt::CastPaying { .. } => Phrase::CastPaying.text(lang).to_string(),
        // The `..` here is what the whole repair was: `proposer` travels with
        // the question and was dropped one line short of the sentence.
        YesNoPrompt::DrawOffer { proposer } => {
            Phrase::DrawOfferedBy.fill(lang, &[&seat_name(lang, statics, proposer)])
        }
        YesNoPrompt::CommanderZone { .. } => Phrase::CommanderToCommandZone.text(lang).to_string(),
        // The destination is the whole of the decision (CR 903.8 taxes only
        // the command zone), so the line has to name it rather than ask the
        // same question twice.
        YesNoPrompt::CommanderReplace { to_library, .. } => if to_library {
            Phrase::CommanderInsteadOfLibrary
        } else {
            Phrase::CommanderInsteadOfHand
        }
        .text(lang)
        .to_string(),
        // The line says *that* it is optional and not what the clause does:
        // the ability's own text belongs on the stack entry beside it, and
        // a second rendering of it here would be a translation of a
        // translation.
        YesNoPrompt::UseRegenerationShield { .. } => {
            Phrase::UseRegenerationShield.text(lang).to_string()
        }
        YesNoPrompt::UseShieldCounter { .. } => Phrase::UseShieldCounter.text(lang).to_string(),
        YesNoPrompt::UseDamageClearingReplacement { .. } => {
            Phrase::UseDamageClearingReplacement.text(lang).to_string()
        }
        YesNoPrompt::RemoveCountersInsteadOfUntapping { kind, count, .. } => {
            Phrase::RemoveCountersInsteadOfUntapping.fill(
                lang,
                &[
                    &count.to_string(),
                    &counter_label_counted(kind, lang, usize::from(count)),
                ],
            )
        }
        YesNoPrompt::ApplyAdditionalCountersNext { additional, .. } => {
            Phrase::ApplyAdditionalCountersNext.fill(lang, &[&additional.to_string()])
        }
        YesNoPrompt::MayDo => Phrase::UseTheOptionalAbility.text(lang).to_string(),
        // The owner answers, about a card that may be somebody else's
        // spell's target: the answer names both ends, so "no" is not read
        // as "leave it where it is".
        YesNoPrompt::TopOfLibrary { .. } => Phrase::TopOfLibraryOrBottom.text(lang).to_string(),
        // "No" is not "nothing": the card still leaves exile, for the hand.
        YesNoPrompt::Discover { .. } => Phrase::CastDiscovered.text(lang).to_string(),
        YesNoPrompt::SkipTurn { .. } => Phrase::SkipTurnToUntap.text(lang).to_string(),
        YesNoPrompt::Generic => Phrase::YesOrNo.text(lang).to_string(),
    }
}

fn priority_line(lang: Lang, turn: Turn) -> String {
    match turn {
        Turn::Mine => Phrase::YourMove.text(lang).to_string(),
        Turn::Theirs => Phrase::YouMayRespond.text(lang).to_string(),
    }
}
