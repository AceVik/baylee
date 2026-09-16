//! Emry, Lurker of the Loch — {2}{U} — Legendary Creature — Merfolk Wizard
//! Oracle: Affinity for artifacts (This spell costs {1} less to cast for each artifact you control.)
//! Oracle: When Emry enters, mill four cards.
//! Oracle: {T}: Choose target artifact card in your graveyard. You may cast that card this turn. (You still pay its costs. Timing rules still apply.)
//! Set: EOC #71 — Edge of Eternities Commander | Scryfall ID: c977d89a-bfd1-4e98-9d95-3e41c53dd188 | Oracle ID: da3e7d3d-2ca0-40c3-9602-fca37c92f507
// PARTIAL — she mills her controller four on the way in and taps to hand one
// artifact card in their own graveyard a permission to be cast this turn for
// its printed cost; the printed affinity never comes off her own cost, so she
// is always paid for at {2}{U}.
// NOT SUPPORTED: `Affinity for artifacts` — "costs {1} less to cast for each
// artifact you control". A printed reduction is `FaceDef::cost_reduction`,
// whose one variant is `CostReduction::NotStartingPlayer(n)`: a flat {n} on a
// yes/no condition, with nothing that counts permanents on the battlefield
// (`docs/card-dsl.md`, "Cost reducers"). She therefore plays exactly as though
// the line were not printed.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::EMRY_LURKER_OF_THE_LOCH,
    oracle_id = "da3e7d3d-2ca0-40c3-9602-fca37c92f507",
    scryfall_id = "c977d89a-bfd1-4e98-9d95-3e41c53dd188",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    commander = CommanderRule::Legendary,
    faces = &[face!(
        name = "Emry, Lurker of the Loch",
        mana_cost = mana!("{2}{U}"),
        types = TypeSet::CREATURE,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::creature::MERFOLK, subtypes::creature::WIZARD],
        power = Some(1),
        toughness = Some(2),
    ),],
    coverage = Coverage::Partial("affinity for artifacts does not reduce the cost"),
    abilities = &[
        triggered!(
            Trigger::ETB,
            &[Effect::Mill {
                amount: Amount::Fixed(4),
                target: PlayerRel::You,
            }]
        ),
        // "You may cast that card this turn. (You still pay its costs. Timing
        // rules still apply.)" is `Effect::GrantFlashback`: it registers an
        // `UntilEndOfTurn` `Modifier::GrantsFlashback` on the chosen card, and
        // that is precisely the permission Emry prints — `casting::can_cast`
        // lets the grant out of the graveyard, then probes the card's own
        // printed cost and runs `timing_allows` on it like any other spell.
        //
        // The half of flashback Emry does *not* print — CR 702.34a's exile
        // instead of the graveyard — cannot reach her, and that is why this
        // spelling is honest rather than close: it rides on
        // `Rider::Flashback`, which `finalize_spell` reads only on its
        // non-permanent branch, and every artifact card is a permanent card.
        // Whoever later widens that exile towards "any time it would leave the
        // stack" (a countered spell, say) has to give Emry a permission of her
        // own here.
        activated!(
            Cost::TAP,
            &[Effect::GrantFlashback],
            target = Some(TargetSpec::CardInGraveyard(
                &Filter::ARTIFACT,
                PlayerRel::You
            ))
        ),
    ],
);

// Engine-level test belongs in baylee-engine (card_tests): nothing in the pool
// plays `Effect::GrantFlashback` in a game yet — Stingcaster Mage's own note
// says so — and Emry is the first card to point it at a *permanent* card, so
// the grant wants one turn of its own: tap her, choose an artifact in the
// graveyard, cast it for its printed mana cost at sorcery speed, and watch it
// resolve onto the battlefield rather than into exile. The second half of that
// game is CR 400.7: the grant is keyed to an `ObjectId` this engine keeps
// across a zone change, so an artifact cast and then put back into the
// graveyard the same turn must not still be castable off the same activation.
