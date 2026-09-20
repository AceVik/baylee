//! What the seat can tap for mana, read off the choice the engine offered.
//!
//! [`baylee_client_core::manaplan`] decides *which* sources to tap; this is
//! the half that cannot live there, because knowing that ability 2 of a
//! Command Tower makes mana takes the compiled card registry, and
//! `baylee-client-core` deliberately does not link it.
//!
//! The list is built from `LegalActions` and nothing else, so a source that
//! is tapped, summoning sick, or otherwise unavailable never appears — the
//! engine already answered that question and this does not second-guess it.
//! What the registry adds is only *what comes out*.

use baylee_client_core::manaplan::{Source, Tap, basic_land_color};
use baylee_core::mana::ManaCost;
use baylee_engine::choice::LegalActions;
use baylee_view::PlayerView;

use baylee_cards_dsl::{AbilityDef, CostPart};

/// Every mana source the seat may tap right now.
///
/// **One entry per permanent**, which is the whole subtlety here. A Forest
/// appears twice in `LegalActions` — once in `mana_abilities` as the CR 305.6
/// shortcut and once in `abilities` as the `{T}: Add {G}` printed on the card
/// — and it can still only be tapped once. Two entries would let the planner
/// pay `{G}{G}` with one Forest, which is a plan the engine refuses after the
/// land is already tapped.
///
/// **The consequence is that the expensive mode of a permanent offering two
/// is out of reach**, and that is a bargain rather than an oversight. One
/// entry per permanent means the comparator picks a mode and the planner
/// never sees the other, so a card printing a free tap beside a priced one is
/// planned as the free one only: Havenwood Battleground is never planned for
/// `{G}{G}`, and Vivid Crag is a red source and never a blue one. **Twenty-five
/// faces in the pool are in that position**, 7 losing an amount and 18 a
/// colour. Before #165 the same one-entry rule pointed the other way and was
/// worse — all 25 *always* paid — and the two are not symmetric: a shy
/// planner costs a player some clicks, an over-eager one strands a
/// half-tapped board mid-cast with no way back. Choosing per cost instead of
/// per permanent needs `Source` to carry modes and `manaplan::assign` to pick
/// one of them, and
/// `the_expensive_mode_is_out_of_reach_and_that_is_the_bargain` is the test
/// that goes red the day it can.
#[must_use]
pub fn sources(view: &PlayerView, legal: &LegalActions) -> Vec<Source> {
    let mut sources = Vec::new();

    // The CR 305.6 shortcut. The engine offers it only for a land with
    // exactly one basic type, and the colour follows from the *projected*
    // subtypes — an animated Dryad Arbor still taps for green.
    for &id in &legal.mana_abilities {
        let Some(object) = view.battlefield.iter().find(|o| o.id == id) else {
            continue;
        };
        if let Some(color) = basic_land_color(&object.subtypes) {
            sources.push(Source::fixed(id, Tap::Intrinsic, color));
        }
    }

    // Printed mana abilities: Command Tower, a Llanowar Elf, a Sol Ring —
    // and the one that is printed on no card, which the view carries instead.
    for &(id, index) in &legal.abilities {
        let source = match baylee_engine::choice::granted_slot(index) {
            Some(slot) => granted_source(view, id, slot),
            None => printed_source(view, id, index),
        };
        if let Some(source) = source {
            sources.push(source);
        }
    }

    // A permanent taps once, so it is one source: the cheapest of what it
    // offered, and the roomiest of those.
    //
    // **Price outranks everything else**, which is the whole of #165. "Best"
    // used to mean more mana and then more colours, and what the tap *cost*
    // was not in the comparison at all — `mana_shape` reads `cost.mana` and
    // never looks at `cost.parts`, so `{T}, Sacrifice this land: Add {G}{G}`
    // is a shape this client accepts, and it also wins on amount. The
    // planner sacrificed Havenwood Battleground every time it tapped it.
    // Measured over the pool on 20.09.2026: 61 readable mana abilities carry
    // a cost part beyond the tap, 32 faces offer both a free and a priced
    // one, and on 25 of those the priced one won.
    //
    // Below it the old order stands unchanged: more mana, then more colours,
    // and the intrinsic shortcut wins a tie because it costs one fewer round
    // trip and is never asked for a colour.
    let mut ranked: Vec<(bool, Source)> = sources
        .into_iter()
        .map(|source| (priced(view, &source), source))
        .collect();
    ranked.sort_by(|(a_priced, a), (b_priced, b)| {
        a.id.cmp(&b.id)
            .then_with(|| a_priced.cmp(b_priced))
            .then_with(|| b.amount.cmp(&a.amount))
            .then_with(|| b.colors.len().cmp(&a.colors.len()))
            .then_with(|| matches!(a.tap, Tap::Ability(_)).cmp(&matches!(b.tap, Tap::Ability(_))))
    });
    ranked.dedup_by(|(_, a), (_, b)| a.id == b.id);
    ranked.into_iter().map(|(_, source)| source).collect()
}

/// Whether tapping this source costs anything **beyond** the tap.
///
/// Deliberately not a sixth reader in [`baylee_cards_dsl`]'s family: all five
/// of those answer what comes *out* of an ability and every one of them is
/// about the `AddMana`. This asks what goes *in*, and it is a planner's
/// question — a label and a mana bubble do not care what a tap costs. It is
/// in this module for the reason the module exists: reading a cost off a card
/// takes the compiled registry.
///
/// **A positive list of exactly one.** Anything that is not
/// [`CostPart::TapSelf`] is a price, so a `CostPart` added tomorrow is priced
/// without anyone remembering this function. Listing what counts as expensive
/// instead would go silent on the next variant, and silent here means free.
fn priced(view: &PlayerView, source: &Source) -> bool {
    let Tap::Ability(index) = source.tap else {
        // The CR 305.6 shortcut. Tapping is the whole of it.
        return false;
    };
    if baylee_engine::choice::granted_slot(index).is_some() {
        // An ability a continuous effect granted is printed on no card, and
        // `GrantedMana` carries colours and an amount and **no cost** — so
        // free is the only thing this can say, and it is right for the card
        // that grants them: a Chromatic Lantern's is `{T}` and nothing more.
        // Saying otherwise would be worse than imprecise. A Lantern'd
        // Mountain offers the intrinsic tap *and* the grant, and calling the
        // grant priced would hand the dedup to the intrinsic and leave the
        // land making red only, which is the Lantern's whole point undone.
        return false;
    }
    let Some(
        AbilityDef::Activated { cost, effects, .. }
        | AbilityDef::ActivatedConditional { cost, effects, .. },
    ) = ability_at(view, source.id, index)
    else {
        // Unreadable, so assume it costs something. That sorts it behind any
        // tap this client can read, and where it is the only offer the dedup
        // keeps it regardless — so the guess is never the reason a source
        // disappears.
        return true;
    };
    // The two halves of a price, and they sit in different places on the
    // card. One is written in the cost — a land sacrificed, a counter
    // removed, a life paid. The other is written in the effects, beside the
    // mana: Adarkar Wastes charges nothing to tap and deals you a damage for
    // the privilege. Both are reasons to reach for a different land first,
    // and neither is visible to the other.
    let costs_more_than_a_tap = cost
        .parts
        .iter()
        .any(|part| !matches!(part, CostPart::TapSelf));
    // `mana_with_riders` accepted this source, so exactly one of its effects
    // is the `AddMana` and anything else is a rider.
    let does_more_than_add = effects.len() > 1;
    costs_more_than_a_tap || does_more_than_add
}

/// Ability `index` of `object`, out of the registry.
///
/// The face matters: an MDFC's back has its own abilities, and reading the
/// front's list for it would name the wrong one.
#[must_use]
pub fn ability_at(
    view: &PlayerView,
    object: baylee_core::ids::ObjectId,
    index: u32,
) -> Option<&'static AbilityDef> {
    let card = view.object(object)?.card?;
    let def = baylee_cards::by_index(card.index)?;
    let abilities = def.abilities_for_face(card.face as usize);
    abilities.get(usize::try_from(index).ok()?)
}

/// Whether this printed ability is the **same button** as the CR 305.6
/// shortcut the engine already offered for this permanent.
///
/// The ability chooser's question, and it is not the planner's. A Forest
/// prints `{T}: Add {G}` and the engine offers the shortcut for the same tap,
/// so listing both is listing one button twice — that is the whole of what
/// this is for, and it is why it compares what comes *out* rather than
/// trusting that anything readable must be a duplicate.
///
/// Asking "is this readable as a source" instead was a defect with two faces.
/// A ridered tap became readable with #149, so Yavimaya Coast's `{T}: Add
/// {G} or {U}. This land deals 1 damage to you.` was struck off the menu the
/// day the planner learned to read it — and the planner will not choose it
/// either, because `priced` ranks it behind the clean tap, so the land's
/// coloured half was reachable by no route at all. The same sentence had been
/// quietly true of every priced tap since #165: Havenwood Battleground's
/// `{T}, Sacrifice this land: Add {G}{G}` is read, so it was struck off, and
/// once the clean tap won the dedup nothing offered it either.
///
/// So the strict reader is the right one here and the relaxed one is not.
/// CR 305.6 mana is a free, single-effect, one-colour tap by definition, so
/// an ability that is anything else cannot be the shortcut wearing a card's
/// clothes.
#[must_use]
pub fn duplicates_intrinsic(
    view: &PlayerView,
    object: baylee_core::ids::ObjectId,
    index: u32,
) -> bool {
    let Some(intrinsic) = view
        .object(object)
        .and_then(|o| basic_land_color(&o.subtypes))
    else {
        // No basic land type, so there is no shortcut to duplicate.
        return false;
    };
    let Some(ability) = ability_at(view, object, index) else {
        return false;
    };
    let (AbilityDef::Activated {
        cost,
        effects,
        mana_ability: true,
        ..
    }
    | AbilityDef::ActivatedConditional {
        cost,
        effects,
        mana_ability: true,
        ..
    }) = ability
    else {
        return false;
    };
    // The strict door on purpose: a tap that also does something else is a
    // different button from the shortcut, whatever mana it happens to make.
    let Some((source, amount, restricted)) = baylee_cards_dsl::mana_shape(cost, effects) else {
        return false;
    };
    if restricted || amount != Some(1) {
        return false;
    }
    produced_colors(view, object, index, source).as_deref() == Some(&[intrinsic][..])
}

/// The source that ability `index` of `object` is, when it is one this client
/// can read.
///
/// Also the answer to "is this a mana ability" for the ability chooser, which
/// has to leave them out: a Forest's printed `{T}: Add {G}` is the same tap as
/// the CR 305.6 shortcut, and offering both is offering the same button twice.
#[must_use]
pub fn printed_source(
    view: &PlayerView,
    object: baylee_core::ids::ObjectId,
    index: u32,
) -> Option<Source> {
    mana_ability(view, object, index, ability_at(view, object, index)?)
}

/// Reads one ability as a mana source, or decides it is not one this can use.
///
/// The reading itself is [`baylee_cards_dsl::mana_with_riders`], which is where it
/// has to live: the same question is asked of an ability a continuous effect
/// *grants*, and that one is not printed on any card, so this module cannot
/// be the one that knows the answer. What the shape leaves open is which
/// colours a source that depends on the board makes, and that is this
/// module's to answer — it is the one with a [`PlayerView`] in its hand.
fn mana_ability(
    view: &PlayerView,
    id: baylee_core::ids::ObjectId,
    index: u32,
    ability: &'static AbilityDef,
) -> Option<Source> {
    // `ActivatedConditional` belongs here beside `Activated`, and the
    // condition is deliberately not re-checked: this list is built from
    // `LegalActions` and nothing else, and the engine offers a conditional
    // ability only once its condition holds. Reading the first variant alone
    // meant a permanent whose *only* mana ability has a condition on it
    // counted for nothing — Mox Opal is exactly that card, so a player with
    // metalcraft up had the planner tap around a mana it was being offered.
    let (AbilityDef::Activated {
        cost,
        effects,
        mana_ability: true,
        ..
    }
    | AbilityDef::ActivatedConditional {
        cost,
        effects,
        mana_ability: true,
        ..
    }) = ability
    else {
        return None;
    };
    // The planner's door rather than the strict one: an ability that also
    // does something else is still a source, and `priced` is what keeps it
    // behind every clean tap. See the module header in `manaread.rs` for why
    // the bar moves for this caller and for no other.
    let (source, amount, restricted) = baylee_cards_dsl::mana_with_riders(cost, effects)?;
    // Still refused, and for the reason `simple_mana` refused it: what a
    // Cavern of Souls' mana may be spent on is a rules question, and
    // answering it this side of the wire is the guess the reading exists to
    // avoid. Path of Ancestry is the one in the owner's deck.
    if restricted {
        return None;
    }
    // And an amount only the board can count is refused here for the reason
    // the shape stopped short of naming one: a plan that guessed at Harabaz
    // Druid's X would leave a board half tapped. A bubble asks a smaller
    // question and takes it — see [`offers`].
    let amount = amount?;
    let colors = produced_colors(view, id, index, source)?;
    Some(Source {
        id,
        tap: Tap::Ability(index),
        colors,
        amount,
    })
}

/// Which colours a read mana source actually makes, at this board.
///
/// The half of the reading that needs a game. Two of the four sources have no
/// answer without one, which is why [`baylee_cards_dsl::mana_shape`] stops
/// one step earlier and hands the `ManaSource` back for a caller holding a
/// [`PlayerView`] to finish.
///
/// Those two are finished by **reading the answer**, not by working it out.
/// Reflecting Pool, Exotic Orchard and Fellwar Stone want the union of
/// `produced_colors` over the lands of one side of the table, and that is a
/// *projected* characteristic — a Chromatic Lantern's grant is in it and so
/// is a land that has lost its abilities — which no registry carries and no
/// client can compute. So `baylee-gamehost` resolves both through the
/// engine's own `resolve::colors_of` and puts the result on the permanent
/// (`PublicObject::board_mana`), exactly as it does for a granted ability.
/// Before that existed this arm answered `None` and a Reflecting Pool was
/// dead mana: it counted for nothing in a plan, lit up for nothing in hand,
/// and could not be tapped by hand either.
///
/// `object` and `index` are what turn the view's answer back into *this*
/// ability's: the host named which printed ability it resolved, and a
/// permanent with a second mana ability beside it must not read the first
/// one's colours onto the second's row.
pub(crate) fn produced_colors(
    view: &PlayerView,
    object: baylee_core::ids::ObjectId,
    index: u32,
    source: baylee_cards_dsl::ManaSource,
) -> Option<Vec<baylee_core::mana::ManaColor>> {
    match source {
        baylee_cards_dsl::ManaSource::Fixed(color) => Some(vec![color]),
        baylee_cards_dsl::ManaSource::Choice(colors) => Some(colors.to_vec()),
        // All four are the host's to answer, and the chosen colour is the
        // one the client could not have derived with the whole board in
        // front of it: the answer is on the object, printed on no card.
        baylee_cards_dsl::ManaSource::CommanderIdentity
        | baylee_cards_dsl::ManaSource::LandColor { .. }
        | baylee_cards_dsl::ManaSource::Chosen
        | baylee_cards_dsl::ManaSource::ChosenOr(_) => {
            let board = view.object(object)?.board_mana.as_ref()?;
            (board.index == index).then(|| board.colors.clone())
        }
    }
}

/// Every tap `object` has for mana, **one per ability** rather than one per
/// permanent.
///
/// The mirror image of [`sources`], and the two differences are both the
/// mana bubble's doing.
///
/// It does **not** dedupe. `sources` reduces a permanent to the one tap a
/// plan may spend, because two entries would let the planner pay `{G}{G}`
/// with one Forest. A bubble has the opposite problem: a Plains under an
/// effect granting it any colour makes `{W}` without asking and the other
/// four by asking, and a list that kept only the grant would put the player
/// through a colour prompt to get the white the land already prints.
/// [`baylee_client_core::manaplan::pours`] does the reducing instead, and it
/// reduces per *colour*.
///
/// And it reads each ability through [`baylee_cards_dsl::mana_offer`], which
/// asks only what colours are on offer — see there for why a bubble may draw
/// two abilities a planner must refuse, and why it is told whether the amount
/// behind one of them is a number.
#[must_use]
pub fn offers(
    view: &PlayerView,
    legal: &LegalActions,
    object: baylee_core::ids::ObjectId,
) -> Vec<baylee_client_core::manaplan::Offer> {
    use baylee_client_core::manaplan::Offer;
    let mut out = Vec::new();
    if legal.mana_abilities.contains(&object)
        && let Some(permanent) = view.battlefield.iter().find(|o| o.id == object)
        && let Some(color) = basic_land_color(&permanent.subtypes)
    {
        out.push(Offer {
            tap: Tap::Intrinsic,
            colors: vec![color],
            // One mana of the land's own colour, by CR 305.6 and by nothing
            // else: there is no card text to read and no amount to doubt.
            fixed: true,
        });
    }
    for &(id, index) in &legal.abilities {
        if id != object {
            continue;
        }
        let read = match baylee_engine::choice::granted_slot(index) {
            // A grant is printed on no card, so there is no ability to read:
            // the host has already reduced it to colours *and* a count, which
            // is why a grant is always a number here.
            Some(slot) => granted_source(view, object, slot).map(|source| (source.colors, true)),
            None => match ability_at(view, object, index) {
                Some(
                    AbilityDef::Activated {
                        cost,
                        effects,
                        mana_ability: true,
                        ..
                    }
                    | AbilityDef::ActivatedConditional {
                        cost,
                        effects,
                        mana_ability: true,
                        ..
                    },
                ) => baylee_cards_dsl::mana_offer(cost, effects).and_then(|(source, amount)| {
                    Some((
                        produced_colors(view, object, index, source)?,
                        amount.is_some(),
                    ))
                }),
                _ => None,
            },
        };
        if let Some((colors, fixed)) = read.filter(|(colors, _)| !colors.is_empty()) {
            out.push(Offer {
                tap: Tap::Ability(index),
                colors,
                fixed,
            });
        }
    }
    out
}

/// Whether one press of this tap pours a number this side of the wire can
/// name.
///
/// The `fixed` of [`offers`], asked of one tap and without a `LegalActions`
/// in hand — because its two callers ask *after* the pips are built: what
/// prefix a bubble carries, and whether a written mana row opens a bubble of
/// its own rather than being sent.
///
/// Everything but a printed ability answers yes. The CR 305.6 shortcut is one
/// mana of the land's own colour and there is no text to doubt; a grant has
/// already been reduced to colours and a count by the host.
pub(crate) fn countable(view: &PlayerView, object: baylee_core::ids::ObjectId, tap: Tap) -> bool {
    let Tap::Ability(index) = tap else {
        return true;
    };
    if baylee_engine::choice::granted_slot(index).is_some() {
        return true;
    }
    match ability_at(view, object, index) {
        Some(
            AbilityDef::Activated {
                cost,
                effects,
                mana_ability: true,
                ..
            }
            | AbilityDef::ActivatedConditional {
                cost,
                effects,
                mana_ability: true,
                ..
            },
        ) => baylee_cards_dsl::mana_offer(cost, effects).is_none_or(|(_, amount)| amount.is_some()),
        // Not a mana ability at all, or a card this client cannot read.
        // Being unable to say "this is X" is not the same as saying it is.
        _ => true,
    }
}

/// The mana a continuous effect grants `object` the ability to make.
///
/// Read off the view rather than the registry, because there is nothing in
/// the registry to read: a land under a Chromatic Lantern has an ability its
/// printed card does not mention. `baylee-gamehost` projects what it makes
/// (see `PublicObject::granted_mana`) precisely so this client can plan
/// through it instead of leaving the player to tap those lands by hand.
///
/// `slot` is which granted ability is being asked about, and the answer is
/// `None` for every slot but the one the view named: Urza's Saga is granted
/// two abilities and only one of them makes mana. Saying otherwise would tap
/// the permanent for a Construct and then try to pay a spell with it.
#[must_use]
pub fn granted_source(
    view: &PlayerView,
    object: baylee_core::ids::ObjectId,
    slot: u32,
) -> Option<Source> {
    let granted = view.object(object)?.granted_mana.as_ref()?;
    if granted.slot != slot {
        return None;
    }
    Some(Source {
        id: object,
        tap: Tap::Ability(baylee_engine::choice::granted_ability(slot)),
        colors: granted.colors.clone(),
        amount: granted.amount,
    })
}

/// How many abilities the face this object is showing prints.
///
/// The bound [`table_mana`] walks, rather than a number chosen by hand: a
/// scan that stopped at eight would miss the ninth ability of whatever card
/// eventually has one, and it would miss it in silence.
#[must_use]
pub fn ability_count(view: &PlayerView, object: baylee_core::ids::ObjectId) -> usize {
    let Some(card) = view.object(object).and_then(|o| o.card) else {
        return 0;
    };
    let Some(def) = baylee_cards::by_index(card.index) else {
        return 0;
    };
    def.abilities_for_face(card.face as usize).len()
}

/// The five basic land types and the mana CR 305.6 gives them.
///
/// Deliberately **not** [`basic_land_color`], which answers `None` for a
/// land with two basic types because a planner has to know which single
/// colour a tap produces. The hearth is asking a different question — what
/// colours are standing on this table — and a Taiga is an honest answer of
/// two.
const BASIC_MANA: [(baylee_core::ids::SubtypeId, baylee_core::mana::ManaColor); 5] = {
    use baylee_core::generated::subtypes::land;
    use baylee_core::mana::ManaColor;
    [
        (land::PLAINS, ManaColor::White),
        (land::ISLAND, ManaColor::Blue),
        (land::SWAMP, ManaColor::Black),
        (land::MOUNTAIN, ManaColor::Red),
        (land::FOREST, ManaColor::Green),
    ]
};

/// How much of each colour of mana stands on the whole table.
///
/// Indexed the way [`ManaColor`] is numbered and `tabletop::PIE` is ordered
/// — white, blue, black, red, green — so it feeds the five flames at the
/// middle of the table straight off.
///
/// **A count of permanents, not of mana.** A Sol Ring makes two and a
/// Nykthos makes as many as your devotion; neither is a colour, and a tally
/// that summed amounts would be answering "how big is the biggest turn
/// possible here" rather than "what is this table made of". So each
/// permanent that can make coloured mana is worth exactly **one**, split
/// between the colours it may make: a Forest is a whole green, a Command
/// Tower is a fifth of each. Splitting is the honest reading of a colour
/// nobody has chosen yet — a Command Tower that lit all five fires at full
/// height would say there is a table's worth of white here when there is
/// one land.
///
/// **Presence, not availability.** A tapped land counts and so does a
/// summoning-sick dork. The question is what colours are at this table, and
/// a fire that went out every time its controller spent their mana would be
/// a second, noisier turn indicator — which the rail already is. It is also
/// what makes the reading cheap: no `LegalActions`, so the same walk answers
/// for every seat's permanents and not only for the one holding priority.
///
/// Two holes, named rather than papered over. A **token** has no `card`, so
/// a Treasure counts for nothing — which is wrong and is the same gap
/// `board::provenance_of` works around by name; closing it wants the token
/// registry and a reason better than this one. And a **face-down**
/// permanent counts for nothing, which is right: nobody at the table knows
/// what it makes.
#[must_use]
pub fn table_mana(view: &PlayerView) -> [f32; 5] {
    let mut tally = [0.0_f32; 5];
    for object in &view.battlefield {
        // One permanent, one union of colours, counted once. Building the
        // set first is what stops a land being counted twice — a printed
        // `{T}: Add {G}` beside the type line's own green — and it is the
        // same rule `sources` enforces by deduplicating on `id`.
        let mut colors = [false; 5];
        if !object.status.is_face_down() {
            for (subtype, color) in BASIC_MANA {
                if object.subtypes.contains(subtype) {
                    colors[color as usize] = true;
                }
            }
        }
        if let Some(granted) = &object.granted_mana {
            for color in &granted.colors {
                if let Some(slot) = colors.get_mut(*color as usize) {
                    *slot = true;
                }
            }
        }
        for index in 0..ability_count(view, object.id) {
            let Ok(index) = u32::try_from(index) else {
                continue;
            };
            let Some(source) = printed_source(view, object.id, index) else {
                continue;
            };
            for color in source.colors {
                if let Some(slot) = colors.get_mut(color as usize) {
                    *slot = true;
                }
            }
        }

        let made = colors.iter().filter(|on| **on).count();
        if made == 0 {
            continue;
        }
        #[allow(clippy::cast_precision_loss)] // five at the most
        let share = 1.0 / made as f32;
        for (slot, on) in tally.iter_mut().zip(colors) {
            if on {
                *slot += share;
            }
        }
    }
    tally
}

/// The printed cost of a card in hand.
#[must_use]
pub fn hand_cost(card: &baylee_view::HandObject) -> Option<ManaCost> {
    let def = baylee_cards::by_index(card.card.index)?;
    let face = def
        .faces
        .get(card.card.face as usize)
        .or(def.faces.first())?;
    Some(face.mana_cost)
}

#[cfg(test)]
mod tests {
    use super::*;
    use baylee_client_core::test_support::{ViewBuilder, token};
    use baylee_core::ids::ObjectId;
    use baylee_core::mana::ManaColor;
    use baylee_engine::choice::GRANTED_ABILITY;

    /// A land under a Chromatic Lantern, as the seat is shown it: a Mountain
    /// with an ability that is on no card, and the engine offering it.
    fn lantern_land() -> (PlayerView, LegalActions) {
        let id = ObjectId::new(7, 0);
        let mut land = token(7, 0, "Mountain", 0, 0);
        land.types = baylee_core::types::TypeSet::LAND;
        land.power = None;
        land.toughness = None;
        land.granted_mana = Some(baylee_view::GrantedMana {
            slot: 0,
            colors: vec![
                ManaColor::White,
                ManaColor::Blue,
                ManaColor::Black,
                ManaColor::Red,
                ManaColor::Green,
            ],
            amount: 1,
        });
        let view = ViewBuilder::new(2).with_battlefield(0, [land]).build();
        let legal = LegalActions {
            abilities: vec![(id, GRANTED_ABILITY)],
            mana_abilities: vec![id],
            ..LegalActions::default()
        };
        (view, legal)
    }

    /// A real card off the registry, so the ability walk has something to
    /// read.
    fn card(slot: u32, controller: u8, name: &str) -> baylee_view::PublicObject {
        let index = baylee_cards::decks::by_name(name)
            .unwrap_or_else(|| panic!("{name} is in the registry"));
        let def = baylee_cards::by_index(index).expect("a card at that index");
        let mut obj = baylee_client_core::test_support::printed(slot, controller, name, 0);
        obj.card = Some(baylee_view::CardIdentity {
            index,
            print: baylee_core::ids::PrintRef::new(0),
            face: 0,
        });
        obj.subtypes = baylee_core::types::SubtypeSet::from_slice(def.faces[0].subtypes);
        obj.types = def.faces[0].types;
        obj
    }

    /// A land with two basic types feeds both its fires, and each gets
    /// **half** of it.
    ///
    /// Which is the split rule doing the thing it is for: a Taiga taps once
    /// and the mana is red *or* green. Counted whole on both, five Taigas
    /// would say this table holds ten sources of coloured mana, and the two
    /// tallest flames on it would be as tall as ten Forests.
    ///
    /// `basic_land_color` answers `None` here, which is why [`BASIC_MANA`]
    /// exists beside it: a planner has to know which single colour a tap
    /// gives and the hearth has to know which colours are on the table, and
    /// those are different questions about the same land.
    #[test]
    fn a_dual_land_feeds_both_its_fires_and_each_gets_half() {
        let view = ViewBuilder::new(2)
            .with_battlefield(0, [card(1, 0, "Taiga")])
            .build();
        let tally = table_mana(&view);
        assert!(
            (tally[ManaColor::Red as usize] - 0.5).abs() < 1e-5,
            "red is {tally:?}"
        );
        assert!(
            (tally[ManaColor::Green as usize] - 0.5).abs() < 1e-5,
            "green is {tally:?}"
        );
        assert!(tally[ManaColor::White as usize].abs() < 1e-5, "{tally:?}");
        // And the land is worth one land, however many fires it feeds.
        assert!(
            (tally.iter().sum::<f32>() - 1.0).abs() < 1e-5,
            "a Taiga came to {tally:?}"
        );
    }

    /// And a permanent that may make any colour is worth one permanent, not
    /// five — which is the whole reason the share is a division.
    #[test]
    fn a_source_of_every_colour_is_split_between_them() {
        let (view, _) = lantern_land();
        let tally = table_mana(&view);
        for (index, share) in tally.iter().enumerate() {
            assert!(
                (share - 0.2).abs() < 1e-5,
                "colour {index} took {share} of a Lantern's land: {tally:?}"
            );
        }
    }

    /// A Forest prints no mana ability — CR 305.6 does — so the two readings
    /// must not both fire. One permanent is one, however it was recognised.
    #[test]
    fn one_permanent_is_counted_once_however_it_was_read() {
        let view = ViewBuilder::new(2)
            .with_battlefield(0, [card(1, 0, "Forest"), card(2, 1, "Forest")])
            .build();
        let tally = table_mana(&view);
        assert!(
            (tally[ManaColor::Green as usize] - 2.0).abs() < 1e-5,
            "two Forests, one each side of the table: {tally:?}"
        );
        // And the reading crosses the table: the second Forest belongs to
        // the opponent, and the hearth is the whole table's fire.
        assert!(tally.iter().sum::<f32>() > 1.5, "{tally:?}");
    }

    /// The gap this closes. `GRANTED_ABILITY` is `u32::MAX`, so the registry
    /// lookup that reads every other ability finds nothing there — a land
    /// under a Lantern counted for zero and the player tapped it by hand.
    #[test]
    fn a_granted_mana_ability_is_a_source_the_planner_can_see() {
        let (view, legal) = lantern_land();
        let sources = sources(&view, &legal);
        assert_eq!(sources.len(), 1, "one permanent, one source");
        let source = &sources[0];
        assert_eq!(source.amount, 1);
        assert_eq!(source.colors.len(), 5, "any colour, as the Lantern says");
        assert_eq!(
            source.tap,
            Tap::Ability(GRANTED_ABILITY),
            "tapped through the handle the engine offered it under"
        );
    }

    /// A real Mountain under the Lantern: both producers fire — the CR 305.6
    /// shortcut says red, the grant says any colour — and the permanent still
    /// taps once. Two entries would let the planner pay `{R}{U}` with one
    /// land, which is a plan the engine refuses after it is already tapped.
    #[test]
    fn a_basic_under_a_lantern_is_one_source_and_it_is_the_better_one() {
        let (mut view, legal) = lantern_land();
        view.battlefield[0]
            .subtypes
            .insert(baylee_core::generated::subtypes::land::MOUNTAIN);

        let sources = sources(&view, &legal);
        assert_eq!(sources.len(), 1, "a land taps once, however many ways");
        assert_eq!(
            sources[0].colors.len(),
            5,
            "the grant wins: the Lantern makes this land strictly better"
        );
    }

    /// The same land with nothing granting it anything. Worth its own test
    /// because the failure it guards is the loud one: a source invented out
    /// of an empty field is a plan the engine refuses halfway through, with
    /// the lands already tapped.
    #[test]
    fn a_land_with_no_grant_offers_nothing() {
        let (mut view, legal) = lantern_land();
        view.battlefield[0].granted_mana = None;
        assert!(sources(&view, &legal).is_empty());
    }

    /// Mox Opal, whose only mana ability is behind metalcraft.
    ///
    /// `registry_printed` looks the index up by name, which is what
    /// makes the ability reading reach the real card.
    fn mox_opal() -> (PlayerView, LegalActions) {
        let id = ObjectId::new(3, 0);
        let mut mox = crate::registry_printed(3, 0, "Mox Opal");
        mox.types = baylee_core::types::TypeSet::ARTIFACT;
        mox.power = None;
        mox.toughness = None;
        let view = ViewBuilder::new(2).with_battlefield(0, [mox]).build();
        let legal = LegalActions {
            abilities: vec![(id, 0)],
            ..LegalActions::default()
        };
        (view, legal)
    }

    /// A Command Tower on the table and the engine offering its one printed
    /// ability, with `commanders` for the viewing seat left to the caller.
    fn command_tower() -> (PlayerView, LegalActions) {
        let id = ObjectId::new(5, 0);
        let mut tower = crate::registry_printed(5, 0, "Command Tower");
        tower.types = baylee_core::types::TypeSet::LAND;
        tower.power = None;
        tower.toughness = None;
        let view = ViewBuilder::new(2).with_battlefield(0, [tower]).build();
        let legal = LegalActions {
            abilities: vec![(id, 0)],
            ..LegalActions::default()
        };
        (view, legal)
    }

    /// What the host projected onto a permanent for ability `index`.
    ///
    /// The colours are the caller's, because that is the point of every test
    /// below: this side of the wire no longer derives them from anything, so
    /// a test that computed them here would be asserting about its own
    /// arithmetic. Whether the projection is *right* is
    /// `baylee-gamehost`'s question, and is asked there against a real board.
    fn projected(colors: &[baylee_core::mana::ManaColor], index: u32) -> baylee_view::BoardMana {
        baylee_view::BoardMana {
            index,
            colors: colors.to_vec(),
        }
    }

    /// The owner's own game: General Tazri's `{W}{U}{B}{R}{G}` ability puts
    /// all five colours in her identity (CR 903.4), so the Tower is the
    /// five-colour deck's only green source — and the planner could not see
    /// it at all, because the reading stopped at "the board knows". The
    /// Harabaz Druid in hand cost `{1}{G}` and the Tower had to be tapped by
    /// hand.
    #[test]
    fn a_command_tower_makes_what_the_host_projected_onto_it() {
        use baylee_core::mana::ManaColor::{Black, Blue, Green, Red, White};
        let (mut view, legal) = command_tower();
        view.battlefield[0].board_mana = Some(projected(&[White, Blue, Black, Red, Green], 0));

        let sources = sources(&view, &legal);
        assert_eq!(sources.len(), 1, "one permanent, one source");
        assert_eq!(sources[0].amount, 1);
        assert_eq!(
            sources[0].colors.len(),
            5,
            "Tazri's activated ability costs one of each"
        );
        assert!(
            sources[0].colors.contains(&Green),
            "the green the Druid needed"
        );
    }

    /// A narrower identity, so the same land makes fewer colours. Worth its
    /// own test because a five-colour answer that happened to be a constant
    /// would pass the one above and would tap the Tower for green in a deck
    /// that has none — which is the plan the engine refuses at the colour
    /// prompt, with the land already tapped.
    ///
    /// Aminatou, the Fateshifter is `{W}{U}{B}` and is the other seat's
    /// commander in the game this was reported from.
    #[test]
    fn the_land_says_what_it_was_told_and_not_the_rainbow() {
        use baylee_core::mana::ManaColor::{Black, Blue, White};
        let (mut view, legal) = command_tower();
        view.battlefield[0].board_mana = Some(projected(&[White, Blue, Black], 0));
        let colors = &sources(&view, &legal)[0].colors;
        assert_eq!(colors, &vec![White, Blue, Black]);
    }

    /// The offline house duel, which is not a commander game at all: the
    /// ability still resolves and colourless is what is left (the engine's
    /// own fallback, which the host projects like any other answer). A
    /// refusal here would be the common case failing.
    #[test]
    fn a_tower_projected_colorless_taps_for_colorless() {
        let (mut view, legal) = command_tower();
        view.battlefield[0].board_mana =
            Some(projected(&[baylee_core::mana::ManaColor::Colorless], 0));
        let sources = sources(&view, &legal);
        assert_eq!(sources.len(), 1);
        assert_eq!(
            sources[0].colors,
            vec![baylee_core::mana::ManaColor::Colorless]
        );
    }

    /// And the opposite answer to the one above, which is the whole reason
    /// they are two tests: no projection at all is a *refusal*. The host
    /// leaves the field empty only when there is nothing to plan with — a
    /// Reflecting Pool beside no lands — and a client that read that as `{C}`
    /// would stall the run at the colour prompt with the land already tapped.
    #[test]
    fn a_land_the_host_projected_nothing_for_is_not_a_source() {
        let (view, legal) = command_tower();
        assert!(view.battlefield[0].board_mana.is_none());
        assert!(sources(&view, &legal).is_empty());
    }

    /// The projection names *which* ability it resolved, and that index is
    /// read rather than trusted. A permanent with a second mana ability
    /// beside the board-dependent one would otherwise wear the first one's
    /// colours on the second one's row — one field on the object, two rows
    /// that could claim it.
    #[test]
    fn a_projection_for_another_ability_is_not_this_ones_colours() {
        let (mut view, legal) = command_tower();
        view.battlefield[0].board_mana = Some(projected(&[baylee_core::mana::ManaColor::Green], 1));
        assert!(
            sources(&view, &legal).is_empty(),
            "the engine offered ability 0 and the host answered about ability 1"
        );
    }

    /// A mana ability with a condition on it is still a mana ability
    /// (CR 605.1), and the engine has already decided the condition holds —
    /// it would not be in `LegalActions` otherwise. Reading only
    /// `AbilityDef::Activated` here counted a Mox Opal for zero, and it is
    /// the whole of what that card does.
    #[test]
    fn a_conditional_mana_ability_is_a_source_the_planner_can_see() {
        let (view, legal) = mox_opal();
        let sources = sources(&view, &legal);
        assert_eq!(sources.len(), 1, "one permanent, one source");
        assert_eq!(sources[0].amount, 1);
        assert_eq!(
            sources[0].colors.len(),
            5,
            "one mana of any colour, as the Mox says"
        );
        assert_eq!(
            sources[0].tap,
            Tap::Ability(0),
            "tapped through the handle the engine offered it under"
        );
    }

    /// A land offering both of its printed mana abilities, which is the
    /// shape #165 is about.
    fn both_modes(name: &str) -> (PlayerView, LegalActions) {
        offering(name, &[0, 1])
    }

    fn mana_cost(src: &str) -> baylee_core::mana::ManaCost {
        baylee_core::mana::ManaCost::try_parse(src).expect("a valid cost")
    }

    /// #165. Havenwood Battleground prints `{T}: Add {G}` beside `{T},
    /// Sacrifice this land: Add {G}{G}`, and the second makes more mana — so
    /// the comparator that ranked on amount alone kept the sacrifice and
    /// threw the free tap away. A player who asked the client for one green
    /// was handed a plan that gave up the land.
    #[test]
    fn havenwood_is_tapped_for_its_printed_green_and_not_sacrificed() {
        let (view, legal) = both_modes("Havenwood Battleground");
        let sources = sources(&view, &legal);
        assert_eq!(sources.len(), 1, "one permanent is one source");
        assert_eq!(
            sources[0].tap,
            Tap::Ability(0),
            "kept the sacrifice over the free tap"
        );
        assert_eq!(sources[0].amount, 1, "one green, which is what was asked");

        // And the half a player would actually see.
        let plan = baylee_client_core::manaplan::plan(
            &mana_cost("{G}"),
            &baylee_view::ManaPoolView::default(),
            &sources,
        )
        .expect("a land that taps for green pays {G}");
        assert_eq!(plan.steps.len(), 1);
        assert_eq!(
            plan.steps[0].tap,
            Tap::Ability(0),
            "the plan sacrifices the land for one green"
        );
    }

    /// The same defect wearing its other face, and the commoner one: 18 of
    /// the 25 lose a **colour** rather than an amount. Vivid Crag prints
    /// `{T}: Add {R}` beside `{T}, Remove a charge counter: Add one mana of
    /// any color`, equal amounts, and the second offers five colours — so it
    /// won on breadth and the land burned a counter to make the red it
    /// prints.
    #[test]
    fn a_vivid_land_is_planned_as_the_colour_it_prints_and_keeps_its_counter() {
        let (view, legal) = both_modes("Vivid Crag");
        let sources = sources(&view, &legal);
        assert_eq!(sources.len(), 1, "one permanent is one source");
        assert_eq!(
            sources[0].tap,
            Tap::Ability(0),
            "spent a charge counter for the colour already printed on the land"
        );
        assert_eq!(
            sources[0].colors,
            vec![ManaColor::Red],
            "the printed red and nothing else"
        );
    }

    /// **A pinned limitation, not an assertion of correctness.**
    ///
    /// `{G}{G}` off a lone Havenwood Battleground is payable at the table —
    /// the card prints exactly that — and this client will not offer it,
    /// because one entry per permanent means the comparator chose the free
    /// mode and the planner never learns the other exists. That is the price
    /// of #165 and it is the right way round: a shy planner costs a player
    /// some clicks, an over-eager one strands a half-tapped board mid-cast
    /// with no way back.
    ///
    /// **It is a schedule.** It goes red the day `manaplan::Source` carries
    /// modes and `assign` picks one per permanent, and that redness is the
    /// success. A limitation test deleted when the limitation lifts was never
    /// a test.
    ///
    /// **One sentence, two causes.** "Expensive" means a price in the cost —
    /// Havenwood's sacrifice, a Vivid land's charge counter — and, since
    /// #149, a rider in the effects as well: Adarkar Wastes is never planned
    /// for white for exactly the reason Vivid Crag is never planned for blue,
    /// and `priced` is the one function that weighs both. So whichever of the
    /// two lifts first, this test moves for a reason a reader can name rather
    /// than for a reason they have to reconstruct.
    #[test]
    fn the_expensive_mode_is_out_of_reach_and_that_is_the_bargain() {
        let (view, legal) = both_modes("Havenwood Battleground");
        let sources = sources(&view, &legal);
        assert!(
            baylee_client_core::manaplan::plan(
                &mana_cost("{G}{G}"),
                &baylee_view::ManaPoolView::default(),
                &sources,
            )
            .is_none(),
            "the second mode became reachable — see the doc on this test"
        );
        // The counter-proof that the assertion above is about the *mode* and
        // not about the land being unreadable: one green is planned, so the
        // source is there and it is the cheap one.
        assert!(
            baylee_client_core::manaplan::plan(
                &mana_cost("{G}"),
                &baylee_view::ManaPoolView::default(),
                &sources,
            )
            .is_some(),
            "the land pays no mana at all, so the test above proves nothing"
        );
    }

    /// A land offering the mana abilities named by index, as the engine
    /// would.
    fn offering(name: &str, indices: &[u32]) -> (PlayerView, LegalActions) {
        let view = ViewBuilder::new(2)
            .with_battlefield(0, [card(1, 0, name)])
            .build();
        let legal = LegalActions {
            abilities: indices
                .iter()
                .map(|&index| (ObjectId::new(1, 0), index))
                .collect(),
            ..LegalActions::default()
        };
        (view, legal)
    }

    /// #149, and the whole of what it buys. Ancient Tomb prints one mana
    /// ability — `{T}: Add {C}{C}. This land deals 2 damage to you.` — and
    /// the strict reader refused it for the damage, so the land was not a
    /// source at all. It counted for nothing in every plan and a player
    /// tapped it by hand each time.
    #[test]
    fn a_land_whose_only_mana_ability_has_a_rider_is_a_source_at_all() {
        let (view, legal) = offering("Ancient Tomb", &[0]);
        let sources = sources(&view, &legal);
        assert_eq!(sources.len(), 1, "Ancient Tomb is still invisible");
        assert_eq!(sources[0].amount, 2, "two colourless, as printed");
        assert_eq!(sources[0].colors, vec![ManaColor::Colorless]);

        let plan = baylee_client_core::manaplan::plan(
            &mana_cost("{2}"),
            &baylee_view::ManaPoolView::default(),
            &sources,
        )
        .expect("one tap of Ancient Tomb pays {2}");
        assert_eq!(plan.steps.len(), 1, "one tap");
    }

    /// The guard, and the reason the two halves of #149 are one commit.
    ///
    /// Adarkar Wastes prints `{T}: Add {C}` beside `{T}: Add {W} or {U}.
    /// This land deals 1 damage to you.` — and **both cost only `{T}`**, so
    /// the cost half of `priced` cannot tell them apart. Teach the reader to
    /// accept riders without teaching `priced` that an effect beside the
    /// mana is a price too, and the two-colour tap wins the dedup on
    /// breadth: the client would pay a life to make colourless for a generic
    /// cost.
    ///
    /// Green before #149 for a different reason — the reader refused the
    /// coloured tap outright — so this discriminates only against the half
    /// of the change it is here to guard. Strip `does_more_than_add` from
    /// `priced` and it goes red.
    #[test]
    fn a_painland_pays_a_generic_cost_without_dealing_itself_damage() {
        let (view, legal) = offering("Adarkar Wastes", &[0, 1]);
        let sources = sources(&view, &legal);
        assert_eq!(sources.len(), 1, "one permanent is one source");
        assert_eq!(
            sources[0].tap,
            Tap::Ability(0),
            "took the damaging tap over the free one"
        );
        assert_eq!(
            sources[0].colors,
            vec![ManaColor::Colorless],
            "the clean colourless tap, which is what a generic cost asks for"
        );
    }

    /// **The reader family's relation, made a build failure.**
    ///
    /// `mana_shape` is documented as `mana_with_riders` with one clause put
    /// back, and a sentence is not a check. Over every ability in the pool:
    /// what the strict reader accepts, the loose one accepts *identically*;
    /// and what the loose one accepts with something beside the mana, the
    /// strict one refuses. Either half failing means the two have drifted
    /// into different answers about one ability, which is the fault the
    /// module header says this family exists to prevent.
    ///
    /// The population is bounded at both ends for the reason `cross-read`'s
    /// is: a reader that goes blind reports agreement over nothing.
    #[test]
    fn the_strict_reader_is_the_loose_one_with_its_clause_put_back() {
        let (mut agreed, mut riders) = (0usize, 0usize);
        for def in baylee_cards::all() {
            for face in 0..def.faces.len() {
                for ability in def.abilities_for_face(face) {
                    let (AbilityDef::Activated {
                        cost,
                        effects,
                        mana_ability: true,
                        ..
                    }
                    | AbilityDef::ActivatedConditional {
                        cost,
                        effects,
                        mana_ability: true,
                        ..
                    }) = ability
                    else {
                        continue;
                    };
                    let strict = baylee_cards_dsl::mana_shape(cost, effects);
                    let loose = baylee_cards_dsl::mana_with_riders(cost, effects);
                    if let Some(strict) = strict {
                        assert_eq!(
                            Some(strict),
                            loose,
                            "the loose reader lost an ability the strict one reads"
                        );
                        agreed += 1;
                    } else if loose.is_some() {
                        assert!(
                            effects.len() > 1,
                            "the strict reader refused for something other than a rider"
                        );
                        riders += 1;
                    }
                }
            }
        }
        assert!(
            agreed > 800,
            "only {agreed} abilities read alike — the walk went blind"
        );
        assert!(
            riders > 30,
            "only {riders} ridered abilities found — the walk went blind"
        );
    }

    /// What #149 is worth, counted rather than claimed: the cards that had
    /// **no** readable mana ability at all and now have one.
    ///
    /// A card with a clean tap beside a ridered one gains nothing here — the
    /// dedup keeps one entry per permanent and `priced` gives it to the
    /// clean one — so Adarkar Wastes is a colourless source before and
    /// after. The yield is entirely the lands whose *only* mana ability has
    /// something beside the mana.
    #[test]
    fn the_cards_this_makes_visible_are_the_ones_with_no_clean_tap() {
        let mut gained: Vec<&str> = Vec::new();
        for def in baylee_cards::all() {
            for face in 0..def.faces.len() {
                let (mut strict, mut loose) = (false, false);
                for ability in def.abilities_for_face(face) {
                    let (AbilityDef::Activated {
                        cost,
                        effects,
                        mana_ability: true,
                        ..
                    }
                    | AbilityDef::ActivatedConditional {
                        cost,
                        effects,
                        mana_ability: true,
                        ..
                    }) = ability
                    else {
                        continue;
                    };
                    strict |= baylee_cards_dsl::mana_shape(cost, effects).is_some();
                    loose |= baylee_cards_dsl::mana_with_riders(cost, effects).is_some();
                }
                if loose && !strict {
                    gained.push(def.faces[face].name);
                }
            }
        }
        gained.sort_unstable();
        assert!(
            gained.contains(&"Ancient Tomb"),
            "the card the change was written for is not among {gained:?}"
        );
        // A budget on a known population rather than a target: a card added
        // with a rider and no clean tap costs nobody a red gate, and a drift
        // past this says the population moved and wants reading.
        assert!(
            (8..=16).contains(&gained.len()),
            "{} cards gained a source: {gained:?}",
            gained.len()
        );
    }
}
