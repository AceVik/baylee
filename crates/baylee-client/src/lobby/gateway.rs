//! Local gateway selection, independent of account-owned preferences.
#[allow(clippy::wildcard_imports)] // the lobby widget vocabulary
use super::*;
use baylee_client_core::lobby::gateway_info::Warning;

/// Accept only an HTTP(S) origin/base path, with no embedded credentials.
pub(super) fn normalize(value: &str) -> Option<String> {
    let url = url::Url::parse(value.trim()).ok()?;
    if !matches!(url.scheme(), "http" | "https")
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return None;
    }
    Some(url.as_str().trim_end_matches('/').to_string())
}

impl LobbyState {
    pub(super) fn select_gateway(&mut self, index: usize) -> bool {
        let Some(url) = self.gateways.get(index).cloned() else {
            return false;
        };
        self.gateway_epoch = self.gateway_epoch.wrapping_add(1);
        let lang = self.lobby.lang();
        // A fresh lobby, because nothing another gateway said is true of this
        // one. The username typed or remembered is the player's and not the
        // gateway's, so it comes along: without it the one launch-to-launch
        // convenience the sign-in form has was gone the moment a gateway was
        // chosen, which is now always before the form is seen.
        let username = self.lobby.field(Field::Username).to_string();
        self.lobby = Lobby::new();
        self.lobby.set_lang(lang);
        self.lobby.set_field(Field::Username, &username);
        self.art_cache = false;
        self.gateway = url;
        self.gateway_selected = true;
        self.gateway_cursor = None;
        self.front_menu = false;
        self.lobby.set_gateway_ready(true);
        self.lobby.set_registration_enabled(false);
        true
    }

    /// Back from the account form to the gateway form.
    ///
    /// The epoch moves as it does on a new choice, so nothing the gateway
    /// left behind still answers into the form. Choosing the same gateway
    /// again is one tap, and the address typed into the form survives it.
    pub(super) fn leave_gateway(&mut self) {
        self.gateway_epoch = self.gateway_epoch.wrapping_add(1);
        self.gateway_selected = false;
        self.front_menu = false;
        self.lobby.set_gateway_ready(false);
        // The list is off screen until now, so this is when a sign-in made
        // since it was last drawn may move a gateway up.
        self.order_gateways();
    }

    /// Orders the saved gateways by use (see
    /// [`baylee_client_core::lobby::gateway_use::GatewayUses::order`]). Only
    /// while the list is not on screen: the rows are pressed by index.
    fn order_gateways(&mut self) {
        self.uses.order(&mut self.gateways);
        self.gateway_cursor = None;
    }

    /// Removes a saved gateway, with what this device knew about it.
    pub(super) fn forget_gateway(&mut self, url: &str) {
        self.gateways.retain(|saved| saved != url);
        self.probes.remove(url);
        self.uses.forget(url);
        self.gateway_cursor = None;
    }

    /// Moves the arrow keys' row one down, or up. From none, down lands on
    /// the first row and up on the last; at either end it stays.
    pub(super) fn move_gateway_cursor(&mut self, down: bool) {
        let Some(last) = self.gateways.len().checked_sub(1) else {
            return;
        };
        self.gateway_cursor = Some(match (self.gateway_cursor, down) {
            (None, true) => 0,
            (None, false) => last,
            (Some(at), true) => (at + 1).min(last),
            (Some(at), false) => at.saturating_sub(1),
        });
    }

    /// Starts asking the typed address about itself, if it is an address.
    ///
    /// Answers the address to ask. Nothing is saved yet: that waits for
    /// [`Self::gateway_answered`], so an address nobody answers at is caught
    /// while the player is still looking at what they typed. Nothing while
    /// an answer is still owed: the Save button is disabled then, and Enter
    /// in the address field comes through here too.
    pub(super) fn check_gateway(&mut self) -> Option<String> {
        if self.adding.is_some() {
            return None;
        }
        let Some(url) = normalize(self.lobby.field(Field::Gateway)) else {
            self.lobby.tell_refusal(Phrase::GatewayUrlInvalid, &[]);
            return None;
        };
        self.lobby.tell(Phrase::GatewayChecking, &[&url]);
        self.probes.insert(url.clone(), Probe::Asking);
        self.adding = Some(url.clone());
        Some(url)
    }

    /// Files what an address said. Answers whether that saved a new address,
    /// which is when the settings file has to be written.
    pub(super) fn gateway_answered(&mut self, url: String, probe: Probe) -> bool {
        let found = probe.is_gateway();
        self.probes.insert(url.clone(), probe);
        if self.adding.as_ref() != Some(&url) {
            return false;
        }
        self.adding = None;
        if !found {
            // The field keeps what was typed: the likeliest fix is one
            // character, and it is easier made than retyped.
            self.lobby.tell_refusal(Phrase::GatewayNotFound, &[&url]);
            return false;
        }
        self.lobby.tell(Phrase::GatewaySaved, &[&url]);
        self.lobby.set_field(Field::Gateway, "");
        if self.gateways.contains(&url) {
            return false;
        }
        self.gateways.push(url);
        true
    }

    /// The saved addresses nobody has asked yet, marked as being asked.
    pub(super) fn unasked_gateways(&mut self) -> Vec<String> {
        let unasked: Vec<String> = self
            .gateways
            .iter()
            .filter(|url| !self.probes.contains_key(*url))
            .cloned()
            .collect();
        for url in &unasked {
            self.probes.insert(url.clone(), Probe::Asking);
        }
        unasked
    }
}

/// Where a gateway row's words come from, and what colour each is.
struct RowWords {
    /// The operator's name, or the address when there is none.
    title: String,
    /// The address, under a name; nothing when the title already is it.
    address: Option<String>,
    /// The version as a row has room for, `v0.1.0`, or what stands in for
    /// one.
    version: String,
    /// What the version says in full, for the hint over it.
    version_in_full: String,
    /// The version's ink: what it means for this client.
    ink: Color,
    /// Whether the gateway answered, as the dot's colour.
    reach: Color,
    /// What the dot says, pointed at.
    reach_said: String,
    /// What is wrong with it for this client, which is the warning mark.
    /// Only a version can be: a gateway that is not answering says so with
    /// its dot, and a mark as well would be one fact said twice.
    warning: Option<Warning>,
}

fn row_words(url: &str, probe: Option<&Probe>, lang: Lang) -> RowWords {
    let warning = probe
        .and_then(|p| p.warning(baylee_protocol::PROTOCOL_VERSION, baylee_view::VIEW_VERSION))
        .filter(|w| *w != Warning::Silent);
    let ink = match warning {
        None if matches!(probe, Some(Probe::Known(_))) => palette::HEAL,
        None => palette::MUTED,
        Some(w) if w.refuses_games() => palette::DANGER,
        Some(_) => palette::ACTIVE,
    };
    let (reach, reach_said) = match probe {
        Some(Probe::Known(_) | Probe::Older) => (palette::HEAL, Phrase::GatewayAnswering),
        Some(Probe::Silent) => (palette::DANGER, Phrase::GatewayNotAnswering),
        Some(Probe::Asking) | None => (palette::MUTED, Phrase::GatewayCheckingShort),
    };
    let (name, version, version_in_full) = match probe {
        Some(Probe::Known(info)) => (
            info.name.clone(),
            info.short_version(),
            info.version.clone(),
        ),
        Some(Probe::Older) => (
            None,
            "v?".to_string(),
            Phrase::GatewayVersionUnknown.text(lang).to_string(),
        ),
        Some(Probe::Silent) => (
            None,
            "—".to_string(),
            Phrase::GatewayNotAnswering.text(lang).to_string(),
        ),
        Some(Probe::Asking) | None => (
            None,
            "…".to_string(),
            Phrase::GatewayCheckingShort.text(lang).to_string(),
        ),
    };
    let (title, address) = match name {
        Some(name) => (name, Some(url.to_string())),
        None => (url.to_string(), None),
    };
    RowWords {
        title,
        address,
        version,
        version_in_full,
        ink,
        reach,
        reach_said: reach_said.text(lang).to_string(),
        warning,
    }
}

/// How tall a gateway row is: two lines and the room around them, and a
/// finger's height on a phone.
fn row_height(metrics: Metrics) -> f32 {
    if metrics.frame == Frame::Phone {
        metrics.tap + 12.0
    } else {
        56.0
    }
}

/// The room between two rows.
const ROW_GAP: f32 = 8.0;

/// How many rows are in sight before the list scrolls.
pub(super) const ROWS_IN_SIGHT: usize = 4;

/// The width of a row's warning cell, reserved on every row so that the
/// versions and bins of all rows stand in one column.
fn mark_width(metrics: Metrics) -> f32 {
    if metrics.frame == Frame::Phone {
        metrics.tap
    } else {
        34.0
    }
}

/// The side of a row's bin.
fn bin_side(metrics: Metrics) -> f32 {
    if metrics.frame == Frame::Phone {
        32.0
    } else {
        24.0
    }
}

/// The saved gateways, in the order [`LobbyState::uses`] gave them at launch.
///
/// Four rows are in sight. A fifth and more scroll inside the same height,
/// with a scrollbar beside them, so a long list cannot grow the card it
/// stands on; the wheel, a finger and the arrow keys all move it, and the
/// row the arrows are on is kept in sight.
pub(super) fn list(
    commands: &mut Commands,
    state: &LobbyState,
    fonts: &UiFonts,
    metrics: Metrics,
    scrolled_to: &Scrolled,
) -> Entity {
    let row = row_height(metrics);
    let in_sight = ROWS_IN_SIGHT as f32;
    let height = in_sight * row + (in_sight - 1.0) * ROW_GAP;
    let scrolls = state.gateways.len() > ROWS_IN_SIGHT;
    let offset = in_view(
        scrolled_to.get(List::Gateways),
        state.gateway_cursor,
        row,
        height,
    );
    let list = commands
        .spawn((
            Scrollable(List::Gateways),
            ScrollPosition(Vec2::new(0.0, offset)),
            Node {
                width: percent(100),
                flex_grow: 1.0,
                flex_shrink: 0.0,
                flex_direction: FlexDirection::Column,
                row_gap: px(ROW_GAP),
                overflow: Overflow::scroll_y(),
                ..default()
            },
        ))
        .id();
    for (index, url) in state.gateways.iter().enumerate() {
        let row = gateway_row(commands, state, fonts, metrics, index, url);
        commands.entity(list).add_child(row);
    }
    if !scrolls {
        return list;
    }
    // The frame holds the height; the scrollbar's host fills it and puts the
    // track beside the rows.
    let frame = commands
        .spawn((
            Node {
                width: percent(100),
                height: px(height),
                flex_shrink: 0.0,
                flex_direction: FlexDirection::Column,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    super::scrollbars::attach(commands, frame, list, metrics);
    frame
}

/// The list's scroll offset with the row at `cursor` in sight: unchanged
/// when it already is, else just far enough to show the whole row.
fn in_view(offset: f32, cursor: Option<usize>, row: f32, height: f32) -> f32 {
    let Some(index) = cursor else {
        return offset;
    };
    #[allow(clippy::cast_precision_loss)] // a list of saved addresses
    let top = index as f32 * (row + ROW_GAP);
    if top < offset {
        top
    } else if top + row > offset + height {
        top + row - height
    } else {
        offset
    }
}

/// One saved gateway:
/// ```text
///  ● Name                        [bin]  [!]
///    address                    v0.1.0
/// ```
/// A dot for whether it answered, the name over the address, and in the
/// right column the bin over the short version. The warning mark, when there
/// is one, stands in a cell of its own at the right end, reserved on every
/// row so the columns line up.
///
/// The bin and the mark lie over the row as its siblings and not inside it,
/// so pressing the one or holding a finger on the other to read why does not
/// also choose the gateway. The version and the dot are inside, and let a
/// press through to the row: pointing at them only explains them.
fn gateway_row(
    commands: &mut Commands,
    state: &LobbyState,
    fonts: &UiFonts,
    metrics: Metrics,
    index: usize,
    url: &str,
) -> Entity {
    let lang = state.lobby.lang();
    let words = row_words(url, state.probes.get(url), lang);
    let enabled = !state.lobby.busy() && state.adding.is_none();
    let marked = state.gateway_cursor == Some(index);
    // No label of its own: the row lays its words out in two lines, which a
    // button's one label cannot.
    let control = button(
        commands,
        fonts,
        metrics,
        "",
        Press::SelectGateway(index),
        palette::PANEL,
        enabled,
    );
    let mark_w = mark_width(metrics);
    commands
        .entity(control)
        .entry::<Node>()
        .and_modify(move |mut node| {
            node.flex_grow = 1.0;
            node.min_width = px(0);
            node.min_height = px(row_height(metrics));
            node.column_gap = px(0);
            node.padding = UiRect {
                left: px(12),
                right: px(mark_w),
                top: px(6),
                bottom: px(6),
            };
        });
    if marked {
        // The arrow keys' row: lit as a tab that is open is, and edged in
        // the caret's colour, since it is where Enter goes.
        commands.entity(control).insert((
            BackgroundColor(palette::PANEL_HOT),
            BorderColor::all(palette::ACCENT),
            crate::ambience::Feel::new(palette::PANEL_HOT),
        ));
    }
    let dot = reach_dot(commands, &words);
    commands.entity(dot).insert(passes_presses());
    commands.entity(dot).entry::<Node>().and_modify(|mut node| {
        node.margin = UiRect::right(px(12));
    });
    let texts = row_texts(commands, fonts, metrics, &words);
    // The right column: room for the bin on top, the version at the foot.
    let side = commands
        .spawn((
            Node {
                flex_shrink: 0.0,
                align_self: AlignSelf::Stretch,
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::FlexEnd,
                justify_content: JustifyContent::FlexEnd,
                margin: UiRect::left(px(12)),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let version = commands
        .spawn((
            Text::new(words.version.clone()),
            tf(fonts, metrics.small),
            TextColor(words.ink),
            super::hint::HoverHint(words.version_in_full.clone()),
            passes_presses(),
        ))
        .id();
    commands.entity(side).add_child(version);
    commands.entity(control).add_children(&[dot, texts, side]);

    let row = commands
        .spawn((
            Node {
                width: percent(100),
                flex_shrink: 0.0,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(row).add_child(control);
    let bin = bin(commands, fonts, metrics, lang, index, enabled);
    commands.entity(row).add_child(bin);
    if let Some(mark) = mark_cell(commands, &words, fonts, metrics, lang) {
        commands.entity(row).add_child(mark);
    }
    row
}

/// Whether a gateway answered, as a dot that says so when pointed at.
fn reach_dot(commands: &mut Commands, words: &RowWords) -> Entity {
    commands
        .spawn((
            Node {
                width: px(8),
                height: px(8),
                flex_shrink: 0.0,
                border_radius: BorderRadius::all(px(4)),
                ..default()
            },
            BackgroundColor(words.reach),
            super::hint::HoverHint(words.reach_said.clone()),
        ))
        .id()
}

/// A row's name over its address.
fn row_texts(
    commands: &mut Commands,
    fonts: &UiFonts,
    metrics: Metrics,
    words: &RowWords,
) -> Entity {
    let texts = commands
        .spawn((
            Node {
                flex_grow: 1.0,
                min_width: px(0),
                flex_direction: FlexDirection::Column,
                row_gap: px(2),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let title = commands
        .spawn((
            Text::new(words.title.clone()),
            crate::hud::tf_bold(fonts, metrics.text),
            TextColor(palette::INK),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(texts).add_child(title);
    if let Some(address) = &words.address {
        let address = commands
            .spawn((
                Text::new(address.clone()),
                tf(fonts, metrics.small),
                TextColor(palette::MUTED),
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(texts).add_child(address);
    }
    texts
}

/// Hover without hold: pointing explains the thing, and a press goes on to
/// the row under it.
fn passes_presses() -> Pickable {
    Pickable {
        should_block_lower: false,
        is_hoverable: true,
    }
}

/// A row's bin, at the top of its right column: asks, then removes the
/// gateway from this device's list.
fn bin(
    commands: &mut Commands,
    fonts: &UiFonts,
    metrics: Metrics,
    lang: Lang,
    index: usize,
    enabled: bool,
) -> Entity {
    let side = bin_side(metrics);
    let bin = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                top: px(4),
                right: px(mark_width(metrics) - side * 0.5 + 4.0),
                width: px(side),
                height: px(side),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                border_radius: BorderRadius::all(px(4)),
                ..default()
            },
            BackgroundColor(Color::NONE),
            super::hint::HoverHint(Phrase::ForgetGateway.text(lang).to_string()),
        ))
        .id();
    if enabled {
        commands.entity(bin).insert((
            Press::ForgetGateway(index),
            crate::ambience::Feel::rising_to(Color::NONE, palette::PANEL_HOT),
        ));
    } else {
        commands.entity(bin).insert(Pickable::IGNORE);
    }
    let glyph = commands
        .spawn((
            Text::new(BIN_GLYPH.to_string()),
            crate::hud::icon_tf(fonts, metrics.small),
            TextColor(palette::MUTED),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(bin).add_child(glyph);
    bin
}

/// The warning mark with its hint, at the row's right end, or nothing when
/// this client and the gateway agree.
fn mark_cell(
    commands: &mut Commands,
    words: &RowWords,
    fonts: &UiFonts,
    metrics: Metrics,
    lang: Lang,
) -> Option<Entity> {
    let warning = words.warning?;
    let cell = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                right: px(0),
                top: px(0),
                bottom: px(0),
                width: px(mark_width(metrics)),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            super::hint::HoverHint(warning.explain(lang)),
        ))
        .id();
    let glyph = commands
        .spawn((
            Text::new(WARNING_GLYPH.to_string()),
            crate::hud::icon_tf(fonts, metrics.text),
            TextColor(words.ink),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(cell).add_child(glyph);
    Some(cell)
}

/// A saved gateway's name, or its address when it has none.
pub(super) fn title_of(state: &LobbyState, url: &str) -> String {
    row_words(url, state.probes.get(url), state.lobby.lang()).title
}

/// The line under the account form's title: the address when the title is
/// a name, and the short version, which explains itself when pointed at.
pub(super) fn chosen_line(
    commands: &mut Commands,
    state: &LobbyState,
    fonts: &UiFonts,
    metrics: Metrics,
) -> Entity {
    let lang = state.lobby.lang();
    let words = row_words(&state.gateway, state.probes.get(&state.gateway), lang);
    let line = commands
        .spawn((
            Node {
                flex_wrap: FlexWrap::Wrap,
                column_gap: px(metrics.gap),
                align_items: AlignItems::Center,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let dot = reach_dot(commands, &words);
    commands.entity(line).add_child(dot);
    if let Some(address) = &words.address {
        let address = commands
            .spawn((
                Text::new(address.clone()),
                tf(fonts, metrics.small),
                TextColor(palette::MUTED),
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(line).add_child(address);
    }
    let version = commands
        .spawn((
            Text::new(words.version.clone()),
            tf(fonts, metrics.small),
            TextColor(words.ink),
            super::hint::HoverHint(match words.warning {
                Some(warning) => format!("{}\n{}", words.version_in_full, warning.explain(lang)),
                None => words.version_in_full.clone(),
            }),
        ))
        .id();
    commands.entity(line).add_child(version);
    line
}

/// Font Awesome's triangle-exclamation.
pub(super) const WARNING_GLYPH: char = '\u{f071}';

/// Font Awesome's trash-can.
const BIN_GLYPH: char = '\u{f2ed}';

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn gateways_are_normalized_and_never_contain_credentials() {
        assert_eq!(
            normalize(" HTTPS://Example.com:443/base/ "),
            Some("https://example.com/base".into())
        );
        assert_eq!(
            normalize("http://127.0.0.1:28766"),
            Some("http://127.0.0.1:28766".into())
        );
        for bad in [
            "",
            "example.com",
            "file:///etc/passwd",
            "https://user:pass@host",
            "https://host/?token=secret",
            "https://host/#x",
        ] {
            assert_eq!(normalize(bad), None);
        }
    }
}
