//! Public and shared hands are readable without granting permission to play them.
use super::{
    ArtSize, BrowseRow, BrowseZone, Color, Colors, Facts, Flag, ImageKey, Interaction, Names,
    Opening, PlayerView, Query, RowStanding, Surface,
};

pub(super) fn rows(
    view: &PlayerView,
    zone: BrowseZone,
    mine: Option<&Interaction>,
    query: &Query,
    opening: Opening,
    names: Names<'_>,
) -> Vec<BrowseRow> {
    let mut out = Vec::new();
    let BrowseZone::ShownHand(player) = zone else {
        return out;
    };
    let Some(hand) = view.shared_hands.iter().find(|hand| hand.player == player) else {
        return out;
    };
    for card in &hand.cards {
        let shown = (names.hand)(card).unwrap_or_else(|| card.name.clone());
        let selectable = mine.is_some_and(|it| it.selectable().contains(&card.id));
        if (opening == Opening::ForChoice && mine.is_some() && !selectable)
            || (!query.is_anything() && !shown_hand_matches(query, card, &shown))
        {
            continue;
        }
        out.push(BrowseRow {
            id: card.id,
            name: shown,
            art: Some(ImageKey::of(card.card, ArtSize::Small)),
            zone: BrowseZone::ShownHand(player),
            standing: RowStanding {
                selectable,
                selected: mine.is_some_and(|it| it.is_selected(card.id)),
                focused: mine.and_then(Interaction::aim)
                    == Some(crate::interaction::Pick::Object(card.id)),
            },
            place: None,
            pile: None,
            mana_value: card.mana_value,
            types: card.types,
            token: false,
        });
    }
    out
}

/// A revealed hand exposes printed facts rather than battlefield projections.
fn shown_hand_matches(query: &Query, card: &baylee_view::HandObject, shown: &str) -> bool {
    let line = crate::card_face::projected_type_line(&crate::card_face::Characteristics {
        types: card.types,
        ..Default::default()
    });
    let letters: String = Color::ALL
        .into_iter()
        .filter(|color| card.colors.contains(*color))
        .map(Color::symbol)
        .collect();
    let flags = if card.commander {
        &[Flag::Commander][..]
    } else {
        &[]
    };
    crate::cardquery::matches(
        query,
        &Facts {
            name: shown,
            english_name: &card.name,
            alt_names: &[],
            type_line: &line,
            kinds: &[],
            oracle: "",
            colors: Colors::of_letters(&letters),
            identity: Colors::default(),
            mana_cost: "",
            mana_value: card.mana_value,
            power: None,
            toughness: None,
            loyalty: None,
            flags,
        },
        Surface::ZONE,
    )
    .shown()
}
