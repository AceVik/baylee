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
        // one. The address typed or remembered is the player's and not the
        // gateway's, so it comes along: without it the one launch-to-launch
        // convenience the sign-in form has was gone the moment a gateway was
        // chosen, which is now always before the form is seen.
        let email = self.lobby.field(Field::Email).to_string();
        self.lobby = Lobby::new();
        self.lobby.set_lang(lang);
        self.lobby.set_field(Field::Email, &email);
        client_core::images::reset_art_base();
        self.gateway = url;
        self.gateway_selected = true;
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
        self.lobby.set_gateway_ready(false);
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

/// Where a gateway row's words come from, and what colour its version is.
struct RowWords {
    /// The operator's name, or the address when there is none.
    title: String,
    /// The address, under a name; nothing when the title already is it.
    address: Option<String>,
    /// The version, or what stands in for one.
    version: String,
    ink: Color,
    warning: Option<Warning>,
}

fn row_words(url: &str, probe: Option<&Probe>, lang: Lang) -> RowWords {
    let warning =
        probe.and_then(|p| p.warning(baylee_protocol::PROTOCOL_VERSION, baylee_view::VIEW_VERSION));
    let ink = match warning {
        None if matches!(probe, Some(Probe::Known(_))) => palette::HEAL,
        None => palette::MUTED,
        Some(w) if w.refuses_games() => palette::DANGER,
        Some(_) => palette::ACTIVE,
    };
    let (name, version) = match probe {
        Some(Probe::Known(info)) => (info.name.clone(), info.version.clone()),
        Some(Probe::Older) => (None, Phrase::GatewayVersionUnknown.text(lang).into()),
        Some(Probe::Silent) => (None, Phrase::GatewayNotAnswering.text(lang).into()),
        Some(Probe::Asking) | None => (None, Phrase::GatewayCheckingShort.text(lang).into()),
    };
    match name {
        Some(name) => RowWords {
            title: name,
            address: Some(url.to_string()),
            version,
            ink,
            warning,
        },
        None => RowWords {
            title: url.to_string(),
            address: None,
            version,
            ink,
            warning,
        },
    }
}

/// How tall a gateway row is: two lines and the room around them, and a
/// finger's height on a phone.
fn row_height(metrics: Metrics) -> f32 {
    if metrics.frame == Frame::Phone {
        metrics.tap
    } else {
        56.0
    }
}

/// The saved gateways: four rows in sight and the rest a scroll away, so a
/// long list cannot grow the card it stands on.
pub(super) fn list(
    commands: &mut Commands,
    state: &LobbyState,
    fonts: &UiFonts,
    metrics: Metrics,
    scrolled_to: &Scrolled,
) -> Entity {
    let gap = metrics.gap * 0.5;
    let list = commands
        .spawn((
            Scrollable(List::Gateways),
            ScrollPosition(Vec2::new(0.0, scrolled_to.get(List::Gateways))),
            Node {
                width: percent(100),
                flex_shrink: 0.0,
                flex_direction: FlexDirection::Column,
                row_gap: px(gap),
                max_height: px(4.0 * row_height(metrics) + 3.0 * gap),
                overflow: Overflow::scroll_y(),
                ..default()
            },
        ))
        .id();
    for (index, url) in state.gateways.iter().enumerate() {
        let row = gateway_row(commands, state, fonts, metrics, index, url);
        commands.entity(list).add_child(row);
    }
    list
}

/// One saved gateway: a button to choose it, and at its right end a mark
/// when something is wrong with it, or else a chevron, since choosing it
/// goes somewhere.
///
/// The end cell lies over the button's right end as its sibling and not
/// inside it, so a finger held on a warning to read why does not also choose
/// the gateway. The chevron lets a press through to the button under it.
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
    let control = button(
        commands,
        fonts,
        metrics,
        &words.title,
        Press::SelectGateway(index),
        palette::PANEL,
        !state.lobby.busy() && state.adding.is_none(),
    );
    commands
        .entity(control)
        .entry::<Node>()
        .and_modify(move |mut node| {
            node.flex_direction = FlexDirection::Column;
            node.align_items = AlignItems::Start;
            node.justify_content = JustifyContent::Center;
            node.row_gap = px(2);
            node.flex_grow = 1.0;
            node.min_width = px(0);
            node.min_height = px(row_height(metrics));
            node.padding.right = px(metrics.tap);
        });
    let line = second_line(commands, &words, fonts, metrics);
    commands.entity(control).add_child(line);
    let row = row(commands, metrics, false);
    commands.entity(row).add_child(control);
    let end = end_cell(commands, &words, fonts, metrics, lang, true);
    commands.entity(row).add_children(&end);
    row
}

/// The gateway the account face is signing in to: the same words its row
/// has, on a surface that is not a button. Back is the way to another one.
pub(super) fn plaque(
    commands: &mut Commands,
    state: &LobbyState,
    fonts: &UiFonts,
    metrics: Metrics,
) -> Entity {
    let lang = state.lobby.lang();
    let words = row_words(&state.gateway, state.probes.get(&state.gateway), lang);
    let body = commands
        .spawn((
            Node {
                flex_grow: 1.0,
                min_width: px(0),
                min_height: px(row_height(metrics)),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Start,
                justify_content: JustifyContent::Center,
                row_gap: px(2),
                padding: UiRect {
                    left: px(10),
                    right: px(metrics.tap),
                    top: px(4),
                    bottom: px(4),
                },
                border: UiRect::all(px(1)),
                border_radius: BorderRadius::all(px(4)),
                ..default()
            },
            BackgroundColor(palette::PANEL_LIT),
            BorderColor::all(palette::DOCK_EDGE.with_alpha(0.45)),
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
    let line = second_line(commands, &words, fonts, metrics);
    commands.entity(body).add_children(&[title, line]);
    let row = row(commands, metrics, false);
    commands.entity(row).add_child(body);
    let end = end_cell(commands, &words, fonts, metrics, lang, false);
    commands.entity(row).add_children(&end);
    row
}

/// A row's second line: the address under a name, and the version in the
/// ink of what it means for this client.
fn second_line(
    commands: &mut Commands,
    words: &RowWords,
    fonts: &UiFonts,
    metrics: Metrics,
) -> Entity {
    let line = commands
        .spawn((
            Node {
                flex_wrap: FlexWrap::Wrap,
                column_gap: px(metrics.gap * 0.5),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
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
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(line).add_child(version);
    line
}

/// What stands at a row's right end: the warning mark with its hint, or,
/// where there is nothing to warn of and `chevron` asks for one, a chevron.
fn end_cell(
    commands: &mut Commands,
    words: &RowWords,
    fonts: &UiFonts,
    metrics: Metrics,
    lang: Lang,
    chevron: bool,
) -> Vec<Entity> {
    let (glyph, ink) = match words.warning {
        Some(_) => (WARNING_GLYPH, words.ink),
        None if chevron => (CHEVRON_GLYPH, palette::MUTED),
        None => return Vec::new(),
    };
    let cell = commands
        .spawn(Node {
            position_type: PositionType::Absolute,
            right: px(0),
            top: px(0),
            bottom: px(0),
            width: px(metrics.tap),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        })
        .id();
    match words.warning {
        Some(warning) => {
            commands
                .entity(cell)
                .insert(super::hint::HoverHint(warning.explain(lang)));
        }
        None => {
            commands.entity(cell).insert(Pickable::IGNORE);
        }
    }
    let glyph = commands
        .spawn((
            Text::new(glyph.to_string()),
            crate::hud::icon_tf(fonts, metrics.text),
            TextColor(ink),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(cell).add_child(glyph);
    vec![cell]
}

/// Font Awesome's triangle-exclamation.
const WARNING_GLYPH: char = '\u{f071}';

/// Font Awesome's chevron-right.
const CHEVRON_GLYPH: char = '\u{f054}';

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
