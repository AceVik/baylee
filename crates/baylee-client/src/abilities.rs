//! What a permanent can do, and what to call it.
//!
//! The client could not activate an ability at all before this: clicking a
//! permanent selected it for whatever choice was pending, and a Forest, a
//! mana dork and a planeswalker were all equally inert. `Interaction::activate`
//! existed and nothing called it.
//!
//! What is here is the list, in a stable order, built only from what the
//! engine offered — and a label for each, which is the part that needs the
//! card registry and is therefore the reason this is not in
//! `baylee-client-core`. "Ability 2" is a label a player has to guess at;
//! "Tap for {G}" and "+1" are not.

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
    /// The action that does it — built through [`Interaction::activate`], so
    /// it is one the engine listed.
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
    /// `false` wherever the cost cannot be read: a granted ability is printed
    /// on no card, and `PREPARED_CAST` is not an ability at all. Guessing
    /// there would fire something unarmed, and an extra tap is the cheaper
    /// way to be wrong.
    pub tap_only: bool,
    /// What it costs, written so [`crate::manaui::spawn_rich`] can draw it:
    /// `{2}, {T}` becomes two discs and a comma.
    ///
    /// The sheet puts this on the right of the row and the ability's own
    /// sentence on the left, which is why it is a field and no longer merely
    /// the [`Self::label`] a button had room for. `None` where there is no
    /// cost to read — a grant is printed on no card, a prepared cast is not
    /// an ability, and a free ability's cost is nothing rather than "0".
    pub cost: Option<String>,
    /// Where this ability's printed sentence is, for a client holding the
    /// card's text in the player's own language.
    ///
    /// The same handle an ability on the stack travels under
    /// ([`baylee_view::StackText`]) and resolved the same way, because it is
    /// the same question: which sentence of this face is this ability. `None`
    /// for everything the generated table has no row for — a reserved index,
    /// a granted ability, a printed one no sentence fits — and the sheet then
    /// falls back to [`Self::label`].
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
    /// `None` are printed on no card and can only be named by this client.
    ///
    /// - the CR 305.6 mana of a basic land type, which a Bayou's text does
    ///   not mention and which is offered as `ActivateManaAbility`, carrying
    ///   no index because there is nothing on the card to index;
    /// - a **granted** ability, which the Chromatic Lantern prints and the
    ///   land under it does not ([`baylee_engine::choice::granted_slot`]);
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
            AbilityDef::Activated { cost, .. } | AbilityDef::ActivatedConditional { cost, .. }
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
        let tap = Tap::Ability(index);
        let offers: Vec<_> = crate::manasources::offers(view, legal, object)
            .into_iter()
            .filter(|offer| offer.tap == tap)
            .collect();
        return baylee_client_core::manaplan::pours(object, &offers)
            .into_iter()
            .filter_map(|pour| pip_row(view, legal, object, pour))
            .collect();
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
                label: mana_label(lang, &source),
                mana: true,
                // Already one tap because it is a mana ability; answered
                // truthfully anyway, so the two reasons stay separable.
                tap_only: match source.tap {
                    Tap::Ability(index) => tap_only(view, object, index),
                    Tap::Intrinsic => true,
                },
                cost: match source.tap {
                    Tap::Ability(index) => printed_cost(lang, view, object, index),
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

    for &(source, index) in &legal.abilities {
        if source != object {
            continue;
        }
        if already_on_screen(view, offered_as_mana, object, index) {
            continue;
        }
        let Some(action) = interaction.activate(object, index) else {
            continue;
        };
        // The synthetic indices are not positions on the card, so neither the
        // registry nor the card's ability list has anything to say about
        // them — and the fallback label counts them out as "Ability N", which
        // on `GRANTED_ABILITY` overflows: in a debug build the client dies the
        // moment a Chromatic Lantern's land is under the pointer, and in a
        // release build the button reads "Ability 0". A prepared cast is a
        // cast and never a mana ability; a granted one is whichever the view
        // said, per slot.
        let (label, mana) = if let Some(slot) = baylee_engine::choice::granted_slot(index) {
            (Phrase::GrantedAbility.text(lang).to_string(), {
                match view.object(object).and_then(|o| o.granted_mana.as_ref()) {
                    // Whether *this* grant is the mana one, not whether the
                    // permanent has one anywhere: Urza's Saga is granted a
                    // mana ability and a Construct ability, and
                    // `legal.mana_abilities` holds the Saga for the first.
                    Some(granted) => granted.slot == slot,
                    // The view could not reduce this grant to "n mana of
                    // these colours" — a `LandColor` source, say. The engine
                    // still answered the question, but per *permanent*, so
                    // it only settles anything where the permanent offers one
                    // grant. Where it offers several, the safe reading is
                    // "not a mana ability": that costs an extra tap, while
                    // the other way round fires a Construct with no arming.
                    //
                    // And the permanent's *own* mana ability has to be
                    // counted out first, because it is in the same list: a
                    // Mountain granted one non-mana ability is named there
                    // for CR 305.6, and reading that as the grant would fire
                    // the grant unarmed — the exact mistake this branch is
                    // written to avoid.
                    None => {
                        granted_mana_offers(view, legal, object) >= 1
                            && granted_count(legal, object) == 1
                    }
                }
            })
        } else if index == baylee_engine::choice::PREPARED_CAST {
            (Phrase::PreparedCast.text(lang).to_string(), false)
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
            // Only a printed ability can answer this: a grant is on no card
            // and a prepared cast is not an ability.
            tap_only: baylee_engine::choice::granted_slot(index).is_none()
                && index != baylee_engine::choice::PREPARED_CAST
                && tap_only(view, object, index),
            cost: printed_cost(lang, view, object, index),
            printed: printed_sentence(view, object, index),
            pour: None,
        });
    }
    pour_out(view, legal, object, &mut out);
    out
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

/// How many of `object`'s entries in `legal.mana_abilities` are *grants*.
///
/// The engine names a permanent there once for a mana ability of its own —
/// printed, or the intrinsic one a basic land type carries (CR 305.6) — and
/// once more per granted mana ability. So its own has to be subtracted before
/// what is left can be read as grants at all.
///
/// The subtraction is deliberately unconditional on whether that own ability
/// is *currently* activatable: a permanent whose own ability is already spent
/// is named once for the grant alone, and counting it out anyway answers
/// "not a mana ability", which costs a tap instead of firing something
/// unarmed.
fn granted_mana_offers(
    view: &PlayerView,
    legal: &baylee_engine::choice::LegalActions,
    object: ObjectId,
) -> usize {
    let named = legal
        .mana_abilities
        .iter()
        .filter(|o| **o == object)
        .count();
    named.saturating_sub(usize::from(owns_a_mana_ability(view, object)))
}

/// Whether the permanent has a mana ability that is not a grant.
///
/// The subtypes are the projected ones, which is what CR 305.6 asks for: an
/// animated Mountain is still a Mountain and still taps for `{R}`.
///
/// Deliberately not `manaplan::basic_land_color`, which answers a different
/// question — it returns `None` for a Taiga, because *which* colour one tap
/// makes has no single answer there. What is asked here is whether the land
/// has an intrinsic mana ability at all, and a dual has two.
fn owns_a_mana_ability(view: &PlayerView, object: ObjectId) -> bool {
    use baylee_core::generated::subtypes::land;
    let Some(o) = view.object(object) else {
        return false;
    };
    if [
        land::PLAINS,
        land::ISLAND,
        land::SWAMP,
        land::MOUNTAIN,
        land::FOREST,
    ]
    .into_iter()
    .any(|t| o.subtypes.contains(t))
    {
        return true;
    }
    let Some(card) = o.card else {
        return false;
    };
    baylee_cards::by_index(card.index).is_some_and(|def| {
        def.abilities_for_face(card.face as usize).iter().any(|a| {
            matches!(
                a,
                AbilityDef::Activated {
                    mana_ability: true,
                    ..
                } | AbilityDef::ActivatedConditional {
                    mana_ability: true,
                    ..
                }
            )
        })
    })
}

/// How many granted abilities the engine is offering for `object` right now.
fn granted_count(legal: &baylee_engine::choice::LegalActions, object: ObjectId) -> usize {
    legal
        .abilities
        .iter()
        .filter(|(o, i)| *o == object && baylee_engine::choice::granted_slot(*i).is_some())
        .count()
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
            }
        )
    )
}

/// "Tap for {G}", or "Tap for {U} or {B}" where there is a choice to make —
/// and "Tap for any color" where there is no choice worth drawing.
fn mana_label(lang: Lang, source: &baylee_client_core::manaplan::Source) -> String {
    Phrase::TapFor.fill(lang, &[&mana_choice(lang, &source.colors, source.amount)])
}

/// A printed ability's label: a planeswalker's loyalty cost, otherwise what
/// the ability costs to activate.
///
/// Deliberately short — this is a button on a bar that already carries the
/// prompt, and a player who needs the full wording has the card's own text a
/// hover away. But short is not the same as opaque: "Ability 2" is a label a
/// player has to count out on the card, and it was the only one this could
/// produce. `{2}, {T}` is read at a glance and is the half of an ability a
/// player is actually deciding about.
fn printed_label(lang: Lang, view: &PlayerView, object: ObjectId, index: u32) -> String {
    let unnamed = || Phrase::AbilityNumbered.fill(lang, &[&(index + 1).to_string()]);
    let Some(def) = crate::manasources::ability_at(view, object, index) else {
        return unnamed();
    };
    match def {
        AbilityDef::Loyalty { cost, .. } => manapip::loyalty_token(*cost),
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
            cost_label(lang, cost).unwrap_or_else(unnamed)
        }
        _ => unnamed(),
    }
}

/// What a printed ability costs, as one short rich string.
///
/// The other half of [`printed_label`], split out because the sheet draws the
/// two in different columns: a planeswalker's loyalty change *is* its cost,
/// an activated ability's is what it pays, and a trigger has none at all.
///
/// The reserved indices answer `None` here rather than being checked by the
/// caller, because `ability_at` already has to look the ability up and a
/// reserved index simply finds nothing.
fn printed_cost(lang: Lang, view: &PlayerView, object: ObjectId, index: u32) -> Option<String> {
    match crate::manasources::ability_at(view, object, index)? {
        AbilityDef::Loyalty { cost, .. } => Some(manapip::loyalty_token(*cost)),
        AbilityDef::Activated { cost, .. } | AbilityDef::ActivatedConditional { cost, .. } => {
            cost_label(lang, cost)
        }
        _ => None,
    }
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
    let card = view.object(object)?.card?;
    let line = baylee_cards::lines::ability_line(card.index, card.face as usize, index)?;
    Some(baylee_view::StackText {
        face: card.face,
        line: line.line,
        of: line.of,
    })
}

/// What an activated ability costs, as one short string.
///
/// `None` for a free ability: "" is not a button and "Free" would be a claim
/// about the *effect* rather than the cost, so the caller falls back to the
/// ability's position instead.
fn cost_label(lang: Lang, cost: &Cost) -> Option<String> {
    let mut parts: Vec<String> = Vec::new();
    if cost.mana != ManaCost::ZERO {
        parts.push(cost.mana.to_string());
    }
    for part in cost.parts {
        parts.push(match part {
            CostPart::TapSelf => "{T}".to_string(),
            CostPart::UntapSelf => "{Q}".to_string(),
            CostPart::SacrificeSelf => Phrase::CostSacrificeThis.text(lang).to_string(),
            CostPart::Sacrifice(_) => Phrase::CostSacrifice.text(lang).to_string(),
            CostPart::PayLife(n) => Phrase::CostPayLife.fill(lang, &[&n.to_string()]),
            CostPart::PayLifeX => Phrase::CostPayXLife.text(lang).to_string(),
            CostPart::Discard(_) => Phrase::CostDiscard.text(lang).to_string(),
            CostPart::DiscardSelf => Phrase::CostDiscardThis.text(lang).to_string(),
            CostPart::ExileSelf => Phrase::CostExileThis.text(lang).to_string(),
            CostPart::ReturnSelfToHand => Phrase::CostReturnThis.text(lang).to_string(),
            CostPart::ExileFromHand(_) => Phrase::CostExileACard.text(lang).to_string(),
            CostPart::TapOther(_) => Phrase::CostTapAnother.text(lang).to_string(),
            CostPart::ReturnToHand(_) => Phrase::CostReturnAnother.text(lang).to_string(),
            CostPart::RemoveCounterSelf { n: 1, .. } => {
                Phrase::CostRemoveCounter.text(lang).to_string()
            }
            CostPart::RemoveCounterSelf { n, .. } => {
                Phrase::CostRemoveCounters.fill(lang, &[&n.to_string()])
            }
            CostPart::RemoveCounterSelfX { .. } => {
                Phrase::CostRemoveCountersX.text(lang).to_string()
            }
            CostPart::PutCounterSelf { n: 1, .. } => Phrase::CostPutCounter.text(lang).to_string(),
            CostPart::PutCounterSelf { n, .. } => {
                Phrase::CostPutCounters.fill(lang, &[&n.to_string()])
            }
        });
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
/// for — and [`cost_label`] writes it as one line because that is how a card
/// prints it. Drawn as a narrow column beside a sentence it wants to be a
/// list again: a comma that wrapped onto a line of its own is the shape that
/// made this necessary, and `{2}{U}{U}` over `{T}` over `Sacrifice this` is
/// how a player reads what an ability charges anyway.
///
/// The inverse of one `join`, which is why the two are written together here
/// rather than each where it is used.
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

#[cfg(test)]
mod tests {
    use super::*;
    use baylee_client_core::test_support::{ViewBuilder, token};
    use baylee_core::ids::PlayerId;
    use baylee_engine::choice::{GRANTED_ABILITY, LegalActions, PREPARED_CAST, Pending};

    fn offering(abilities: Vec<(ObjectId, u32)>, mana: Vec<ObjectId>) -> Interaction {
        Interaction::new(
            Pending::Priority {
                player: PlayerId::new(0),
                legal: Box::new(LegalActions {
                    abilities,
                    mana_abilities: mana,
                    ..LegalActions::default()
                }),
            },
            PlayerId::new(0),
        )
    }

    /// The two synthetic indices are offered like any other ability and have
    /// to be labelled without asking the card about them.
    ///
    /// `GRANTED_ABILITY` is `u32::MAX`, and the fallback label counts an
    /// ability out as `index + 1` — so a permanent under a Chromatic Lantern
    /// killed the client outright in a debug build, on the frame the chooser
    /// was built. Nothing in the duel-flow ledger reaches this: it drives
    /// `Interaction`, not the list of buttons drawn from it.
    #[test]
    fn a_granted_ability_is_labelled_without_asking_the_card_it_is_not_on() {
        let id = ObjectId::new(1, 0);
        let view = ViewBuilder::new(2)
            .with_battlefield(0, [token(1, 0, "Ally", 2, 2)])
            .build();

        // Granted, the view has nothing to say about what it makes — and the
        // engine says it makes mana, so it is one tap, not arm-then-act.
        // That fallback is the only reading left when a grant cannot be
        // reduced to "n mana of these colours", and it is sound here because
        // the permanent is offering exactly one grant for it to be about.
        let i = offering(vec![(id, GRANTED_ABILITY)], vec![id]);
        let out = options(Lang::En, &view, &i, id);
        assert_eq!(out.len(), 1, "one granted ability, one button: {out:?}");
        assert_eq!(out[0].label, "Granted ability");
        assert!(out[0].mana, "the engine offered it as a mana ability");
        assert_eq!(
            out[0].action,
            PlayerAction::ActivateAbility {
                source: id,
                ability_index: GRANTED_ABILITY,
            }
        );

        // The same ability granted by something that is not a mana source.
        let i = offering(vec![(id, GRANTED_ABILITY)], vec![]);
        assert!(
            !options(Lang::En, &view, &i, id)[0].mana,
            "nothing else can tell the client this, so it must be the offer"
        );

        // A prepared cast is a cast: never a mana ability, and never
        // "Ability 4294967295".
        let i = offering(vec![(id, PREPARED_CAST)], vec![]);
        let out = options(Lang::En, &view, &i, id);
        assert_eq!(out[0].label, "Cast the prepared spell");
        assert!(!out[0].mana);
    }
    /// A Chromatic Lantern's land, as it actually arrives: the engine offers
    /// the grant *and* the view says what it makes. Both halves of the
    /// chooser then have something to say about the same tap, and for one
    /// commit they both said it — the player got two buttons that did the
    /// same thing, one of them labelled "Granted ability".
    #[test]
    fn a_granted_mana_ability_is_one_button_and_not_two() {
        let id = ObjectId::new(1, 0);
        let mut land = token(1, 0, "Mountain", 0, 0);
        land.types = baylee_core::types::TypeSet::LAND;
        land.power = None;
        land.toughness = None;
        land.granted_mana = Some(baylee_view::GrantedMana {
            slot: 0,
            colors: vec![ManaColor::Red],
            amount: 1,
        });
        let view = ViewBuilder::new(2).with_battlefield(0, [land]).build();

        let i = offering(vec![(id, GRANTED_ABILITY)], vec![id]);
        let out = options(Lang::En, &view, &i, id);
        assert_eq!(out.len(), 1, "one tap, one button: {out:?}");
        assert!(out[0].mana);
    }

    /// Urza's Saga is granted two abilities and only one of them makes mana.
    /// Reading "is this a mana ability" off `legal.mana_abilities` — which
    /// holds the *permanent*, not the slot — marked the Construct ability as
    /// one tap, which would have sent it with no arming and no confirmation.
    #[test]
    fn a_second_grant_is_not_a_mana_ability_because_the_first_one_is() {
        let id = ObjectId::new(1, 0);
        let mut saga = token(1, 0, "Urza's Saga", 0, 0);
        saga.types = baylee_core::types::TypeSet::LAND;
        saga.power = None;
        saga.toughness = None;
        saga.granted_mana = Some(baylee_view::GrantedMana {
            slot: 0,
            colors: vec![ManaColor::Colorless],
            amount: 1,
        });
        let view = ViewBuilder::new(2).with_battlefield(0, [saga]).build();

        let i = offering(
            vec![
                (id, baylee_engine::choice::granted_ability(0)),
                (id, baylee_engine::choice::granted_ability(1)),
            ],
            vec![id],
        );
        let out = options(Lang::En, &view, &i, id);
        assert_eq!(out.len(), 2, "two grants, two buttons: {out:?}");
        let construct = out
            .iter()
            .find(|o| {
                o.action
                    == PlayerAction::ActivateAbility {
                        source: id,
                        ability_index: baylee_engine::choice::granted_ability(1),
                    }
            })
            .expect("chapter II is offered");
        assert!(
            !construct.mana,
            "the permanent has a granted mana ability; this is not it"
        );
    }

    /// The same fallback with two grants on the permanent, which is where it
    /// stops being sound: `legal.mana_abilities` names the permanent, so it
    /// cannot say *which* of them makes the mana. Reading it anyway would
    /// have fired the other one on a single tap, with no arming and no way
    /// back — and arming a mana ability by mistake only ever costs a tap.
    #[test]
    fn an_unreadable_grant_is_not_a_mana_ability_when_there_are_two_of_them() {
        let id = ObjectId::new(1, 0);
        let view = ViewBuilder::new(2)
            .with_battlefield(0, [token(1, 0, "Ally", 2, 2)])
            .build();
        let i = offering(
            vec![
                (id, baylee_engine::choice::granted_ability(0)),
                (id, baylee_engine::choice::granted_ability(1)),
            ],
            vec![id],
        );
        let out = options(Lang::En, &view, &i, id);
        assert_eq!(out.len(), 2, "two grants, two buttons: {out:?}");
        assert!(
            out.iter().all(|o| !o.mana),
            "neither can be claimed as the mana one: {out:?}"
        );
    }

    /// The other way that fallback goes wrong, and the one a real card
    /// reaches first: a *basic land* is named in `legal.mana_abilities` for
    /// its own intrinsic mana (CR 305.6), whatever it was granted. Counting
    /// that entry as evidence about the grant marks a non-mana grant as one
    /// tap, and it fires with no arming.
    ///
    /// No card in the pool grants a land a non-mana ability today — Lantern,
    /// Saga and Guide all grant mana — so this guards the reading rather
    /// than a card, which is why it is written from both sides.
    #[test]
    fn a_basics_own_mana_says_nothing_about_a_grant_that_cannot_be_read() {
        use baylee_core::generated::subtypes::land;
        let id = ObjectId::new(1, 0);
        let mut mountain = token(1, 0, "Mountain", 0, 0);
        mountain.types = baylee_core::types::TypeSet::LAND;
        mountain.subtypes = baylee_core::types::SubtypeSet::from_slice(&[land::MOUNTAIN]);
        mountain.power = None;
        mountain.toughness = None;
        // The view could not reduce the grant to "n mana of these colours".
        mountain.granted_mana = None;
        let view = ViewBuilder::new(2).with_battlefield(0, [mountain]).build();

        // The land really does have two things to do — its own `{R}` and the
        // grant — so both are drawn. What matters is which of them is one
        // tap: the intrinsic is, the unreadable grant is not.
        let granted = |legal: Vec<ObjectId>| {
            let i = offering(vec![(id, GRANTED_ABILITY)], legal);
            let out = options(Lang::En, &view, &i, id);
            assert_eq!(out.len(), 2, "the intrinsic and the grant: {out:?}");
            assert!(
                out.iter().any(|o| o.mana
                    && o.action == PlayerAction::ActivateManaAbility { source: id }),
                "the Mountain still taps for `{{R}}`: {out:?}"
            );
            out.into_iter()
                .find(|o| {
                    o.action
                        == PlayerAction::ActivateAbility {
                            source: id,
                            ability_index: GRANTED_ABILITY,
                        }
                })
                .expect("the grant is offered")
        };

        // Named once, and its own `{R}` is what that entry is.
        assert!(
            !granted(vec![id]).mana,
            "the Mountain's own mana is not evidence about the grant"
        );

        // Named twice: its own, and a granted one. Now there is an entry
        // left over and exactly one grant it can be about.
        assert!(
            granted(vec![id, id]).mana,
            "one entry over and one grant to be about"
        );
    }

    /// The card the whole restricted-mana path was reported broken on.
    ///
    /// The three shapes a mana choice is written in, and why there are three.
    ///
    /// The owner reported "Tap for WUBRG" as ugly, and it was two faults in
    /// one string: bare letters where the interface draws symbols everywhere
    /// else, and five of them in a row where the card itself says three
    /// words. A short choice is the case that keeps its symbols — two discs
    /// and a conjunction read at a glance.
    #[test]
    fn a_mana_choice_is_written_as_symbols_until_it_is_every_colour() {
        use baylee_core::mana::ManaColor::{Black, Blue, Green, Red, White};
        assert_eq!(mana_choice(Lang::En, &[Green], 1), "{G}");
        assert_eq!(
            mana_choice(Lang::En, &[Green], 3),
            "{G}{G}{G}",
            "a source that makes three of one colour says so three times",
        );
        assert_eq!(mana_choice(Lang::En, &[Blue, Black], 1), "{U} or {B}");
        assert_eq!(
            mana_choice(Lang::En, &[White, Blue, Black], 1),
            "{W}, {U} or {B}",
        );
        assert_eq!(
            mana_choice(Lang::En, &[White, Blue, Black, Red, Green], 1),
            "any color",
            "five discs is not a symbol, it is arithmetic",
        );
        assert_eq!(
            mana_choice(Lang::De, &[White, Blue, Black, Red, Green], 1),
            "beliebige Farbe",
        );
        assert_eq!(mana_choice(Lang::De, &[Blue, Black], 1), "{U} oder {B}");
    }

    /// Jasmine Dragon Tea Shop prints two mana abilities: `{T}: Add {C}` and
    /// `{T}: Add one mana of any color`, the second spendable only on Allies.
    /// `manasources` reduces a permanent to the one tap it can read and
    /// `simple_mana` refuses restricted mana on purpose, so the first won the
    /// mana slot and the second fell through to the cost label — which is
    /// `{T}`, the same tap, saying nothing about what it makes. Two buttons,
    /// one of them unreadable, and the unreadable one is the reason the land
    /// is in the deck.
    ///
    /// The partition in [`pour_out`] changed the *shape* of the answer and
    /// not the claim: the `{C}` is a header pip now, because a pip is exactly
    /// what a colour with a number behind it wants, and the restricted tap
    /// keeps its sentence because a pip cannot say "and only on Ally spells".
    /// Still two offers, still tellable apart.
    #[test]
    fn a_restricted_mana_ability_says_what_it_makes() {
        let id = ObjectId::new(1, 0);
        let mut land = crate::registry_printed(1, 0, "Jasmine Dragon Tea Shop");
        land.types = baylee_core::types::TypeSet::LAND;
        land.power = None;
        land.toughness = None;
        let view = ViewBuilder::new(2).with_battlefield(0, [land]).build();

        let i = offering(vec![(id, 0), (id, 1)], vec![]);
        let out = options(Lang::En, &view, &i, id);
        let labels: Vec<&str> = out.iter().map(|o| o.label.as_str()).collect();
        assert_eq!(
            labels,
            vec!["{C}", "Tap for any color (restricted)"],
            "both taps have to be tellable apart: {out:?}"
        );
        assert!(
            out[0].pour.is_some(),
            "the colourless tap is the header pip"
        );
        assert!(
            out[1].pour.is_none(),
            "and the restricted one is a written row, because a pip cannot \
             carry what it may be spent on"
        );
        assert_eq!(Split::of(&out), Split { pips: 1, rows: 1 });
        assert!(
            out.iter().all(|o| o.mana),
            "both are mana abilities (CR 605.1), so both stay one tap"
        );
        assert_eq!(
            out[1].action,
            PlayerAction::ActivateAbility {
                source: id,
                ability_index: 1,
            },
            "and the second button sends the second ability"
        );
    }
}
