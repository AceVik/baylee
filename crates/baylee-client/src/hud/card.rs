//! A card's face on a UI node: its art, or the text panel that stands in
//! for art that will not load.

#[allow(clippy::wildcard_imports)] // the HUD's own vocabulary
use super::*;

// ------------------------------------------------- art, or the card's text

/// What the overlay needs in order to choose between a card's art and its own
/// constructed face.
///
/// Bundled because every card-drawing helper here needs the same set, and
/// four more parameters on functions that already carry eleven is how a
/// signature stops being readable.
pub(super) struct FaceCtx<'a> {
    pub(super) texts: &'a crate::cardtext::CardTexts,
    pub(super) mode: &'a crate::face::FaceMode,
    pub(super) settings: &'a crate::settings::ClientSettings,
    /// The seat's view, for the one face that cannot be built without it: an
    /// ability on the stack has no card, so its text and its name are the
    /// source permanent's and the source is found through here. `None` before
    /// the first view arrives, which is also when there is nothing to draw.
    pub(super) view: Option<&'a baylee_view::PlayerView>,
}

impl FaceCtx<'_> {
    /// Whether every card is showing its face, whatever its art is doing.
    ///
    /// Part of the redraw gate: this one is a held key and a setting, so it
    /// changes without a new snapshot.
    pub(super) fn always(&self) -> bool {
        self.mode.held || self.settings.prefer_text_view
    }

    /// A card's characteristics, whatever is being *drawn* for it.
    ///
    /// [`Self::object`] answers a question about the picture — "show the face
    /// instead of the art" — and answers `None` whenever the art is winning.
    /// The zone browser's rows want the other thing: the cost and the type
    /// line are written beside a thumbnail that is always the art, so asking
    /// through the toggle left every row with a name and two empty columns
    /// until somebody held the text modifier.
    pub(super) fn facts(&self, object: &baylee_view::PublicObject) -> CardFace {
        crate::face::of_object(object, self.view, self.texts)
    }

    /// The face to draw instead of a card's art, or `None` to draw the art.
    pub(super) fn object(
        &self,
        object: &baylee_view::PublicObject,
        textures: &CardTextures,
        art: Option<ImageKey>,
    ) -> Option<CardFace> {
        crate::face::wants_face(self.mode, self.settings, textures, art)
            .then(|| crate::face::of_object(object, self.view, self.texts))
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
    // Neither the slot nor what is inside it is ever the pointer's target:
    // the art is always inside something that *is* — a hand card, a tray row,
    // a stack entry — and a picture that took the hover for itself would
    // leave that thing dark under the one part of it a player looks at.
    //
    // Both, and not the slot alone, because `build_hover_map` stops at the
    // **first** entity that carries no `Pickable` at all, and the backend
    // reports the deepest node first. A marked slot around an unmarked child
    // is exactly as opaque as an unmarked slot; it only moves which entity
    // does the blocking.
    commands.entity(child).insert(Pickable::IGNORE);
    commands.entity(slot).insert(Pickable::IGNORE);
    commands.entity(slot).add_child(child);
    slot
}
