//! What a permanent can do, and what to call it.
//!
//! The client could not activate an ability at all before this: clicking a
//! permanent selected it for whatever choice was pending, and a Forest, a
//! mana dork and a planeswalker were all equally inert. `Interaction::activate`
//! existed and nothing called it.
//!
//! What is here is the list, in a stable order, built only from what the
//! engine offered or marked payable after making mana — and a label for each,
//! which is the part that needs the
//! card registry and is therefore the reason this is not in
//! `baylee-client-core`. "Ability 2" is a label a player has to guess at;
//! "Tap for {G}" and "+1" are not.

use baylee_client_core::card_face::TextBlock;
use baylee_client_core::i18n::{Lang, Phrase};
use baylee_client_core::interaction::Interaction;
use baylee_client_core::manapip;
use baylee_client_core::manaplan::Tap;
use baylee_core::ids::ObjectId;
use baylee_core::mana::ManaColor;
use baylee_engine::choice::PlayerAction;
use baylee_view::PlayerView;

use baylee_cards_dsl::{AbilityDef, Cost, CostPart};
use baylee_core::mana::ManaCost;

/// One thing a permanent is offering to do.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct AbilityOption {
    /// The eventual action. An unpaid offer needs a mana run first; only
    /// membership in `LegalActions::abilities` permits sending it now.
    pub action: PlayerAction,
    /// What the button says.
    pub label: String,
    /// Whether this is a mana ability (CR 605.1).
    ///
    /// The one exception to arm-then-act: floating mana is the cheap mistake,
    /// so a mana ability stays one tap. Not `legal.mana_abilities`, which
    /// carries the CR 305.6 shortcut and granted abilities but not a printed
    /// `{T}: Add {G}` — a mana dork would otherwise ask for a confirmation a
    /// basic land does not.
    pub mana: bool,
    /// Whether the whole cost is tapping this permanent and nothing else.
    ///
    /// The other half of the one-tap exemption, and the same rule seen
    /// plainly: a cost paid out of the card itself, which the next untap step
    /// gives back. `{T}: Draw a card` is one tap for the reason a mana ability
    /// is — the mistake costs a turn of that permanent and nothing more. A
    /// sacrifice, a discard, life, mana or a loyalty cost is not paid out of
    /// the card in that sense, and stays arm-then-act.
    ///
    /// `false` wherever the cost cannot be read: a granted ability is in no
    /// ability list this client reads, and `PREPARED_CAST` is not an ability
    /// at all. Guessing
    /// there would fire something unarmed, and an extra tap is the cheaper
    /// way to be wrong.
    pub tap_only: bool,
    /// What it costs, as the engine has it: its mana and its tap or untap
    /// symbol in Scryfall's spelling (`{2}, {T}`), or a walker's badge
    /// (`{L+2}`). Symbols only, and in no language.
    ///
    /// Not the words a row draws. The sheet's cost column is the head of the
    /// ability's **printed** sentence, and this is what licenses cutting it
    /// there ([`baylee_client_core::abilitysheet::cut`]): the owner's rule is
    /// the card's own text, *„nicht custom texte für abilities verwenden,
    /// sondern die echten texte von skryfall"*, and a cost column composed
    /// from the ability was the last custom text on the row. It is drawn
    /// only where no printed sentence exists at all — the CR 305.6 mana of a
    /// basic land type is `{T}` — and then as symbols alone.
    ///
    /// A prepared cast's is the spell's printed mana cost (`{1}{B}`), which is
    /// what casting the copy pays.
    ///
    /// `None` where the ability costs no symbol, or has no cost to read: a
    /// grant is in no ability list this client reads.
    pub cost: Option<String>,
    /// Where this ability's printed sentence is, for a client holding the
    /// card's text in the player's own language.
    ///
    /// The same handle an ability on the stack travels under
    /// ([`baylee_view::StackText`]) and resolved the same way, because it is
    /// the same question: which sentence of this face is this ability. `None`
    /// for everything the generated table has no row for — a reserved index,
    /// a granted ability, a printed one no sentence fits — and the sheet then
    /// draws the prepared spell's or the grantor's words ([`prepared_words`],
    /// [`grant_words`]), else [`Self::label`].
    pub printed: Option<baylee_view::StackText>,
    /// The colour this row pours, when the row is one pip of a mana header.
    ///
    /// `Some` only on a row [`pour_out`] put there, which it does for each
    /// colour a permanent's taps come to — and the rows those pips stand for
    /// are gone, while a mana row no pip stands for keeps its sentence beside
    /// them. Where nothing is left written the header *is* the list, which is
    /// the bubble [`pouring`] names. What makes a pip a pip either way is that
    /// [`crate::input::arm_ability`] answers a row carrying one with a
    /// one-step [`crate::ManaRun`] instead of with an activation — so the
    /// permanent taps on *this* press, having been asked the colour first.
    pub pour: Option<baylee_client_core::manaplan::Pour>,
}

impl AbilityOption {
    /// Which position in the **card's own** ability list this row activates,
    /// if it is one at all.
    ///
    /// The one door for "does the printing say anything about this", which
    /// is the question the sheet's words hang on: every row that answers
    /// `Some` has a printed sentence and draws it, and the three that answer
    /// `None` are printed on no card the permanent has.
    ///
    /// - the CR 305.6 mana of a basic land type, which a Bayou's text does
    ///   not mention and which is offered as `ActivateManaAbility`, carrying
    ///   no index because there is nothing on the card to index;
    /// - a **granted** ability, which the Chromatic Lantern prints and the
    ///   land under it does not ([`baylee_engine::choice::granted_slot`];
    ///   its row draws the Lantern's sentence, [`grant_words`]);
    /// - a **prepared cast**, which is a cast and not an ability
    ///   ([`baylee_engine::choice::PREPARED_CAST`]).
    ///
    /// Asked in one place rather than at each call site, because they are one
    /// question and a second reader would let the next reserved index be
    /// forgotten by one of them —
    /// `activated-conditional-is-a-forgotten-twin`, the same shape.
    #[must_use]
    pub fn printed_index(&self) -> Option<u32> {
        let PlayerAction::ActivateAbility { ability_index, .. } = self.action else {
            return None;
        };
        if baylee_engine::choice::granted_slot(ability_index).is_some()
            || baylee_engine::choice::is_special_action(ability_index)
            || ability_index == baylee_engine::choice::PREPARED_CAST
        {
            return None;
        }
        Some(ability_index)
    }
}

/// Where a sheet's pips end and its numbered rows begin.
///
/// [`pour_out`] puts every pip at the front, so this is a count and not a set,
/// and the whole of the sheet's arithmetic follows from it: the pips are a
/// **header** — one centred row of marks, standing on every page of the list,
/// carrying no digit — and `abilitysheet` counts the rest.
///
/// The alternative was to let the pips take the first digits, which is what
/// they do on a bubble, where there is nothing else on the paper. On a sheet
/// it would put a `6` on Mind Stone's first written row, and a player who has
/// learned "press the number beside it" would be reading a list that starts at
/// six.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Split {
    /// How many pips lead the list.
    pub pips: usize,
    /// How many written rows follow them, which is what the digits count.
    pub rows: usize,
}

impl Split {
    /// Reads the split off a list.
    #[must_use]
    pub fn of(options: &[AbilityOption]) -> Self {
        let pips = options
            .iter()
            .take_while(|option| option.pour.is_some())
            .count();
        Self {
            pips,
            rows: options.len() - pips,
        }
    }

    /// Which option the written row `at` is.
    #[must_use]
    pub const fn option(self, at: usize) -> usize {
        self.pips + at
    }

    /// What the **digits** count: where the numbered rows start, and how many
    /// there are.
    ///
    /// Two answers from one rule. On a sheet the pips are a header and the
    /// digits count the written rows under them. On a *bubble* there are no
    /// written rows, so the pips are what the digits count — which is not an
    /// exception to the rule but the same one applied to a list with nothing
    /// else in it, and it is what keeps a Tundra answerable by `2`.
    #[must_use]
    pub const fn numbered(self) -> (usize, usize) {
        if self.rows == 0 {
            (0, self.pips)
        } else {
            (self.pips, self.rows)
        }
    }

    /// Which page the cursor standing on option `pick` is on.
    ///
    /// Zero for a pip, because the header is on every page: a cursor on a pip
    /// is on a row that is drawn wherever the list happens to be.
    #[must_use]
    pub const fn page_of(self, pick: usize) -> usize {
        pick.saturating_sub(self.pips) / baylee_client_core::abilitysheet::PAGE
    }
}

/// Whether this list is a mana bubble rather than an ability sheet.
///
/// One predicate rather than a second list shape, because everything between
/// the two — the digits, the cursor, the pager, the click — is the same
/// machinery, and a sum type here would have made seven readers branch to
/// reach the same place. What differs is the ink: rows of sentences against a
/// row of pips, which is [`crate::hud::sheet`]'s business alone.
///
/// A bubble is the case where the pips are *all* there is. A permanent that
/// makes mana and does something else keeps its sheet and gets the pips as a
/// header on it, which is [`Split`].
#[must_use]
pub fn pouring(options: &[AbilityOption]) -> bool {
    !options.is_empty() && Split::of(options).rows == 0
}

/// Whether an ability's whole cost is `{T}` on the permanent that has it.
///
/// Read off the printed `Cost` rather than off its label, because the label is
/// a translated sentence and the question is a fact about the card.
#[must_use]
fn tap_only(view: &PlayerView, object: ObjectId, index: u32) -> bool {
    matches!(
        crate::manasources::ability_at(view, object, index),
        Some(
            AbilityDef::Activated { cost, .. }
            | AbilityDef::ActivatedConditional { cost, .. }
            | AbilityDef::ModalActivated { cost, .. }
        ) if cost.mana == ManaCost::ZERO && cost.parts == [CostPart::TapSelf]
    )
}

/// Whether this printed ability is **already on screen** as this permanent's
/// mana row.
///
/// Two conditions, catching different things. The first is the row this
/// permanent's tap actually became, whichever ability won it — including a
/// granted one, which is on no card and which the registry therefore cannot
/// recognise. The second is the CR 305.6 shortcut wearing a card's clothes: a
/// Forest prints `{T}: Add {G}` and the engine offers the shortcut for the
/// same tap, so listing both is listing one button twice.
///
/// It carries its own name because it is its own question. It used to ask
/// [`crate::manasources::printed_source`] — "is this readable as a source at
/// all" — which is the **planner's** question, and it struck off every ability
/// the planner *could* read rather than the one already on screen. That was
/// harmless only while the two questions happened to have the same answer.
/// They stopped: a priced tap has been read since #165 and a ridered one since
/// #149, and neither wins the planner's dedup, so Havenwood Battleground's
/// sacrifice and Yavimaya Coast's coloured half were struck off a menu that
/// was the last place they were reachable from.
#[must_use]
fn already_on_screen(
    view: &PlayerView,
    offered_as_mana: Option<Tap>,
    object: ObjectId,
    index: u32,
) -> bool {
    offered_as_mana == Some(Tap::Ability(index))
        || (offered_as_mana == Some(Tap::Intrinsic)
            && crate::manasources::duplicates_intrinsic(view, object, index))
}

/// Everything `object` is offering right now, in a stable order.
///
/// Stable because the prompt bar draws it as a row of buttons and a list that
/// reordered under a player would activate the wrong thing: the CR 305.6
/// shortcut first, then printed abilities by index.
#[must_use]
pub fn options(
    lang: Lang,
    view: &PlayerView,
    interaction: &Interaction,
    object: ObjectId,
) -> Vec<AbilityOption> {
    options_for(lang, view, interaction, object, None)
}

/// The same list, or the colours of **one** tap on it.
///
/// `asking` is [`crate::Duel::asking_tap`]: the ability whose colour the
/// player has stepped into, and it turns the sheet into a bubble of that
/// tap's pips and nothing else. It is the second half of the owner's third
/// point — *"wenn man den Effekt auswählt, dann verschwindet der Abilities
/// Dialog und es wird wieder der Mana Dialog angezeigt"* — and it exists as a
/// step of its own because the tap it is about makes a number no pip can
/// stand for: five colours twice over is five pips, [`pour_out`] gives them
/// to the tap a player can predict, and the loser is a written row. Pressing
/// that row asks the only question it has left, which is which colour.
///
/// A tap the pips already stand for never gets here — the row is folded away
/// and there is nothing to press. An unknown or withdrawn tap answers with an
/// empty list, which the callers read as "no sheet", because the alternative
/// is a bubble with no pips on it.
#[must_use]
pub fn options_for(
    lang: Lang,
    view: &PlayerView,
    interaction: &Interaction,
    object: ObjectId,
    asking: Option<u32>,
) -> Vec<AbilityOption> {
    let Some(legal) = interaction.legal_actions() else {
        return Vec::new();
    };
    if let Some(index) = asking {
        return mana_pip_options(view, legal, object, index);
    }
    let mut out = Vec::new();

    // The mana half comes through `manasources`, which has already reduced a
    // permanent to the one tap it actually has. A Forest offers its CR 305.6
    // shortcut *and* the `{T}: Add {G}` printed on the card, and a chooser
    // that listed both would be offering the same button twice.
    //
    // `offered_as_mana` is which tap it claimed, so the list below does not
    // offer the same button again under its own name.
    let mut offered_as_mana = None;
    if let Some(source) = crate::manasources::sources(view, legal)
        .into_iter()
        .find(|s| s.id == object)
    {
        // Built from the tap rather than through `Interaction::activate`,
        // which reads index 0 as the CR 305.6 shortcut whenever the object
        // has one. A permanent with both — a typed land whose printed ability
        // at index 0 makes *two* mana — would otherwise be sent the shortcut
        // and make one.
        let action = match source.tap {
            // Built here rather than through `Interaction::activate`, which
            // now prefers an *offered* ability at index 0 over the shortcut —
            // it has to, or a fetchland under a Chromatic Lantern taps for
            // mana instead of searching. `manasources` has already decided
            // that this permanent's mana comes from the shortcut and not from
            // a printed ability, so asking `activate` to guess again could
            // only get a different answer, and a wrong one.
            Tap::Intrinsic => legal
                .mana_abilities
                .contains(&object)
                .then_some(PlayerAction::ActivateManaAbility { source: object }),
            Tap::Ability(index) => legal.abilities.contains(&(object, index)).then_some(
                PlayerAction::ActivateAbility {
                    source: object,
                    ability_index: index,
                },
            ),
        };
        if let Some(action) = action {
            offered_as_mana = Some(source.tap);
            out.push(AbilityOption {
                action,
                // A tap whose amount the board counts (Harabaz Druid's X
                // Allies) is labelled as the printed ability is, "Tap for X
                // mana": the planner reads today's X to pay with it, but the
                // row names the ability, and X is what the ability makes.
                label: match source.tap {
                    Tap::Ability(index) if counted_on_the_board(view, object, index) => {
                        printed_label(lang, view, object, index)
                    }
                    _ => mana_label(lang, &source),
                },
                mana: true,
                // Already one tap because it is a mana ability; answered
                // truthfully anyway, so the two reasons stay separable.
                tap_only: match source.tap {
                    Tap::Ability(index) => tap_only(view, object, index),
                    Tap::Intrinsic => true,
                },
                cost: match source.tap {
                    Tap::Ability(index) => cost_key(view, object, index),
                    // The CR 305.6 shortcut is printed on no card and there
                    // is nothing to read it off; tapping is the whole of it.
                    Tap::Intrinsic => Some("{T}".to_string()),
                },
                // The card's own sentence wherever the card has one, which is
                // what the owner asked for: "Add X mana of any one color,
                // where X is the number of Allies you control" says more —
                // and says it in the printing's own language — than any label
                // this client composes out of a `ManaSource`.
                //
                // It is new, and the reason it could not be done before is in
                // `baylee_cards_codegen::lines::LineShape::Mana`: a mana
                // ability was `Other` on both sides, so the table held `None`
                // for every one of them.
                //
                // `mana_label` stays the fallback, and is the *only* answer
                // for the two taps that are printed nowhere — the CR 305.6
                // shortcut, which a Bayou's text does not mention, and a
                // granted ability, which a Chromatic Lantern prints and the
                // land under it does not.
                printed: match source.tap {
                    Tap::Ability(index) if baylee_engine::choice::granted_slot(index).is_none() => {
                        printed_sentence(view, object, index)
                    }
                    _ => None,
                },
                // Set by `pour_out` below, over the whole list at once or not
                // at all: whether a permanent is a bubble is a question about
                // everything it offers, not about one row of it.
                pour: None,
            });
        }
    }

    for index in payable_indices(view, legal, object) {
        if already_on_screen(view, offered_as_mana, object, index) {
            continue;
        }
        let action = PlayerAction::ActivateAbility {
            source: object,
            ability_index: index,
        };
        // The synthetic indices are not positions on the card, so neither the
        // registry nor the card's ability list has anything to say about
        // them. A prepared cast is a cast and never a mana ability, and it is
        // named by the spell it casts; a granted one is named by its grantor
        // and is a mana ability or not by whichever the view said, per slot.
        let (label, mana) = if let Some(slot) = baylee_engine::choice::granted_slot(index) {
            (
                grant_label(lang, view, object, slot),
                view.object(object)
                    .and_then(|object| object.grant(slot))
                    .map_or_else(
                        || {
                            view.object(object)
                                .and_then(|object| object.granted_mana_for(slot))
                                .is_some()
                        },
                        |grant| grant.mana_ability,
                    ),
            )
        } else if let Some(label) = special_label(lang, view, object, index) {
            (label, false)
        } else if index == baylee_engine::choice::PREPARED_CAST {
            (prepared_label(view, object), false)
        } else {
            (
                printed_label(lang, view, object, index),
                makes_mana(view, object, index),
            )
        };
        out.push(AbilityOption {
            action,
            label,
            mana,
            // Only a printed ability can answer this: a grant is in no list
            // this client reads and a prepared cast is not an ability.
            tap_only: index != baylee_engine::choice::PREPARED_CAST
                && tap_only(view, object, index),
            cost: offered_cost(view, object, index),
            printed: printed_sentence(view, object, index),
            pour: None,
        });
    }
    pour_out(view, legal, object, &mut out);
    out
}

fn payable_indices<'a>(
    view: &'a PlayerView,
    legal: &'a baylee_engine::choice::LegalActions,
    object: ObjectId,
) -> impl Iterator<Item = u32> + 'a {
    let payable = legal
        .unpaid_abilities
        .iter()
        .filter_map(move |&(source, index, _)| {
            (source == object && mana_for(view, legal, source, index).is_some()).then_some(index)
        });
    legal
        .abilities
        .iter()
        .filter_map(move |&(source, index)| (source == object).then_some(index))
        .chain(payable)
}

/// A plan for an engine-verified ability whose only missing cost is mana.
/// Reserve the source itself: tapping or sacrificing it to make mana could
/// invalidate the activation the player is confirming.
#[must_use]
pub fn mana_for(
    view: &PlayerView,
    legal: &baylee_engine::choice::LegalActions,
    source: ObjectId,
    index: u32,
) -> Option<baylee_client_core::manaplan::Plan> {
    let (_, _, cost) = legal
        .unpaid_abilities
        .iter()
        .find(|(s, i, _)| *s == source && *i == index)?;
    let mut pool = view
        .seat(baylee_client_core::decision::resource_player(view))?
        .mana_pool;
    if let Some(object) = view.object(source) {
        pool.spending = object.ability_mana_spending;
    }
    crate::manasources::plan_for_activation(cost, &pool, view, legal, source)
}

/// A permanent's **mana** rows become one row per colour, at the head of the
/// list.
///
/// The owner's fourth point and the first half of his sixth, and it is a
/// rewrite of the list rather than a second kind of list for the reason
/// [`pouring`] gives. What it turns is "this Plains offers one thing, so fire
/// it" — the short-circuit in [`crate::input::activate_card`] — into "this
/// Plains offers white and four colours the grant on it can make, so ask". The
/// card is then tapped by the press that answers, not by the press that asked.
///
/// Where *every* row is mana the pips are the whole sheet, which is the
/// bubble. Where they are not, the pips lead a sheet that still lists
/// everything else — Mind Stone is a `{C}` and a card drawn for a sacrifice,
/// Gates of Istfell a `{W}` and four mana's worth of life and cards — and the
/// owner asked for exactly that: *"die erste Zeile ist quasi sowas wie der
/// Mana-Dialog … nur dass man eben auch die anderen Abilities noch zur Auswahl
/// hat"*. [`Split`] is what the rest of the sheet reads that as.
///
/// **A tap the header does not stand for keeps its sentence.** That is the
/// whole of the partition, and it replaces the all-or-nothing bar this used to
/// open with. A mana row is folded into the pips only where one of them is
/// *that* tap; everything else stays written, and the digits count it.
///
/// Two ways a mana row can fail to be under a pip, and both are cards in the
/// owner's deck:
///
/// - **It is no offer at all.** Jasmine Dragon Tea Shop's any-colour tap is
///   restricted to Allies, and a pip can say "white" but not "white, and only
///   on Ally spells". Its `{T}: Add {C}` becomes the header and the restricted
///   tap keeps the words that are the reason the land is in the deck. This
///   used to leave the whole list alone, out of a worry — a colour in the
///   header with no row to reach it by — that the partition answers directly:
///   the row is right there.
/// - **It lost every colour to another tap.** Harabaz Druid under a Great
///   Divide Guide offers all five colours twice, and one pip per colour is all
///   there is; [`baylee_client_core::manaplan::pours`] gives them to the grant,
///   because a pip claims one mana of the colour pressed and the Druid's own
///   ability makes a number nobody here can count. Folding the loser away is
///   what the owner reported as its own mana ability having "verloren
///   gegangen"; it is a written row now, saying what it makes.
///
/// The other bar is unchanged: **a colour to pour**. One is enough *beside*
/// other rows, because there the pip is a symbol standing where a sentence
/// would be — Mind Stone's lone `{C}` is what the owner asked to see. Alone it
/// is not: a Plains, a Sol Ring, a Llanowar Elf offer nothing to choose
/// between, so they stay on the one-tap path they have always been on.
fn pour_out(
    view: &PlayerView,
    legal: &baylee_engine::choice::LegalActions,
    object: ObjectId,
    out: &mut Vec<AbilityOption>,
) {
    if !out.iter().any(|option| option.mana) {
        return;
    }
    let offers = crate::manasources::offers(view, legal, object);
    let pours = baylee_client_core::manaplan::pours(object, &offers);
    let mut pips: Vec<AbilityOption> = pours
        .into_iter()
        .filter_map(|pour| pip_row(view, legal, object, pour))
        .collect();
    // Which taps the header ended up standing for. Read off the pips rather
    // than off `offers`, because an offer that won no colour is not one of
    // them: it is a second way to tap the same permanent, and the row that
    // names it is the only thing left that can.
    let stood: Vec<Tap> = pips
        .iter()
        .filter_map(|pip| pip.pour.as_ref().map(|pour| pour.step.tap))
        .collect();
    let mut rows: Vec<AbilityOption> = out
        .iter()
        .filter(|option| {
            !option.mana || !tap_of(object, &option.action).is_some_and(|tap| stood.contains(&tap))
        })
        .cloned()
        .collect();
    // Nothing written left means the pips are the whole sheet, which is the
    // bubble, and a bubble with one pip is a question with one answer.
    let floor = if rows.is_empty() { 2 } else { 1 };
    if pips.len() < floor {
        return;
    }
    pips.append(&mut rows);
    *out = pips;
}

/// One pip: a colour, and the tap that pours it.
///
/// Split out of [`pour_out`] because [`options_for`] builds the same row for
/// a bubble that stands for **one** tap rather than for everything the
/// permanent offers, and a second copy of "what a pip is" is two things that
/// can disagree about whether it carries a cost.
fn pip_row(
    view: &PlayerView,
    legal: &baylee_engine::choice::LegalActions,
    object: ObjectId,
    pour: baylee_client_core::manaplan::Pour,
) -> Option<AbilityOption> {
    Some(AbilityOption {
        action: tap_action(legal, &pour.step)?,
        // The pip is the label. A reader that falls back to it —
        // `armed_row`'s fingerprint does — then reads "{W}", which is what
        // the row says.
        label: pip(pour.color).to_string(),
        mana: true,
        // Answered truthfully rather than assumed, for the reason the field's
        // own doc gives: `{T}, Sacrifice this: Add one mana of any color` is a
        // bubble and is not paid out of the card. Nothing reads it here — a
        // pour is armed by `pour` — so the two reasons a row goes through on
        // one press stay separable.
        tap_only: match pour.step.tap {
            Tap::Ability(index) => tap_only(view, object, index),
            Tap::Intrinsic => true,
        },
        // Neither has anything to say here. The cost of a pip is the tap it is
        // drawn on, and its sentence is the colour.
        cost: None,
        printed: None,
        pour: Some(pour),
    })
}

/// What stands before a bubble's pips, when one press of them pours a number
/// nobody here can count.
///
/// The owner's third point: *"allerdings mit einem Präfix, je nachdem was
/// möglich ist. Entweder Xx(Symbol Auswahl), oder für X direkt ausgerechnet
/// die Anzahl an Allys die ich kontrolliere."* A Harabaz Druid's tap makes X
/// mana of **one** colour, so five pips on their own say "which colour" and
/// leave out the half that makes the press worth thinking about.
///
/// One rule rather than a special case: **a bubble whose pips all stand for
/// one tap whose pour is not a number carries a prefix.** That covers the
/// Druid alone, whose bubble opens straight off a click, and the Druid under
/// a Great Divide Guide, whose pips are the *grant* until the written row is
/// pressed and a bubble for its own tap opens. Anything else — a Plains, a
/// Command Tower, a permanent whose pips are two different taps — pours one
/// mana per press and needs no prefix to say so.
///
/// It is `X×` and not the number, which is the second half of what the owner
/// allowed. Counting the Allies means evaluating a `Filter` against a
/// `PlayerView`, and no such evaluator exists this side of the wire — the
/// engine's reads a `GameState`. A wrong count drawn as a fact would be worse
/// than the letter the card itself prints.
#[must_use]
pub fn bubble_prefix(
    view: &PlayerView,
    object: ObjectId,
    options: &[AbilityOption],
) -> Option<String> {
    let mut taps = options
        .iter()
        .filter_map(|option| option.pour.map(|p| p.step.tap));
    let tap = taps.next()?;
    if !taps.all(|other| other == tap) {
        return None;
    }
    (!crate::manasources::countable(view, object, tap)).then(|| "X×".to_string())
}

/// Which tap of `object` an action is, if it is one of its taps at all.
///
/// The mana rows and the pips are built from the same `LegalActions` a few
/// lines apart, so this is not a staleness check — it is how [`pour_out`] asks
/// whether a row it is holding is the one a given pip stands for.
fn tap_of(object: ObjectId, action: &PlayerAction) -> Option<Tap> {
    match *action {
        PlayerAction::ActivateManaAbility { source } if source == object => Some(Tap::Intrinsic),
        PlayerAction::ActivateAbility {
            source,
            ability_index,
        } if source == object => Some(Tap::Ability(ability_index)),
        _ => None,
    }
}

/// The activation one pour's tap is, checked against what the engine listed.
///
/// The same pair of lists [`crate::advance_mana_run`] taps through, read here
/// so the row carries an action like every other row does — a reader that
/// compares actions (an armed deed, a stale-list check) must not find a hole
/// on a bubble's rows.
fn tap_action(
    legal: &baylee_engine::choice::LegalActions,
    step: &baylee_client_core::manaplan::Step,
) -> Option<PlayerAction> {
    match step.tap {
        Tap::Intrinsic => legal.mana_abilities.contains(&step.source).then_some(
            PlayerAction::ActivateManaAbility {
                source: step.source,
            },
        ),
        Tap::Ability(ability_index) => legal
            .abilities
            .contains(&(step.source, ability_index))
            .then_some(PlayerAction::ActivateAbility {
                source: step.source,
                ability_index,
            }),
    }
}

/// Whether a printed ability is a mana ability (CR 605.1).
///
/// Read off the card's own `mana_ability` flag, which is the only answer that
/// is true for every card. `manasources` reduces a permanent to the *one* tap
/// it usually has, which is right for a mana plan and wrong here: Yavimaya
/// Coast prints two mana abilities, and the second would have asked for a
/// confirmation the first does not.
fn makes_mana(view: &PlayerView, object: ObjectId, index: u32) -> bool {
    matches!(
        crate::manasources::ability_at(view, object, index),
        Some(
            AbilityDef::Activated {
                mana_ability: true,
                ..
            } | AbilityDef::ActivatedConditional {
                mana_ability: true,
                ..
            } | AbilityDef::ModalActivated {
                mana_ability: true,
                ..
            }
        )
    )
}

/// Whether printed ability `index` of `object` adds an amount that is not a
/// number on the card but a count the board answers (`mana_written` reads
/// no fixed amount). A granted ability is printed nowhere and never is.
fn counted_on_the_board(view: &PlayerView, object: ObjectId, index: u32) -> bool {
    if baylee_engine::choice::granted_slot(index).is_some() {
        return false;
    }
    matches!(
        crate::manasources::ability_at(view, object, index),
        Some(
            AbilityDef::Activated { effects, .. } | AbilityDef::ActivatedConditional { effects, .. }
        ) if baylee_cards_dsl::mana_written(effects).is_some_and(|(_, amount, _)| amount.is_none())
    )
}

/// "Tap for {G}", or "Tap for {U} or {B}" where there is a choice to make —
/// and "Tap for any color" where there is no choice worth drawing.
///
/// A **bundle** is none of those and is drawn as what arrives: "Tap for
/// {W}{U}". It cannot go through [`mana_choice`], and not merely because the
/// wording would read oddly. That function answers "any color" as soon as all
/// five are listed, which is true of a choice among five and false of
/// Composite Golem, whose sacrifice adds one of each and offers nothing — the
/// player would be told they may have any colour they like, and then be given
/// five specific ones.
fn mana_label(lang: Lang, source: &baylee_client_core::manaplan::Source) -> String {
    let made = if source.bundle {
        source.colors.iter().map(|c| pip(*c)).collect::<String>()
    } else {
        mana_choice(lang, &source.colors, source.amount)
    };
    Phrase::TapFor.fill(lang, &[&made])
}

/// A printed ability's label: a planeswalker's loyalty cost, what a mana
/// ability makes, otherwise the symbols the ability costs.
///
/// Deliberately short, and no longer what a player reads: the sheet and the
/// armed shelf draw the ability's printed sentence and its printed cost
/// ([`printed_words`]). This is what they fall back to where a card prints
/// no sentence for the ability at all, and the sheet's redraw fingerprint.
/// Symbols only, because the rest of a cost is words and the only words
/// allowed on a row are the card's.
///
/// Empty where there are no symbols either. It used to say "Ability 4",
/// which is a number and not the card; the row then draws its cost column
/// and its key, and the words are the sentence's, swept pool-wide by
/// `every_written_row_knows_which_printed_sentence_it_is`.
fn printed_label(lang: Lang, view: &PlayerView, object: ObjectId, index: u32) -> String {
    let unnamed = String::new;
    let Some(def) = crate::manasources::ability_at(view, object, index) else {
        return unnamed();
    };
    match def {
        AbilityDef::Loyalty { cost, .. } => manapip::loyalty_token(*cost),
        AbilityDef::ModalActivated { cost, .. } => cost_key_of(cost).unwrap_or_else(unnamed),
        AbilityDef::Activated {
            cost,
            effects,
            mana_ability,
            ..
        }
        | AbilityDef::ActivatedConditional {
            cost,
            effects,
            mana_ability,
            ..
        } => {
            // **A mana row says what it makes.** One rule for every shape of
            // mana ability, and it replaced two narrow ones that between them
            // left 126 of the pool's 374 written rows drawing their own cost.
            //
            // The two read `mana_shape` and `mana_offer`, which are questions
            // about the ability *as a source* — so they refuse an ability that
            // charges mana (Mystic Gate), one that does anything besides add
            // (Yavimaya Coast's damage), and, between the pair of them, the
            // plain case of all: unrestricted, with an amount, which matched
            // the first branch's `true` and the second's `None` and so neither.
            // `manasources` reads through the same doors, correctly, because
            // it is planning; a label is not.
            //
            // `mana_written` is the label's own door and takes no cost at all.
            // The cost is drawn in its own column a few pixels away, and
            // drawing it twice is the whole defect.
            if *mana_ability
                && let Some((source, amount, restricted)) = baylee_cards_dsl::mana_written(effects)
                && let Some(colors) =
                    crate::manasources::produced_colors(view, object, index, source)
                && !colors.is_empty()
            {
                let colors = mana_choice(lang, &colors, amount.unwrap_or(1));
                // Three wordings, because three different things are unknown.
                // A restriction is a rules question this side cannot answer
                // (CR 106.6), an amount only a board can count is not a number
                // to print, and everything else is simply what it says.
                return match (restricted, amount) {
                    (true, _) => Phrase::TapForRestricted,
                    (false, None) => Phrase::TapForVariable,
                    (false, Some(_)) => Phrase::TapFor,
                }
                .fill(lang, &[&colors]);
            }
            cost_key_of(cost).unwrap_or_else(unnamed)
        }
        _ => unnamed(),
    }
}

/// The spell a prepared permanent casts a copy of (`AbilityDef::Prepared`),
/// read off the card its abilities are printed on.
#[must_use]
pub fn prepared_of(view: &PlayerView, object: ObjectId) -> Option<baylee_view::RulesFace> {
    let object = view.object(object)?;
    object
        .prepared_spell
        .or_else(|| prepared_spell(object.rules?.card))
}

/// The spell `card` is linked to by `AbilityDef::Prepared`, if it has one.
///
/// The metadata includes the inset face when both spells share a definition.
#[must_use]
pub fn prepared_spell(card: baylee_core::ids::CardIndex) -> Option<baylee_view::RulesFace> {
    baylee_cards::by_index(card)?
        .abilities
        .iter()
        .find_map(|ability| match ability {
            AbilityDef::Prepared { card, face } => Some(baylee_view::RulesFace {
                card: *card,
                face: *face,
            }),
            _ => None,
        })
}

/// A prepared cast's label: the spell's English name, which is what a row
/// falls back to and what the sheet's redraw fingerprint reads. The sheet
/// itself draws the spell's name and text in the player's language
/// ([`prepared_words`]). Empty where no spell is linked.
/// The label of a special action (CR 116.2): turning a permanent face up, or
/// unlocking a door of a Room. Neither is a mana ability.
fn special_label(lang: Lang, view: &PlayerView, object: ObjectId, index: u32) -> Option<String> {
    if index == baylee_engine::choice::TURN_FACE_UP {
        return Some(Phrase::TurnFaceUp.text(lang).to_string());
    }
    let half = baylee_engine::choice::door_to_unlock(index)?;
    Some(unlock_label(lang, view, object, half))
}

/// The half of a Room `half` names (0 the left, 1 the right).
fn door(
    view: &PlayerView,
    object: ObjectId,
    half: u8,
) -> Option<&'static baylee_cards_dsl::FaceDef> {
    baylee_cards::by_index(view.object(object)?.card?.index)?
        .faces
        .get(usize::from(half))
}

/// "Unlock Forgotten Cellar": the special action names the door it opens
/// (CR 709.5e), in the pool's English, as a named card is.
fn unlock_label(lang: Lang, view: &PlayerView, object: ObjectId, half: u8) -> String {
    Phrase::UnlockDoor.fill(lang, &[door(view, object, half).map_or("", |f| f.name)])
}

fn prepared_label(view: &PlayerView, object: ObjectId) -> String {
    prepared_of(view, object)
        .and_then(|spell| {
            baylee_cards::by_index(spell.card)?
                .faces
                .get(usize::from(spell.face))
        })
        .map(|face| face.name.to_string())
        .unwrap_or_default()
}

/// What casting a copy of `spell` costs: its selected face's printed mana cost,
/// as symbols. `None` for a spell with no mana cost.
fn spell_cost(spell: baylee_view::RulesFace) -> Option<String> {
    let cost = baylee_cards::by_index(spell.card)?
        .faces
        .get(usize::from(spell.face))?
        .mana_cost;
    (!cost.is_empty()).then(|| cost.to_string())
}

/// What a prepared cast's row says: the spell's name over its whole text,
/// in the player's language where it has arrived, else in English
/// ([`crate::cardtext::CardTexts::face`]), and which of the two it is.
///
/// The spell's text and not the permanent's: a prepared card prints its
/// spell, and the German printing of Emeritus of Woe prints that face
/// empty, where Demonic Tutor's own German printing does not. The linked
/// card is asked for with the cards the view names (`cardtext::wanted`).
#[must_use]
pub fn prepared_words(
    texts: &crate::cardtext::CardTexts,
    view: &PlayerView,
    object: ObjectId,
    option: &AbilityOption,
) -> Option<(Vec<TextBlock>, crate::cardtext::Said)> {
    let PlayerAction::ActivateAbility { ability_index, .. } = option.action else {
        return None;
    };
    if ability_index != baylee_engine::choice::PREPARED_CAST {
        return None;
    }
    let spell = prepared_of(view, object)?;
    let said = if texts
        .get(spell.card, spell.face)
        .is_some_and(|text| text.lang != "en")
    {
        crate::cardtext::Said::Localized
    } else {
        crate::cardtext::Said::Oracle
    };
    let text = texts.face(spell.card, spell.face)?;
    let mut blocks = vec![TextBlock::Rules(text.name)];
    blocks.extend(baylee_client_core::card_face::split_blocks(
        &text.oracle_text,
    ));
    Some((blocks, said))
}

/// Who granted the ability in `slot`, as the view says (#212): the grantor's
/// card and face, and which of that face's sentences granted it.
fn grant_of(view: &PlayerView, object: ObjectId, slot: u32) -> Option<&baylee_view::GrantSource> {
    view.object(object)?.grant(slot)
}

/// A granted ability's label: the English name of the face that grants it,
/// which is what a row falls back to and what the sheet's redraw fingerprint
/// reads. The sheet itself draws that name and sentence in the player's
/// language ([`grant_words`]).
///
/// "Granted ability" only where the view names no grantor: one this seat
/// may not see, or a slot the view has no entry for.
fn grant_label(lang: Lang, view: &PlayerView, object: ObjectId, slot: u32) -> String {
    grant_of(view, object, slot)
        .and_then(|grant| grant.rules)
        .and_then(|rules| crate::cardtext::english(rules.card, usize::from(rules.face)))
        .map_or_else(
            || Phrase::GrantedAbility.text(lang).to_string(),
            |text| text.name,
        )
}

/// What a granted ability's row says: the grantor's name over the sentence
/// that grants it, each in the player's language where it has arrived, else
/// in English ([`crate::cardtext::CardTexts::face`],
/// [`crate::cardtext::said`]), and which of the two the sentence is.
///
/// The ability is printed on no card the permanent has. The land under a
/// Chromatic Lantern prints nothing about its `{T}`. The Lantern prints it,
/// in the sentence saying lands "have" it (CR 113.10), which is the card's
/// own text for this row and says where the ability came from as well.
///
/// The name alone where the view names the grantor and no sentence
/// (`GrantSource::text` is `None`: nothing on its card wrote this grant by
/// value), and `None` where it names no grantor at all. The sheet then
/// draws [`AbilityOption::label`].
#[must_use]
pub fn grant_words(
    texts: &crate::cardtext::CardTexts,
    view: &PlayerView,
    object: ObjectId,
    option: &AbilityOption,
) -> Option<(Vec<TextBlock>, crate::cardtext::Said)> {
    let PlayerAction::ActivateAbility { ability_index, .. } = option.action else {
        return None;
    };
    let slot = baylee_engine::choice::granted_slot(ability_index)?;
    let grant = grant_of(view, object, slot)?;
    let rules = grant.rules?;
    let name = texts.face(rules.card, rules.face)?.name;
    let sentence = grant
        .text
        .and_then(|at| crate::cardtext::said(Some(texts), rules.card, at));
    let said = match &sentence {
        Some((_, said)) => *said,
        None if texts
            .get(rules.card, rules.face)
            .is_some_and(|text| text.lang != "en") =>
        {
            crate::cardtext::Said::Localized
        }
        None => crate::cardtext::Said::Oracle,
    };
    let mut blocks = vec![TextBlock::Rules(name)];
    blocks.extend(sentence.map(|(sentence, _)| sentence).unwrap_or_default());
    Some((blocks, said))
}

/// [`AbilityOption::cost`] for an offered index: a prepared cast's is the
/// spell's mana cost, anything else's its [`cost_key`].
fn offered_cost(view: &PlayerView, object: ObjectId, index: u32) -> Option<String> {
    if let Some(half) = baylee_engine::choice::door_to_unlock(index) {
        return door(view, object, half).map(|face| face.mana_cost.to_string());
    }
    if index == baylee_engine::choice::TURN_FACE_UP {
        let card = view.object(object)?.card?;
        let face = baylee_cards::by_index(card.index)?
            .faces
            .get(usize::from(card.face))?;
        return face.disguise.or(face.morph).map(|c| c.to_string());
    }
    if index == baylee_engine::choice::PREPARED_CAST {
        prepared_of(view, object).and_then(spell_cost)
    } else {
        cost_key(view, object, index)
    }
}

/// What a printed ability costs, as the key its printed head is licensed by.
///
/// See [`AbilityOption::cost`]. A planeswalker's loyalty change *is* its cost,
/// an activated ability's is what it pays, and a trigger has none at all.
///
/// The reserved indices answer `None` here rather than being checked by the
/// caller, because `ability_at` already has to look the ability up and a
/// reserved index simply finds nothing.
fn cost_key(view: &PlayerView, object: ObjectId, index: u32) -> Option<String> {
    match crate::manasources::ability_at(view, object, index)? {
        AbilityDef::Loyalty { cost, .. } => Some(manapip::loyalty_token(*cost)),
        AbilityDef::Activated { cost, .. }
        | AbilityDef::ActivatedConditional { cost, .. }
        | AbilityDef::ModalActivated { cost, .. } => cost_key_of(cost),
        _ => None,
    }
}

/// What a printed ability's row says: the cost its column draws, and the
/// words beside it.
///
/// The sentence is [`crate::cardtext::sentence`]'s — the player's language
/// where their text pairs with the line table, the compiled English Oracle
/// otherwise — and the cut is [`abilitysheet::cut`]'s, licensed by
/// [`AbilityOption::cost`]. One reading for the sheet and the armed shelf, so
/// the two cannot draw a different cost for one ability.
///
/// `None` where the card prints no sentence for the row: the CR 305.6 tap, a
/// grant, a prepared cast, and a permanent whose card the view does not name.
///
/// [`abilitysheet::cut`]: baylee_client_core::abilitysheet::cut
#[must_use]
pub fn printed_words(
    texts: Option<&crate::cardtext::CardTexts>,
    view: &PlayerView,
    object: ObjectId,
    option: &AbilityOption,
) -> Option<baylee_client_core::abilitysheet::Cut> {
    let object = view.object(object)?;
    let blocks = if let Some(at) = option.printed {
        crate::cardtext::sentence(texts, object.rules?.card, at)?
    } else {
        let PlayerAction::ActivateAbility { ability_index, .. } = option.action else {
            return None;
        };
        if object.status.is_face_down() {
            return None;
        }
        crate::cardtext::token_sentence(object.token?, ability_index)?
    };
    Some(baylee_client_core::abilitysheet::cut(
        blocks,
        option.cost.as_deref().unwrap_or_default(),
    ))
}

/// Which printed sentence a permanent's ability came from.
///
/// The face is the one the permanent is *showing*, which is the right answer
/// here and is not the right answer for an ability already on the stack: that
/// one is independent of its source (CR 113.7a) and may outlive a transform,
/// which is why `baylee_view::StackText::face` is captured by the host. An
/// ability being offered is being offered by the object as it stands.
fn printed_sentence(
    view: &PlayerView,
    object: ObjectId,
    index: u32,
) -> Option<baylee_view::StackText> {
    let rules = view.object(object)?.rules?;
    let line = baylee_cards::lines::ability_line(rules.card, rules.face as usize, index)?;
    Some(baylee_view::StackText {
        face: rules.face,
        line: line.line,
        of: line.of,
    })
}

/// The symbols an activated ability's cost prints: its mana, then its tap or
/// untap and energy, apart as the card writes them (`{2}{B}, {T}`).
///
/// Everything else a cost asks — a sacrifice, life, a discard, counters — is
/// printed in words, and only the printed head can say those in the card's
/// own ones; seventeen wordings of them lived here once. `None` when nothing
/// is left, which is every cost printed in words alone.
fn cost_key_of(cost: &Cost) -> Option<String> {
    if cost.mana == ManaCost::ZERO && cost.parts.is_empty() {
        return Some("{0}".to_string());
    }
    let mut parts: Vec<String> = Vec::new();
    if cost.mana != ManaCost::ZERO {
        parts.push(cost.mana.to_string());
    }
    for part in cost.parts {
        match part {
            CostPart::TapSelf => parts.push("{T}".to_string()),
            CostPart::UntapSelf => parts.push("{Q}".to_string()),
            CostPart::PayEnergy(n) => parts.push("{E}".repeat(usize::from(*n))),
            CostPart::PayEnergyX => parts.push("{E}".to_string()),
            // Printed in words, which only the printed head can say.
            _ => {}
        }
    }
    (!parts.is_empty()).then(|| parts.join(COST_JOIN))
}

/// What a cost's payments are written apart with.
///
/// The printed card's own punctuation: `{2}{B}, {T}, Sacrifice this`. Named
/// because [`payments`] takes it back apart, and a separator spelled out at
/// both ends is a pair that drifts.
const COST_JOIN: &str = ", ";

/// A cost, back into the payments it was written from.
///
/// A cost is a *list* — mana, then a tap, then whatever else the card asks
/// for — and a card prints it as one line. Drawn as a narrow column beside a
/// sentence it wants to be a list again: a comma that wrapped onto a line of
/// its own is the shape that made this necessary, and `{2}{U}{U}` over `{T}`
/// over `Sacrifice this artifact` is how a player reads what an ability
/// charges anyway.
///
/// The printed head is split at the card's own punctuation, which is this
/// separator in English and German and the inverse of [`cost_key_of`]'s
/// `join`. A printing that separates its payments otherwise (a full-width
/// comma) comes back as one payment, which the sheet draws as a title over
/// the sentence rather than a column beside it.
pub fn payments(cost: &str) -> impl Iterator<Item = &str> {
    cost.split(COST_JOIN)
        .map(str::trim)
        .filter(|part| !part.is_empty())
}

/// One mana symbol, as a letter.
const fn pip(color: ManaColor) -> &'static str {
    match color {
        ManaColor::White => "{W}",
        ManaColor::Blue => "{U}",
        ManaColor::Black => "{B}",
        ManaColor::Red => "{R}",
        ManaColor::Green => "{G}",
        ManaColor::Colorless => "{C}",
    }
}

/// What a source makes, written so that [`crate::manaui::spawn_rich`] can
/// draw it: the symbols in braces, or the words for "any colour".
///
/// Three shapes, and the first two are the owner's complaint. Five discs in a
/// row is not "any colour" — the printed cards say the words and so does this
/// — and a run of bare letters ("Tap for WUBRG") was never a symbol at all.
/// A short choice keeps its symbols and gets a conjunction, because two or
/// three discs read at a glance where five do not.
fn mana_choice(lang: Lang, colors: &[ManaColor], amount: u8) -> String {
    const EVERY: [ManaColor; 5] = [
        ManaColor::White,
        ManaColor::Blue,
        ManaColor::Black,
        ManaColor::Red,
        ManaColor::Green,
    ];
    if EVERY.iter().all(|c| colors.contains(c)) {
        return Phrase::AnyColor.text(lang).to_string();
    }
    match colors {
        // Nothing to choose: "{G}{G}" is what a Bloom Tender-shaped source
        // makes, and repeating the symbol says so where "{G} x2" would make a
        // player do arithmetic.
        [one] => pip(*one).repeat(amount.max(1) as usize),
        [] => String::new(),
        [rest @ .., last] => {
            let head = rest.iter().map(|c| pip(*c)).collect::<Vec<_>>().join(", ");
            Phrase::OrLast.fill(lang, &[&head, pip(*last)])
        }
    }
}

/// Label an exact mandatory mana offer through the existing ability formatter.
pub(crate) fn required_mana_label(
    lang: Lang,
    view: &PlayerView,
    option: baylee_engine::choice::ManaAbilityChoice,
) -> String {
    let Some(index) = option.ability_index else {
        return view
            .object(option.source.object)
            .and_then(|object| baylee_client_core::manaplan::basic_land_color(&object.subtypes))
            .map_or_else(
                || "{T}".to_string(),
                |color| format!("{{T}}: {}", Phrase::TapFor.fill(lang, &[pip(color)])),
            );
    };
    let cost = match crate::manasources::ability_at(view, option.source.object, index) {
        Some(
            AbilityDef::Activated { cost, .. }
            | AbilityDef::ActivatedConditional { cost, .. }
            | AbilityDef::ModalActivated { cost, .. },
        ) => cost_key_of(cost).unwrap_or_default(),
        _ => String::new(),
    };
    let effect = printed_label(lang, view, option.source.object, index);
    if cost.is_empty() || cost == effect {
        effect
    } else {
        format!("{cost}: {effect}")
    }
}

fn mana_pip_options(
    view: &PlayerView,
    legal: &baylee_engine::choice::LegalActions,
    object: ObjectId,
    index: u32,
) -> Vec<AbilityOption> {
    let tap = Tap::Ability(index);
    let mut offers: Vec<_> = crate::manasources::offers(view, legal, object)
        .into_iter()
        .filter(|offer| offer.tap == tap)
        .collect();
    // A tap with a rider beside its mana (a painland's "{W} or {U}. This
    // land deals 1 damage to you.") is no plain offer, but the planner reads
    // it as the permanent's priced mode, and its colours are what the bubble
    // asks between. A payment window steps into it so the colour is chosen
    // before anything is tapped.
    if offers.is_empty() {
        offers.extend(
            crate::manasources::sources(view, legal)
                .into_iter()
                .filter(|source| source.id == object)
                .find_map(|source| {
                    if source.tap == tap && !source.bundle {
                        Some(source.colors)
                    } else {
                        source
                            .dear
                            .filter(|dear| dear.tap == tap)
                            .map(|dear| dear.colors)
                    }
                })
                .map(|colors| baylee_client_core::manaplan::Offer {
                    tap,
                    colors,
                    fixed: true,
                }),
        );
    }
    baylee_client_core::manaplan::pours(object, &offers)
        .into_iter()
        .filter_map(|pour| pip_row(view, legal, object, pour))
        .collect()
}

#[cfg(test)]
mod tests;
