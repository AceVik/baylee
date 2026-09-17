//! Explicit presentation order. Never changes the engine's hand or its choices.
use baylee_client_core::board::HandCard;
use baylee_view::{HandObject, PlayerView};

/// A contiguous group in the presentation hand.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HandGroup {
    /// Index of its first card.
    pub start: usize,
    /// Category interpreted by the selected ordering.
    pub key: u16,
}

/// Stable sorting and grouping chosen explicitly by the player.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum HandOrder {
    /// Arrival order, without category groups.
    #[default]
    Draw,
    /// Mana value, with a final 10+ group.
    Mana,
    /// Alphabetical initial.
    Name,
    /// Primary card type; artifact creatures remain creatures.
    Type,
    /// Colorless, WUBRG, then multicolor.
    Color,
}

impl HandOrder {
    /// Toolbar order.
    pub const ALL: [Self; 5] = [Self::Draw, Self::Mana, Self::Name, Self::Type, Self::Color];

    /// Short localized button legend.
    pub fn label(self, lang: baylee_client_core::i18n::Lang) -> &'static str {
        use baylee_client_core::i18n::Lang;
        match (self, lang) {
            (Self::Draw, Lang::De) => "Zugfolge",
            (Self::Draw, Lang::En) => "Draw order",
            (Self::Mana, _) => "Mana",
            (Self::Name, _) => "A–Z",
            (Self::Type, Lang::De) => "Typ",
            (Self::Type, Lang::En) => "Type",
            (Self::Color, Lang::De) => "Farbe",
            (Self::Color, Lang::En) => "Color",
        }
    }

    /// Next category for compact toolbars.
    #[must_use]
    pub fn next(self) -> Self {
        Self::ALL[(Self::ALL.iter().position(|&s| s == self).unwrap_or(0) + 1) % Self::ALL.len()]
    }

    /// Bounded category independent of changing playability.
    pub fn group_key(self, card: &HandObject) -> u16 {
        use baylee_core::types::TypeSet;
        match self {
            Self::Draw => 0,
            Self::Mana => u16::try_from(card.mana_value.min(10)).unwrap_or(10),
            Self::Name => card
                .name
                .chars()
                .next()
                .filter(char::is_ascii_alphabetic)
                .map_or(26, |c| c.to_ascii_uppercase() as u16 - u16::from(b'A')),
            Self::Type => [
                TypeSet::LAND,
                TypeSet::CREATURE,
                TypeSet::PLANESWALKER,
                TypeSet::ARTIFACT,
                TypeSet::ENCHANTMENT,
                TypeSet::INSTANT,
                TypeSet::SORCERY,
                TypeSet::BATTLE,
            ]
            .iter()
            .position(|&t| card.types.contains(t))
            .map_or(8, |i| u16::try_from(i).unwrap_or(8)),
            Self::Color => match card.colors.bits() {
                0 => 0,
                1 => 1,
                2 => 2,
                4 => 3,
                8 => 4,
                16 => 5,
                _ => 6,
            },
        }
    }

    /// Localized heading for a category.
    pub fn group_label(self, key: u16, lang: baylee_client_core::i18n::Lang) -> String {
        use baylee_client_core::i18n::Lang;
        match self {
            Self::Draw => self.label(lang).into(),
            Self::Mana => format!("Mana {key}{}", if key == 10 { "+" } else { "" }),
            Self::Name => {
                if key < 26 {
                    char::from_u32(u32::from(b'A') + u32::from(key))
                        .unwrap_or('#')
                        .to_string()
                } else {
                    "#".into()
                }
            }
            Self::Type => {
                let names = if lang == Lang::De {
                    [
                        "Länder",
                        "Kreaturen",
                        "Planeswalker",
                        "Artefakte",
                        "Verzauberungen",
                        "Spontanzauber",
                        "Hexereien",
                        "Schlachten",
                        "Andere",
                    ]
                } else {
                    [
                        "Lands",
                        "Creatures",
                        "Planeswalkers",
                        "Artifacts",
                        "Enchantments",
                        "Instants",
                        "Sorceries",
                        "Battles",
                        "Other",
                    ]
                };
                names[usize::from(key).min(8)].into()
            }
            Self::Color => {
                let names = if lang == Lang::De {
                    [
                        "Farblos",
                        "Weiß",
                        "Blau",
                        "Schwarz",
                        "Rot",
                        "Grün",
                        "Mehrfarbig",
                    ]
                } else {
                    [
                        "Colorless",
                        "White",
                        "Blue",
                        "Black",
                        "Red",
                        "Green",
                        "Multicolor",
                    ]
                };
                names[usize::from(key).min(6)].into()
            }
        }
    }

    /// Reorder presentation cards and return contiguous category boundaries.
    pub fn apply(self, hand: &mut [HandCard], view: &PlayerView) -> Vec<HandGroup> {
        let mut ordered: Vec<_> = view.hand.iter().collect();
        if self != Self::Draw {
            ordered.sort_by(|a, b| {
                let key = self
                    .group_key(a)
                    .cmp(&self.group_key(b))
                    .then_with(|| match self {
                        Self::Mana => a.mana_value.cmp(&b.mana_value),
                        Self::Name => a.name.cmp(&b.name),
                        Self::Type => a.types.bits().cmp(&b.types.bits()),
                        Self::Color => a.colors.bits().cmp(&b.colors.bits()),
                        Self::Draw => std::cmp::Ordering::Equal,
                    });
                key.then_with(|| a.mana_value.cmp(&b.mana_value))
                    .then_with(|| a.name.cmp(&b.name))
                    .then_with(|| a.id.cmp(&b.id))
            });
        }
        let ranks: std::collections::HashMap<_, _> =
            ordered.iter().enumerate().map(|(i, h)| (h.id, i)).collect();
        hand.sort_by_key(|h| ranks.get(&h.id).copied().unwrap_or(usize::MAX));
        let mut groups = Vec::new();
        if self != Self::Draw {
            for (start, card) in ordered.iter().enumerate() {
                let key = self.group_key(card);
                if groups.last().is_none_or(|g: &HandGroup| g.key != key) {
                    groups.push(HandGroup { start, key });
                }
            }
        }
        groups
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use baylee_client_core::test_support::ViewBuilder;
    use baylee_core::{
        color::{Color, ColorSet},
        types::TypeSet,
    };

    #[test]
    fn explicit_groups_survive_rebuild_and_restore_draw_order() {
        let view = ViewBuilder::new(2)
            .with_hand(vec![
                ("Zebra", 3, 1),
                ("Apple", 0, 2),
                ("Birch", 3, 3),
                ("Ash", 1, 4),
            ])
            .build();
        let drawn: Vec<_> = view.hand.iter().map(|h| h.id).collect();
        let mut duel = crate::Duel {
            view: Some(view),
            hand_order: HandOrder::Mana,
            ..Default::default()
        };
        crate::rebuild_board(&mut duel);
        assert_eq!(
            duel.board
                .as_ref()
                .unwrap()
                .hand
                .iter()
                .map(|h| h.name.as_str())
                .collect::<Vec<_>>(),
            ["Apple", "Ash", "Birch", "Zebra"]
        );
        assert_eq!(
            duel.hand_groups
                .iter()
                .map(|g| (g.start, g.key))
                .collect::<Vec<_>>(),
            [(0, 0), (1, 1), (2, 3)]
        );
        crate::rebuild_board(&mut duel);
        assert_eq!(duel.hand_groups.len(), 3);
        assert_eq!(
            duel.view
                .as_ref()
                .unwrap()
                .hand
                .iter()
                .map(|h| h.id)
                .collect::<Vec<_>>(),
            drawn
        );
        duel.hand_order = HandOrder::Draw;
        crate::rebuild_board(&mut duel);
        assert!(duel.hand_groups.is_empty());
        assert_eq!(
            duel.board
                .unwrap()
                .hand
                .iter()
                .map(|h| h.id)
                .collect::<Vec<_>>(),
            drawn
        );
    }

    #[test]
    fn multitype_and_multicolor_cards_have_one_predictable_group() {
        let mut view = ViewBuilder::new(2)
            .with_hand(vec![("Golem", 12, 1)])
            .build();
        let card = &mut view.hand[0];
        card.types = TypeSet::ARTIFACT.union(TypeSet::CREATURE);
        card.colors = ColorSet::of(Color::Blue).union(ColorSet::of(Color::Red));
        assert_eq!(HandOrder::Type.group_key(card), 1);
        assert_eq!(HandOrder::Color.group_key(card), 6);
        assert_eq!(HandOrder::Mana.group_key(card), 10);
        assert_eq!(HandOrder::Name.group_key(card), 6);
    }
}
