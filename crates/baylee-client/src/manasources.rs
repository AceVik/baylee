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

use baylee_cards_dsl::AbilityDef;

/// Every mana source the seat may tap right now.
///
/// **One entry per permanent**, which is the whole subtlety here. A Forest
/// appears twice in `LegalActions` — once in `mana_abilities` as the CR 305.6
/// shortcut and once in `abilities` as the `{T}: Add {G}` printed on the card
/// — and it can still only be tapped once. Two entries would let the planner
/// pay `{G}{G}` with one Forest, which is a plan the engine refuses after the
/// land is already tapped.
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

    // A permanent taps once, so it is one source: the best of whatever it
    // offered. "Best" is the one that leaves the planner the most room — more
    // mana first, then more colours — and the intrinsic shortcut wins a tie
    // because it costs one fewer round trip and is never asked for a colour.
    sources.sort_by(|a, b| {
        a.id.cmp(&b.id)
            .then_with(|| b.amount.cmp(&a.amount))
            .then_with(|| b.colors.len().cmp(&a.colors.len()))
            .then_with(|| matches!(a.tap, Tap::Ability(_)).cmp(&matches!(b.tap, Tap::Ability(_))))
    });
    sources.dedup_by(|a, b| a.id == b.id);
    sources
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
/// The reading itself is [`baylee_cards_dsl::mana_shape`], which is where it
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
    let (source, amount, restricted) = baylee_cards_dsl::mana_shape(cost, effects)?;
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
    let colors = produced_colors(view, source)?;
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
pub(crate) fn produced_colors(
    view: &PlayerView,
    source: baylee_cards_dsl::ManaSource,
) -> Option<Vec<baylee_core::mana::ManaColor>> {
    match source {
        baylee_cards_dsl::ManaSource::Fixed(color) => Some(vec![color]),
        baylee_cards_dsl::ManaSource::Choice(colors) => Some(colors.to_vec()),
        baylee_cards_dsl::ManaSource::CommanderIdentity => commander_identity(view),
        // Reflecting Pool and Exotic Orchard read the *projected*
        // `produced_colors` of every land on one side of the table, and no
        // view carries that — a Chromatic Lantern's grant is in there, and so
        // is a land that has lost its abilities. Reading it back out of the
        // registry would over-count, which is the direction that leaves a
        // board half tapped when the engine refuses the colour. It wants the
        // treatment `PublicObject::granted_mana` got: a projection from
        // gamehost, which is the only side that can see it.
        baylee_cards_dsl::ManaSource::LandColor { .. } => None,
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
                    Some((produced_colors(view, source)?, amount.is_some()))
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

/// The colours a Command Tower makes for the viewing seat.
///
/// The same answer `resolve::mana::colors_of` gives, read off the same two
/// facts: colour identity is a property of the commander *card* wherever it
/// is (CR 903.4), so this is the union over the seat's designated commanders
/// and not over the command zone — an Arcane Signet that stopped producing
/// the moment its commander was cast would be the bug on the engine's side,
/// and it would be this one here too.
///
/// Two answers that are easy to get backwards, and they are opposite:
///
/// - **No commanders at all** is a colourless answer, not a refusal. The
///   ability still resolves in a non-commander game and `{C}` is what is
///   left, which is what the engine does — and the offline house duel is
///   exactly that game, so it is the common case rather than the exotic one.
/// - **A commander this seat cannot name** is a refusal, not a colourless
///   answer. The engine will offer the colours its identity allows, and a
///   client that answered `{C}` would stall the run at the colour prompt
///   with the Tower already tapped.
fn commander_identity(view: &PlayerView) -> Option<Vec<baylee_core::mana::ManaColor>> {
    let seat = view.seat(view.seat)?;
    let mut identity = baylee_core::color::ColorSet::EMPTY;
    for commander in &seat.commanders {
        let card = commander.card.as_ref()?;
        identity = identity.union(baylee_cards::by_index(card.index)?.color_identity);
    }
    let colors: Vec<_> = identity
        .iter()
        .map(baylee_core::mana::ManaColor::from_color)
        .collect();
    if colors.is_empty() {
        return Some(vec![baylee_core::mana::ManaColor::Colorless]);
    }
    Some(colors)
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

    /// One of the viewing seat's commanders, named.
    ///
    /// The index is looked up rather than passed in beside the name. It used
    /// to be both, and the pair is only ever right by luck: an index is
    /// assigned over the whole card corpus, so the number that was General
    /// Tazri is now some other card entirely, and a test asserting about a
    /// five-colour identity would have been reading whatever landed there.
    fn commander(name: &str) -> baylee_view::CommanderView {
        let index = baylee_cards::decks::by_name(name).expect("a card of that name in the pool");
        baylee_view::CommanderView {
            object: ObjectId::new(90 + index.get(), 0),
            card: Some(baylee_view::CardIdentity {
                index,
                print: baylee_core::ids::PrintRef::new(0),
                face: 0,
            }),
            name: name.to_string(),
            casts: 0,
        }
    }

    /// The owner's own game: General Tazri's `{W}{U}{B}{R}{G}` ability puts
    /// all five colours in her identity (CR 903.4), so the Tower is the
    /// five-colour deck's only green source — and the planner could not see
    /// it at all, because the reading stopped at "the board knows". The
    /// Harabaz Druid in hand cost `{1}{G}` and the Tower had to be tapped by
    /// hand.
    #[test]
    fn a_command_tower_makes_what_the_commander_identity_allows() {
        let (mut view, legal) = command_tower();
        view.seats[0].commanders = vec![commander("General Tazri")];

        let sources = sources(&view, &legal);
        assert_eq!(sources.len(), 1, "one permanent, one source");
        assert_eq!(sources[0].amount, 1);
        assert_eq!(
            sources[0].colors.len(),
            5,
            "Tazri's activated ability costs one of each"
        );
        assert!(
            sources[0]
                .colors
                .contains(&baylee_core::mana::ManaColor::Green),
            "the green the Druid needed"
        );
    }

    /// A narrower commander, so the same land makes fewer colours. Worth its
    /// own test because a five-colour answer that happened to be a constant
    /// would pass the one above and would tap the Tower for green in a deck
    /// that has none — which is the plan the engine refuses at the colour
    /// prompt, with the land already tapped.
    ///
    /// Aminatou, the Fateshifter is `{W}{U}{B}` and is the other seat's
    /// commander in the game this was reported from.
    #[test]
    fn the_identity_is_the_commanders_and_not_the_rainbow() {
        let (mut view, legal) = command_tower();
        view.seats[0].commanders = vec![commander("Aminatou, the Fateshifter")];
        let colors = &sources(&view, &legal)[0].colors;
        let identity = baylee_cards::by_index(
            baylee_cards::decks::by_name("Aminatou, the Fateshifter").expect("she is in the pool"),
        )
        .expect("a card at her index")
        .color_identity;
        assert_eq!(
            colors.len(),
            identity.iter().count(),
            "one mana colour per colour of the commander's identity"
        );
        assert!(
            colors.len() < 5,
            "not every deck is five colours: {colors:?}"
        );
    }

    /// The offline house duel, which is not a commander game at all: the
    /// ability still resolves and colourless is what is left (the engine's
    /// own fallback). A refusal here would be the common case failing.
    #[test]
    fn a_tower_with_no_commander_at_the_table_taps_for_colorless() {
        let (view, legal) = command_tower();
        let sources = sources(&view, &legal);
        assert_eq!(sources.len(), 1);
        assert_eq!(
            sources[0].colors,
            vec![baylee_core::mana::ManaColor::Colorless]
        );
    }

    /// And the opposite answer to the one above, which is the whole reason
    /// they are two tests: a commander this seat cannot name is a *refusal*.
    /// The engine will offer the colours that identity allows, so a client
    /// answering `{C}` stalls the run at the colour prompt with the Tower
    /// already tapped.
    #[test]
    fn a_commander_this_seat_cannot_name_is_not_a_colorless_tower() {
        let (mut view, legal) = command_tower();
        let mut unknown = commander("General Tazri");
        unknown.card = None;
        view.seats[0].commanders = vec![unknown];
        assert!(sources(&view, &legal).is_empty());
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
}
