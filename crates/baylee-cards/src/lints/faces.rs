//! What a face prints: colours, identity, power and toughness, loyalty.

use super::*;

/// The [`Color`] one unit of mana carries, if any.
///
/// Colorless mana has no color rather than a sixth one, so it puts nothing
/// into an identity.
pub(super) fn color_of(mana: ManaColor) -> Option<Color> {
    match mana {
        ManaColor::White => Some(Color::White),
        ManaColor::Blue => Some(Color::Blue),
        ManaColor::Black => Some(Color::Black),
        ManaColor::Red => Some(Color::Red),
        ManaColor::Green => Some(Color::Green),
        ManaColor::Colorless => None,
    }
}

/// The colors an effect writes as **mana symbols**, which is what an
/// identity is made of.
///
/// "Add one mana of any color" is the line this has to read backwards, and
/// it is the reason the question is about symbols rather than about mana:
/// that sentence produces every color and prints no symbol, so Command
/// Tower is a colorless card (CR 903.4). The DSL spells it
/// `mana_choice(ALL_MANA_COLORS)`, so the exemption is the five-color *set*
/// and not the constructor that built it — a card that printed "{W}, {U},
/// {B}, {R}, or {G}" would be written the same way and is the same
/// colorless card. `CommanderIdentity` and `LandColor` are that sentence
/// said two other ways.
pub(super) fn mana_symbol_colors(effect: &Effect) -> ColorSet {
    let union = |set: ColorSet, effect: &Effect| set.union(mana_symbol_colors(effect));
    match effect {
        Effect::AddManaFor { color, .. } => color_of(*color).map_or(ColorSet::EMPTY, ColorSet::of),
        Effect::AddMana { source, .. } => match source {
            ManaSource::Fixed(mana) => color_of(*mana).map_or(ColorSet::EMPTY, ColorSet::of),
            ManaSource::Choice(colors) => {
                let named = colors
                    .iter()
                    .filter_map(|mana| color_of(*mana))
                    .fold(ColorSet::EMPTY, |set, color| set.union(ColorSet::of(color)));
                if named == ColorSet::ALL {
                    ColorSet::EMPTY
                } else {
                    named
                }
            }
            // A chosen color prints no symbol either — "one mana of the
            // chosen color" is a sentence, not `{W}` — so only what is
            // written *beside* it counts. Uncharted Haven is colorless and
            // Thriving Heath is white, off the one `{W}` it prints.
            ManaSource::ChosenOr(colors) => colors
                .iter()
                .filter_map(|mana| color_of(*mana))
                .fold(ColorSet::EMPTY, |set, color| set.union(ColorSet::of(color))),
            ManaSource::IntrinsicBasicLandTypes
            | ManaSource::Chosen
            | ManaSource::CommanderIdentity
            | ManaSource::LandColor { .. } => ColorSet::EMPTY,
        },
        // A conditional resolves its branches, so a symbol inside one is
        // still printed on the card. The recursion is [`swept_filters`]'
        // and is incomplete in the same way, which is safe here for a
        // reason worth stating: this lint only ever asks whether the
        // declared identity covers what was *found*, so a nesting it does
        // not walk costs a finding and can never invent one.
        Effect::IfKicked { then, otherwise }
        | Effect::IfEventPowerAtLeast {
            then, otherwise, ..
        } => then.iter().chain(*otherwise).fold(ColorSet::EMPTY, union),
        Effect::MayDo { effects: then }
        | Effect::IfCreaturesDiedAtLeast { then, .. }
        | Effect::IfNotLostLifeThisTurn { then, .. }
        | Effect::IfResolvedTimesThisTurn { then, .. }
        | Effect::IfControlGreatestCmc { then, .. }
        | Effect::IfNoCountersOnSelf { then, .. } => then.iter().fold(ColorSet::EMPTY, union),
        _ => ColorSet::EMPTY,
    }
}

/// Every color an ability shows, in its cost and in what it produces.
///
/// The `match` over the cost is exhaustive for [`branches`]' reason: a new
/// [`AbilityDef`] carrying a cost nobody classified here would be a color
/// the identity lint quietly stopped looking at. Ward's is generic (`{N}`)
/// and so has no color to find.
pub(super) fn ability_colors(ability: &AbilityDef) -> ColorSet {
    let cost: ManaCost = match ability {
        AbilityDef::Activated { cost, .. } | AbilityDef::ActivatedConditional { cost, .. } => {
            cost.mana
        }
        AbilityDef::Echo { cost } | AbilityDef::Suspend { cost, .. } => *cost,
        AbilityDef::ModalSpell { modes, .. } | AbilityDef::ModalTriggered { modes, .. } => modes
            .iter()
            .filter_map(|mode| mode.cost_override)
            .fold(ManaCost::ZERO, |all, mode| all.combine(&mode)),
        AbilityDef::Unimplemented
        | AbilityDef::Spell { .. }
        | AbilityDef::Triggered { .. }
        | AbilityDef::Ward { .. }
        | AbilityDef::Toxic { .. }
        | AbilityDef::SagaChapter { .. }
        | AbilityDef::Prepared { .. }
        | AbilityDef::Static(_)
        | AbilityDef::Replacement(_)
        | AbilityDef::CopyOnEnterUntilEot { .. }
        | AbilityDef::CopyOnEnter { .. }
        | AbilityDef::Loyalty { .. } => ManaCost::ZERO,
    };
    branches(ability)
        .iter()
        .flat_map(|branch| branch.effects)
        .fold(cost.colors(), |set, effect| {
            set.union(mana_symbol_colors(effect))
        })
}

/// Colors the card shows on itself that its `color_identity` does not carry
/// (CR 903.4).
///
/// One direction only. A card must not *hide* a color it prints — that is
/// the mistake that makes a deckbuilder offer an illegal card, and it is
/// decidable from the card alone. Whether the identity carries a color too
/// many is a question about the printing, which `xtask validate` asks
/// against the Scryfall payload; asking it here as well would fail on every
/// back face and color indicator this crate does not model.
pub(super) fn identity_gap(def: &CardDef) -> ColorSet {
    let faces = def.faces.iter().fold(ColorSet::EMPTY, |set, face| {
        set.union(face.mana_cost.colors())
            .union(face.color_indicator)
            .union(face.abilities.iter().fold(ColorSet::EMPTY, |set, ability| {
                set.union(ability_colors(ability))
            }))
    });
    def.abilities
        .iter()
        .fold(faces, |set, ability| set.union(ability_colors(ability)))
        .difference(def.color_identity)
}

/// Why a face's power, toughness and creature type do not agree (CR 208.1).
///
/// Both directions are silent bugs of their own. A creature with no
/// power/toughness is a permanent combat cannot size, and state-based
/// actions read a toughness that is not there; a face carrying numbers that
/// is not a creature is either a wrong type line or a Vehicle whose subtype
/// was dropped, and only the second of those is legal (CR 301.7).
pub(super) fn pt_fault(face: &FaceDef) -> Option<&'static str> {
    let creature = face.types.contains(TypeSet::CREATURE);
    let numbered = face.power.is_some() || face.toughness.is_some();
    // Two subtypes print a body they are not yet entitled to use: a Vehicle
    // is not a creature until it crews (CR 301.7) and a Spacecraft is not
    // one until it is stationed to 8+. Both print the numbers on the card,
    // and a Spacecraft left without them became a creature with no body at
    // all: Inspirit, Flagship Vessel reached 8 charge counters, turned into
    // an artifact creature and was put into its owner's graveyard by the
    // next state-based check.
    let printed_body = [
        baylee_core::generated::subtypes::artifact::VEHICLE,
        baylee_core::generated::subtypes::artifact::SPACECRAFT,
    ]
    .iter()
    .any(|s| face.subtypes.contains(s));
    if creature && !(face.power.is_some() && face.toughness.is_some()) {
        return Some("is a creature with no power/toughness");
    }
    if numbered && !creature && !printed_body {
        return Some("has power/toughness and is neither a creature nor a Vehicle or Spacecraft");
    }
    if printed_body && !(face.power.is_some() && face.toughness.is_some()) {
        return Some("prints a body on the card and carries none in the code");
    }
    None
}

/// Whether a face is a planeswalker that never says what it starts on
/// (CR 306.5b).
pub(super) fn loyalty_fault(face: &FaceDef) -> bool {
    face.types.contains(TypeSet::PLANESWALKER) && face.loyalty.is_none()
}
