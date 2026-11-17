//! Public run-item identities in the log use the same portrait and details as
//! the shop. Never infer an item from a displayed name or look at current zones.
use super::{Commands, Entity, Lang, LogLine, LogLink, UiFonts};
use baylee_client_core::echoes::domain::v2::shop::Goods;
use bevy::prelude::*;

/// Marks a `[Item Name]` [`TextSpan`] in a game-log sentence so hovering the
/// item's name displays the rich Tower Item Detail Overlay anchored to `row`.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct LogItemSpan {
    /// Authoritative public item identity recorded on the log entry.
    pub goods: Goods,
    /// The laid-out log row [`Node`] that anchors the detail overlay.
    pub row: Entity,
}

pub(super) fn goods(link: LogLink) -> Option<Goods> {
    link.card.is_none().then_some(())?;
    crate::itemart::identity::run_token(link.token?)
}

pub(super) fn portraits(
    commands: &mut Commands,
    fonts: &UiFonts,
    lang: Lang,
    line: &LogLine,
    row: Entity,
    text_size: f32,
) {
    // A log entry can mention the same source repeatedly. Four distinct icons
    // are enough for its margin; every name remains in the complete sentence.
    let mut shown = [None; 4];
    let mut count = 0;
    for item in line.names.iter().filter_map(LogLink::of).filter_map(goods) {
        if shown[..count].contains(&Some(item)) {
            continue;
        }
        shown[count] = Some(item);
        count += 1;
        let size = text_size * 1.6;
        let icon = commands
            .spawn((
                crate::itemart::Portrait(item),
                Node {
                    width: crate::shellkit::px_fixed(size),
                    height: crate::shellkit::px_fixed(size),
                    flex_shrink: 0.0,
                    ..default()
                },
                Pickable::default(),
            ))
            .id();
        crate::hud::tower::attach_item_tooltip(commands, icon, item, fonts, lang);
        commands.entity(row).add_child(icon);
        if count == shown.len() {
            break;
        }
    }
}

/// Marks the log row currently wearing a span-hovered Item Detail Overlay.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct LogHoveredItem(pub Goods);

/// Attaches the rich Tower Item Detail Overlay to a log row while the pointer
/// rests on a [`LogItemSpan`] inside that row, and removes it cleanly when the
/// pointer leaves or the log hides. Writes nothing at rest.
#[allow(clippy::too_many_arguments)]
pub(crate) fn hover_log_items(
    mut commands: Commands,
    hovers: Option<Res<bevy::picking::hover::HoverMap>>,
    spans: Query<&LogItemSpan>,
    active: Query<(Entity, &LogHoveredItem)>,
    visibilities: Query<&Visibility, Without<crate::shellkit::controls::FloatingTooltip>>,
    nodes: Query<&Node, Without<crate::shellkit::controls::FloatingTooltip>>,
    parents: Query<&ChildOf>,
    fonts: Option<Res<UiFonts>>,
    settings: Option<Res<crate::settings::ClientSettings>>,
) {
    let wanted = hovers.as_deref().and_then(|map| {
        for hits in map.values() {
            for &hit in hits.keys() {
                if let Ok(item) = spans.get(hit)
                    && crate::shellkit::controls::tooltip_owner_visible(
                        hit,
                        &parents,
                        &visibilities,
                        &nodes,
                    )
                {
                    return Some((item.row, item.goods));
                }
            }
        }
        None
    });
    let current = active.iter().next().map(|(row, item)| (row, item.0));
    if current == wanted && active.iter().count() <= 1 {
        return;
    }
    for (old_row, _) in &active {
        commands.entity(old_row).try_remove::<(
            LogHoveredItem,
            crate::shellkit::controls::Tooltip,
            crate::shellkit::controls::FocusTooltip,
        )>();
    }
    if let (Some((row, goods)), Some(fonts)) = (wanted, fonts.as_deref()) {
        let lang = settings.as_deref().map_or(Lang::En, |s| Lang::of(&s.lang));
        commands.entity(row).try_insert(LogHoveredItem(goods));
        crate::hud::tower::attach_item_tooltip(&mut commands, row, goods, fonts, lang);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::ecs::system::RunSystemOnce;

    #[test]
    fn log_items_use_public_identity_and_shared_details_instead_of_fake_card_links() {
        let mut line = super::super::tests::line("Potion Potion", &[(0, 6), (7, 13)], 1);
        let token = baylee_cards::tokens::token_id(&baylee_cards::tokens::TOWER_HEALING_POTION);
        for name in &mut line.names {
            name.token = Some(token);
        }
        let mut world = World::new();
        let fonts = UiFonts::default();
        let row = super::super::spawn_line(
            &mut world.commands(),
            &fonts,
            Lang::En,
            &line,
            &super::super::PANEL_INKS,
            None,
        );
        world.flush();
        assert!(world.get_entity(row).is_ok());
        assert_eq!(world.query::<&LogLink>().iter(&world).count(), 0);
        // Both mentions carry LogItemSpan so hovering either item name shows the Item Detail Overlay.
        assert_eq!(world.query::<&LogItemSpan>().iter(&world).count(), 2);
        // One margin icon and its shared tooltip's larger portrait, not two
        // duplicate margin icons for two mentions of the same item.
        assert_eq!(
            world
                .query::<&crate::itemart::Portrait>()
                .iter(&world)
                .count(),
            2
        );
        assert!(
            goods(LogLink {
                card: None,
                token: None
            })
            .is_none()
        );
        assert!(
            goods(LogLink {
                card: None,
                token: Some(u16::MAX)
            })
            .is_none()
        );
        assert!(
            goods(LogLink {
                card: Some(super::super::tests::bolt_identity()),
                token: Some(token)
            })
            .is_none()
        );

        // Hovering the item's TextSpan attaches the rich Item Detail Overlay to `row`
        // and cleans it up when the pointer leaves.
        let span_entity = world
            .query_filtered::<Entity, With<LogItemSpan>>()
            .iter(&world)
            .next()
            .expect("item span");
        world.insert_resource(fonts);
        let mut hover_map = bevy::picking::hover::HoverMap::default();
        hover_map.insert(
            bevy::picking::pointer::PointerId::Mouse,
            [(
                span_entity,
                bevy::picking::backend::HitData::new(Entity::PLACEHOLDER, 0.0, None, None),
            )]
            .into(),
        );
        world.insert_resource(hover_map);
        world.run_system_once(hover_log_items).unwrap();
        world.flush();
        assert!(
            world
                .get::<crate::shellkit::controls::Tooltip>(row)
                .is_some(),
            "hovering item name in game log attaches Item Detail Overlay Tooltip to row"
        );
        world
            .resource_mut::<bevy::picking::hover::HoverMap>()
            .clear();
        world.run_system_once(hover_log_items).unwrap();
        world.flush();
        assert!(
            world
                .get::<crate::shellkit::controls::Tooltip>(row)
                .is_none(),
            "leaving item name removes Tooltip cleanly"
        );
    }
}
