//! A retained, scrollable queue of every seat's publicly suspended cards.
use super::{UiFonts, palette, tf, tf_bold};
use crate::{Duel, settings::ClientSettings};
use baylee_client_core::{BrowseZone, Lang, i18n::Phrase, suspended};
use baylee_core::ids::PlayerId;
use bevy::prelude::*;

#[derive(Component)]
pub(crate) struct Queue;

pub(crate) fn sync(
    mut commands: Commands,
    duel: Res<Duel>,
    settings: Res<ClientSettings>,
    fonts: Res<UiFonts>,
    texts: Res<crate::cardtext::CardTexts>,
    roots: Query<Entity, With<Queue>>,
    mut previous: Local<Vec<(PlayerId, String)>>,
) {
    let lang = Lang::of(&settings.lang);
    let rows: Vec<_> = duel
        .view
        .as_ref()
        .into_iter()
        .flat_map(|view| {
            suspended::read(view).into_iter().filter_map(|row| {
                let object = view.object(row.object)?;
                let owner = duel.statics.as_ref().map_or_else(
                    || format!("{} {}", Phrase::ASeat.text(lang), row.owner.get() + 1),
                    |statics| statics.seat_name(row.owner).to_string(),
                );
                Some((
                    row.owner,
                    Phrase::SuspendedCard.fill(
                        lang,
                        &[
                            &crate::face::name_of(object, view, &texts),
                            &owner,
                            &row.counters.to_string(),
                        ],
                    ),
                ))
            })
        })
        .collect();
    if *previous == rows && (rows.is_empty() || !roots.is_empty()) {
        return;
    }
    for entity in &roots {
        commands.entity(entity).despawn();
    }
    previous.clone_from(&rows);
    if rows.is_empty() {
        return;
    }
    let root = commands
        .spawn((
            Queue,
            super::DetachedHud,
            Node {
                position_type: PositionType::Absolute,
                left: px(12),
                top: px(12),
                width: px(300),
                max_width: percent(40),
                max_height: percent(28),
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(px(10)),
                row_gap: px(6),
                border_radius: BorderRadius::all(px(8)),
                overflow: Overflow::scroll_y(),
                ..default()
            },
            BackgroundColor(palette::DIALOG),
            ScrollPosition::default(),
            super::scroll::Scrolls,
            GlobalZIndex(-1),
        ))
        .id();
    commands.entity(root).with_children(|parent| {
        parent.spawn((
            Text::new(Phrase::SuspendedCards.text(lang)),
            tf_bold(&fonts, 16.0),
            TextColor(palette::CANDLE),
            Pickable::IGNORE,
        ));
        for (owner, label) in rows {
            parent
                .spawn((
                    Button,
                    Node {
                        width: percent(100),
                        padding: UiRect::all(px(6)),
                        flex_shrink: 0.0,
                        border_radius: BorderRadius::all(px(4)),
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.18, 0.16, 0.11)),
                ))
                .with_children(|row| {
                    row.spawn((
                        Text::new(label),
                        tf(&fonts, 15.0),
                        TextColor(palette::CANDLE),
                        Pickable::IGNORE,
                    ));
                })
                .observe(
                    move |mut click: On<Pointer<Click>>, mut duel: ResMut<Duel>| {
                        click.propagate(false);
                        duel.browser.open_at(BrowseZone::Exile(owner));
                    },
                );
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use baylee_client_core::test_support::ViewBuilder;
    use baylee_view::{CounterEntry, CounterKind};

    #[test]
    fn queue_updates_counts_and_disappears_but_does_not_rebuild_for_hover() {
        let mut app = App::new();
        let mut view = ViewBuilder::new(3).build();
        let mut card = crate::registry_printed(4, 2, "Ancestral Vision");
        card.suspended = true;
        card.counters.push(CounterEntry {
            kind: CounterKind::Time,
            count: 4,
        });
        view.exile[2].push(card);
        app.insert_resource(Duel {
            view: Some(view),
            ..default()
        })
        .insert_resource(UiFonts {
            text: default(),
            medium: default(),
            bold: default(),
            italic: default(),
            medium_italic: default(),
            serif: default(),
            serif_italic: default(),
            icons: default(),
            mana: default(),
        })
        .init_resource::<crate::cardtext::CardTexts>()
        .init_resource::<ClientSettings>()
        .add_systems(Update, sync);
        app.update();
        let mut roots = app.world_mut().query_filtered::<Entity, With<Queue>>();
        let first = roots.single(app.world()).unwrap();
        let mut words = app.world_mut().query::<&Text>();
        assert!(
            words
                .iter(app.world())
                .any(|t| t.0.contains("4 time counters"))
        );
        app.world_mut().resource_mut::<Duel>().hovered =
            Some(baylee_core::ids::ObjectId::new(4, 0));
        app.update();
        assert_eq!(
            roots.single(app.world()).unwrap(),
            first,
            "hover must preserve scroll position"
        );
        app.world_mut()
            .resource_mut::<Duel>()
            .view
            .as_mut()
            .unwrap()
            .exile[2][0]
            .counters[0]
            .count = 3;
        app.world_mut().resource_mut::<ClientSettings>().lang = "de".to_string();
        app.update();
        assert!(
            words
                .iter(app.world())
                .any(|t| t.0.contains("3 Zeitmarken"))
        );
        assert!(
            !words
                .iter(app.world())
                .any(|t| t.0.contains("4 time counters"))
        );
        app.world_mut()
            .resource_mut::<Duel>()
            .view
            .as_mut()
            .unwrap()
            .exile[2]
            .clear();
        app.update();
        assert_eq!(roots.iter(app.world()).count(), 0);
    }
}
