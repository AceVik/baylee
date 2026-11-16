//! Exact-edition language cosmetics, shared by Tower previews and table art.
//! The frozen preset/earned print table stays unchanged. No metadata request,
//! foreign hidden print, ordinary deck or replay is rewritten by this adapter.
use baylee_client_core::{
    i18n::Lang,
    images::{ImageKey, ImageRequest, image_url},
};
use baylee_core::deckrow::PrintChoice;
use baylee_view::{GameStatic, PrintEntry};
use bevy::prelude::*;
use serde::Deserialize;
use std::{borrow::Cow, sync::OnceLock};
use uuid::Uuid;

#[derive(Deserialize)]
struct Palette {
    schema: u8,
    pairs: Vec<Pair>,
}
#[derive(Deserialize)]
struct Pair {
    en: Uuid,
    de: Uuid,
}

fn palette() -> &'static [(Uuid, Uuid)] {
    static PALETTE: OnceLock<Box<[(Uuid, Uuid)]>> = OnceLock::new();
    PALETTE.get_or_init(|| {
        let raw: Palette =
            serde_json::from_str(include_str!("../assets/tower/print-languages.json"))
                .expect("reviewed bundled print palette");
        assert_eq!(raw.schema, 1, "bundled print palette schema");
        raw.pairs
            .into_iter()
            .map(|pair| (pair.en, pair.de))
            .collect()
    })
}

fn german(id: &str, lang: Lang) -> Option<Uuid> {
    if lang != Lang::De {
        return None;
    }
    let id = Uuid::parse_str(id).ok()?;
    let palette = palette();
    let index = palette.binary_search_by_key(&id, |pair| pair.0).ok()?;
    Some(palette[index].1)
}

/// Presentation-only copy; callers opt in for Tower and keep their frozen model.
pub(crate) fn choice(print: &PrintChoice, lang: Lang) -> Cow<'_, PrintChoice> {
    let Some(id) = print.scryfall_id.as_deref().and_then(|id| german(id, lang)) else {
        return Cow::Borrowed(print);
    };
    let mut shown = print.clone();
    shown.scryfall_id = Some(id.to_string());
    shown.lang = Some("de".to_owned());
    Cow::Owned(shown)
}

pub(crate) fn entry(print: &PrintEntry, lang: Lang) -> Cow<'_, PrintEntry> {
    let Some(id) = german(&print.scryfall_id, lang) else {
        return Cow::Borrowed(print);
    };
    Cow::Owned(PrintEntry {
        scryfall_id: id.to_string(),
        lang: "de".to_owned(),
        finish: print.finish,
    })
}

pub(crate) fn hover(
    deck: &baylee_client_core::deckbuilder::DeckBuilder,
    card: &baylee_client_core::deckbuilder::PoolCard,
    print: Option<&PrintChoice>,
    lang: Lang,
) -> crate::lobby::HoverCard {
    let print = print.or_else(|| {
        deck.limited()?
            .stock
            .get(&card.index)
            .map(|holding| &holding.print)
    });
    print.map_or_else(
        || crate::lobby::hover_of_card(card),
        |print| {
            if deck.limited().is_some() {
                crate::lobby::hover_of_entry(card, &choice(print, lang))
            } else {
                crate::lobby::hover_of_entry(card, print)
            }
        },
    )
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct Policy {
    game: Uuid,
    language: Lang,
}

impl Policy {
    pub(crate) fn request(self, statics: &GameStatic, key: ImageKey) -> Option<ImageRequest> {
        if Uuid::parse_str(&statics.game_id).ok()? != self.game {
            return None;
        }
        let print = statics.print(key.printing()?)?;
        let Cow::Owned(shown) = entry(print, self.language) else {
            return None;
        };
        Some(ImageRequest {
            key,
            url: image_url(&shown, key.face, key.size)?,
            treatment: print.finish.into(),
        })
    }
}

pub(crate) fn install(app: &mut App) {
    app.add_systems(
        Update,
        bind.after(crate::DuelSet::Sync)
            .before(crate::DuelSet::Present),
    );
}

fn bind(
    lobby: Option<Res<crate::lobby::LobbyState>>,
    textures: Option<ResMut<crate::textures::CardTextures>>,
    preload: Option<ResMut<crate::textures::Preload>>,
    materials: Option<ResMut<crate::cardmat::UiCardMaterials>>,
) {
    let wanted = lobby.as_ref().and_then(|state| {
        state.echoes.game().map(|game| Policy {
            game,
            language: state.lobby.lang(),
        })
    });
    let Some(mut textures) = textures else {
        return;
    };
    if textures.tower_prints() == wanted {
        return;
    }
    textures.localize_tower(wanted);
    if let Some(mut preload) = preload {
        *preload = crate::textures::Preload::default();
    }
    if let Some(mut materials) = materials {
        materials.clear_game();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use baylee_client_core::images::{ArtSize, Face, ImageSource};
    use baylee_core::ids::{PlayerId, PrintRef};

    #[test]
    fn bundled_pairs_are_sorted_distinct_and_preserve_finish_and_fallback() {
        let pairs = palette();
        assert!(!pairs.is_empty());
        assert!(pairs.windows(2).all(|pair| pair[0].0 < pair[1].0));
        assert!(
            pairs
                .iter()
                .all(|(en, de)| !en.is_nil() && !de.is_nil() && en != de)
        );
        let print = PrintEntry {
            scryfall_id: pairs[0].0.to_string(),
            lang: "en".into(),
            finish: baylee_view::Finish::Foil,
        };
        let shown = entry(&print, Lang::De);
        assert_eq!(shown.scryfall_id, pairs[0].1.to_string());
        assert_eq!(shown.finish, print.finish);
        assert!(matches!(entry(&print, Lang::En), Cow::Borrowed(_)));
        let unknown = PrintEntry {
            scryfall_id: Uuid::nil().to_string(),
            ..print.clone()
        };
        assert!(matches!(entry(&unknown, Lang::De), Cow::Borrowed(_)));
        assert_eq!(print.lang, "en");
    }

    #[test]
    fn table_translation_is_bound_to_the_tower_game_and_only_earned_prints() {
        let game = Uuid::from_u128(7);
        let mut statics = GameStatic {
            view_version: baylee_view::VIEW_VERSION,
            game_id: game.to_string(),
            your_seat: PlayerId(0),
            seats: Vec::new(),
            prints: Vec::new(),
            decision_secs: None,
            reconnect_secs: None,
            table_seed: 0,
        };
        let key = ImageKey {
            source: ImageSource::Print(PrintRef(0)),
            face: Face::Front,
            size: ArtSize::Small,
        };
        let policy = Policy {
            game,
            language: Lang::De,
        };
        assert!(
            policy.request(&statics, key).is_none(),
            "hidden printing stays unrepresentable"
        );
        statics.prints = vec![Some(PrintEntry {
            scryfall_id: palette()[0].0.to_string(),
            lang: "en".into(),
            finish: baylee_view::Finish::Foil,
        })];
        assert!(
            policy
                .request(&statics, key)
                .unwrap()
                .url
                .contains(&palette()[0].1.to_string())
        );
        statics.game_id = Uuid::from_u128(8).to_string();
        assert!(
            policy.request(&statics, key).is_none(),
            "ordinary games retain their chosen print"
        );
    }
}
