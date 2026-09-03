//! A card's face on a UI node: its art, or the text panel that stands in
//! for art that will not load.

#[allow(clippy::wildcard_imports)] // the HUD's own vocabulary
use super::*;

// ------------------------------------------------- art, or the card's text

/// What the overlay needs in order to choose between a card's art and its own
/// constructed face.
///
/// Bundled because every card-drawing helper here needs all three, and three
/// more parameters on functions that already carry eleven is how a signature
/// stops being readable.
pub(super) struct FaceCtx<'a> {
    pub(super) texts: &'a crate::cardtext::CardTexts,
    pub(super) mode: &'a crate::face::FaceMode,
    pub(super) settings: &'a crate::settings::ClientSettings,
}

impl FaceCtx<'_> {
    /// Whether every card is showing its face, whatever its art is doing.
    ///
    /// Part of the redraw gate: this one is a held key and a setting, so it
    /// changes without a new snapshot.
    pub(super) fn always(&self) -> bool {
        self.mode.held || self.settings.prefer_text_view
    }

    /// The face to draw instead of a card's art, or `None` to draw the art.
    pub(super) fn object(
        &self,
        object: &baylee_view::PublicObject,
        textures: &CardTextures,
        art: Option<ImageKey>,
    ) -> Option<CardFace> {
        crate::face::wants_face(self.mode, self.settings, textures, art)
            .then(|| crate::face::of_object(object, self.texts))
    }

    /// The same, for a card in hand.
    pub(super) fn hand(
        &self,
        card: &baylee_view::HandObject,
        textures: &CardTextures,
        art: Option<ImageKey>,
    ) -> Option<CardFace> {
        crate::face::wants_face(self.mode, self.settings, textures, art)
            .then(|| crate::face::of_hand(card, self.texts))
    }
}

/// Draws a card into a slot of a fixed size: its art, or its face.
///
/// Every place the overlay shows a card goes through here, which is what makes
/// the two interchangeable: the slot is the same size either way, so holding
/// the modifier reveals text without moving anything on screen.
///
/// The art goes through [`CardUiMaterial`] rather than a plain `ImageNode`, so
/// a foil in a player's hand looks like the foil that will land on the table.
/// One shader, one set of constants, two pipelines — two that disagreed about
/// what "foil" means would be worse than one that only ran on the table.
#[allow(clippy::too_many_arguments)] // a slot, a card, and the material store
pub(super) fn spawn_card_art(
    commands: &mut Commands,
    lang: Lang,
    image: Handle<Image>,
    built: Option<&CardFace>,
    width: f32,
    height: f32,
    detail: crate::face::Detail,
    fonts: &UiFonts,
    surface: CardLook,
    cards: Option<&mut UiCards<'_>>,
) -> Entity {
    let slot = commands
        .spawn(Node {
            width: px(width),
            height: px(height),
            overflow: Overflow::clip(),
            ..default()
        })
        .id();
    let child = if let Some(face) = built {
        crate::face::spawn_ui(commands, lang, face, width, detail, fonts)
    } else if let Some(cards) = cards {
        commands
            .spawn((
                MaterialNode(cards.get(surface, Some(image))),
                Node {
                    width: percent(100),
                    height: percent(100),
                    ..default()
                },
            ))
            .id()
    } else {
        // No render plugins, so no material store: draw the art plainly. A
        // headless test builds the whole overlay this way, which is what
        // keeps those tests free of a GPU *and* of the network.
        commands
            .spawn((
                ImageNode::new(image),
                Node {
                    width: percent(100),
                    height: percent(100),
                    ..default()
                },
            ))
            .id()
    };
    commands.entity(slot).add_child(child);
    slot
}
