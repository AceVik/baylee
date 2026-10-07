//! The settings screen's rows and controls on the kit (`DESIGN-v5` §12):
//! label, help with its cost in words, the control, and the storage tag.
//!
//! Every control carries its `Press` and its stop in the screen's
//! `TabOrder` (`settingsui::keys::SETTINGS`); a row the account keeps is
//! read-only before sign-in ("Sign in to change").

use baylee_client_core::i18n::{Lang, Phrase};
use baylee_client_core::settings_map::{self, Row, Scope};
use bevy::prelude::*;

use crate::lobby::Press;
use crate::shellkit::controls::{self, Kit, Live, Weight};
use crate::shellkit::focus::{Current, Stop};
use crate::shellkit::metrics::px_fixed;
use crate::shellkit::role::Role;
use crate::shellkit::surfaces::{self, Storage};
use crate::shellkit::tokens;

use super::keys::SETTINGS;

/// What the screen's builders share: the kit, the language, whether the
/// account's rows can be written, and the column the rows go into.
pub(crate) struct Out<'a, 'w, 's> {
    /// Where nodes are spawned.
    pub(crate) commands: &'a mut Commands<'w, 's>,
    /// The kit.
    pub(crate) kit: Kit<'a>,
    /// The language.
    pub(crate) lang: Lang,
    /// Signed in: the account's rows can be written.
    pub(crate) signed_in: bool,
    /// The column the next row goes into.
    pub(crate) column: Entity,
}

/// A stop of the settings screen.
#[must_use]
pub(crate) const fn stop(id: &'static str) -> Stop {
    Stop::new(SETTINGS.name, id)
}

/// Item `i` of the settings screen's composite `id`.
#[must_use]
pub(crate) fn item(id: &'static str, i: usize) -> Stop {
    Stop::item(SETTINGS.name, id, u8::try_from(i).unwrap_or(u8::MAX))
}

impl Out<'_, '_, '_> {
    /// A row from the map: its label, help and tag, and `control` — or,
    /// for the account's row before sign-in, "Sign in to change".
    pub(crate) fn row(&mut self, row: Row, control: Entity) -> Entity {
        let def = settings_map::of(row);
        let control = if def.scope == Scope::Account && !self.signed_in {
            self.commands.entity(control).despawn();
            controls::label(
                self.commands,
                self.kit,
                Phrase::SettingsSignInToChange.text(self.lang),
                self.kit.m.small,
                tokens::MUTED,
            )
        } else {
            control
        };
        let storage = match def.scope {
            Scope::Account => Storage::Account,
            Scope::Device | Scope::Machine => Storage::Device,
        };
        let line = surfaces::row(
            self.commands,
            self.kit,
            def.label.text(self.lang),
            Some(def.help.text(self.lang)),
            control,
            Some((storage, def.scope.tag().text(self.lang))),
        );
        self.commands.entity(line).insert(RowMark(row));
        self.commands.entity(self.column).add_child(line);
        line
    }

    /// A row whose control is a column of things (a list of keys, the
    /// rails): its label, help and tag over them, the column under.
    pub(crate) fn block(&mut self, row: Row, body: &[Entity]) -> Entity {
        let holder = self
            .commands
            .spawn((
                Node {
                    flex_direction: FlexDirection::Column,
                    width: Val::Percent(100.0),
                    row_gap: px_fixed(self.kit.m.gap),
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .id();
        let def = settings_map::of(row);
        let readable = def.scope != Scope::Account || self.signed_in;
        let empty = self
            .commands
            .spawn((Node::default(), Pickable::IGNORE))
            .id();
        let column = self.column;
        self.column = holder;
        let head = self.row(row, empty);
        self.column = column;
        let _ = head;
        if readable {
            self.commands.entity(holder).add_children(body);
        } else {
            for entity in body {
                self.commands.entity(*entity).despawn();
            }
        }
        self.commands.entity(self.column).add_child(holder);
        holder
    }

    /// A segmented control: `labels`, `selected` filled, item `i` pressing
    /// `press(i)`.
    pub(crate) fn seg(
        &mut self,
        id: &'static str,
        labels: &[&str],
        selected: Option<usize>,
        press: impl Fn(usize) -> Press,
    ) -> Entity {
        let at = selected.unwrap_or(usize::MAX);
        controls::segmented(self.commands, self.kit, labels, at, |i| {
            (press(i), item(id, i), Current(i == at))
        })
    }

    /// A toggle.
    pub(crate) fn toggle(&mut self, id: &'static str, on: bool, press: Press) -> Entity {
        controls::toggle(self.commands, self.kit, on, (press, stop(id)))
    }

    /// A button.
    pub(crate) fn button(
        &mut self,
        id: &'static str,
        text: &str,
        weight: Weight,
        press: Press,
    ) -> Entity {
        controls::button(
            self.commands,
            self.kit,
            text,
            weight,
            Live::Yes,
            None,
            (press, stop(id)),
        )
    }

    /// Item `i` of a composite of buttons (a list of keys, of profiles).
    pub(crate) fn button_item(
        &mut self,
        id: &'static str,
        i: usize,
        text: &str,
        weight: Weight,
        press: Press,
    ) -> Entity {
        controls::button(
            self.commands,
            self.kit,
            text,
            weight,
            Live::Yes,
            None,
            (press, item(id, i)),
        )
    }

    /// A slider, read back by `settingsui::keys::apply_sliders`.
    pub(crate) fn slider(&mut self, id: &'static str, value: u8, which: Volume) -> Entity {
        controls::slider(self.commands, self.kit, value, (which, stop(id)))
    }

    /// A line of words, muted or not.
    pub(crate) fn words(&mut self, text: &str, muted: bool) -> Entity {
        surfaces::prose(self.commands, self.kit, text, muted)
    }

    /// A rule across the panel with a caption on it (Graphics' account
    /// rows, M4-3).
    pub(crate) fn caption_rule(&mut self, text: &str) {
        let line = self
            .commands
            .spawn((
                Node {
                    width: Val::Percent(100.0),
                    align_items: AlignItems::Center,
                    column_gap: self.kit.m.px(12.0),
                    margin: UiRect::vertical(self.kit.m.px(6.0)),
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .id();
        let rule = |commands: &mut Commands| {
            commands
                .spawn((
                    Node {
                        height: px_fixed(1.0),
                        flex_grow: 1.0,
                        min_width: px_fixed(12.0),
                        ..default()
                    },
                    BackgroundColor(tokens::BORDER),
                    Pickable::IGNORE,
                ))
                .id()
        };
        let left = rule(self.commands);
        let words = self
            .commands
            .spawn((
                Text::new(text),
                crate::hud::tf(self.kit.fonts, self.kit.m.small),
                TextColor(tokens::INK),
                TextLayout::justify(Justify::Center),
                Node {
                    flex_shrink: 1.0,
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .id();
        let right = rule(self.commands);
        self.commands
            .entity(line)
            .add_children(&[left, words, right]);
        self.commands.entity(self.column).add_child(line);
    }

    /// A wrapping run of controls under a block.
    pub(crate) fn wrap(&mut self, items: &[Entity]) -> Entity {
        let run = self
            .commands
            .spawn((
                Node {
                    width: Val::Percent(100.0),
                    flex_wrap: FlexWrap::Wrap,
                    column_gap: self.kit.m.px(8.0),
                    row_gap: self.kit.m.px(6.0),
                    align_items: AlignItems::Center,
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .id();
        self.commands.entity(run).add_children(items);
        run
    }

    /// One line of a list inside a block: words that grow, controls at the
    /// end.
    pub(crate) fn line(&mut self, words: &str, help: Option<&str>, controls: &[Entity]) -> Entity {
        let line = self
            .commands
            .spawn((
                Role::Row,
                Node {
                    width: Val::Percent(100.0),
                    min_height: px_fixed(self.kit.m.hit),
                    align_items: AlignItems::Center,
                    column_gap: self.kit.m.px(8.0),
                    flex_wrap: FlexWrap::Wrap,
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .id();
        let column = self
            .commands
            .spawn((
                Node {
                    flex_direction: FlexDirection::Column,
                    flex_grow: 1.0,
                    flex_shrink: 1.0,
                    flex_basis: self.kit.m.px(160.0),
                    min_width: px_fixed(0.0),
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .id();
        let name = surfaces::prose(self.commands, self.kit, words, false);
        self.commands.entity(column).add_child(name);
        if let Some(help) = help {
            let help = surfaces::prose(self.commands, self.kit, help, true);
            self.commands.entity(column).add_child(help);
        }
        self.commands.entity(line).add_child(column);
        self.commands.entity(line).add_children(controls);
        line
    }
}

/// Which row a node is, for the search's jump and the dev-control dump.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct RowMark(pub(crate) Row);

/// Which volume a slider sets.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Volume {
    /// Every sound (`GlobalVolume` through the mix).
    Master,
    /// The music's own level.
    Music,
    /// The table's cues, under the account's ceiling.
    Effects,
}
