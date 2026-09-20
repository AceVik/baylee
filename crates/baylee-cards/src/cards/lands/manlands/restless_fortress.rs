//! Restless Fortress — (no cost) — Land
//! Oracle: This land enters tapped.
//! Oracle: {T}: Add {W} or {B}.
//! Oracle: {2}{W}{B}: This land becomes a 1/4 white and black Nightmare creature until end of turn. It's still a land.
//! Oracle: Whenever this land attacks, defending player loses 2 life and you gain 2 life.
//! Set: WOE #259 — Wilds of Eldraine | Scryfall ID: 675213bb-28d7-460c-a4f3-950f5b9090af | Oracle ID: 8b3726f1-20b8-42ec-8f9b-b361515c3f05
// PARTIAL — hand-written: enters tapped, the {W}/{B} mana line and the
// {2}{W}{B} self-animation are complete; the attack drain reads the opponent
// seat, because no PlayerRel names the defending player.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::RESTLESS_FORTRESS,
    oracle_id = "8b3726f1-20b8-42ec-8f9b-b361515c3f05",
    scryfall_id = "675213bb-28d7-460c-a4f3-950f5b9090af",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::White]),
    faces = &[face!(
        name = "Restless Fortress",
        types = TypeSet::LAND,
        enter_modifiers = &[EnterModifier::Tapped],
    )],
    coverage = Coverage::Partial(
        "the attack trigger's \"defending player\" has no PlayerRel — the opponent \
         seat is exact heads-up and a choice among opponents in multiplayer (M3)"
    ),
    abilities = &[
        mana_ability!(&[Effect::mana_choice(&[ManaColor::White, ManaColor::Black])]),
        // {2}{W}{B}: animate. Type and subtype are layer 4, the colors layer 5
        // and the 1/4 is a layer-7b set. "It's still a land" is what AddType
        // says on its own — nothing here removes a type.
        activated!(
            cost!("{2}{W}{B}"),
            &[
                Effect::continuous(
                    &Filter::This,
                    Modifier::AddType(TypeSet::CREATURE),
                    Duration::UntilEndOfTurn,
                ),
                Effect::continuous(
                    &Filter::This,
                    Modifier::AddSubtype(subtypes::creature::NIGHTMARE),
                    Duration::UntilEndOfTurn,
                ),
                Effect::continuous(
                    &Filter::This,
                    Modifier::SetColor(ColorSet::from_slice(&[Color::White, Color::Black])),
                    Duration::UntilEndOfTurn,
                ),
                Effect::continuous(
                    &Filter::This,
                    Modifier::SetPT(1, 4),
                    Duration::UntilEndOfTurn,
                ),
            ],
        ),
        // NOT SUPPORTED: "defending player loses 2 life" — no PlayerRel names
        // the defending seat, so the trigger reads the opponent seat.
        triggered!(
            Trigger::Attacks(&Filter::This),
            &[
                Effect::LoseLife {
                    amount: Amount::Fixed(2),
                    target: PlayerRel::Opponent,
                },
                Effect::gain_life(2),
            ],
        ),
    ],
);
