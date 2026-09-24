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
        self.lobby = Lobby::new();
        self.lobby.set_lang(lang);
        client_core::images::reset_art_base();
        self.gateway = url;
        self.gateway_selected = true;
        self.lobby.set_gateway_ready(true);
        self.lobby.set_registration_enabled(false);
        self.lobby.focus_on(Field::Email);
        true
    }

    /// Starts asking the typed address about itself, if it is an address.
    ///
    /// Answers the address to ask. Nothing is saved yet: that waits for
    /// [`Self::gateway_answered`], so an address nobody answers at is caught
    /// while the player is still looking at what they typed.
    pub(super) fn check_gateway(&mut self) -> Option<String> {
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

/// One saved gateway: a button to choose it, and a mark when something is
/// wrong with it.
///
/// The mark lies over the button's right end, as its sibling and not inside
/// it, so a finger held on it to read why does not also choose the gateway.
/// Every row is then the full width of the field under the list, whether it
/// carries a mark or not.
fn gateway_row(
    commands: &mut Commands,
    state: &LobbyState,
    fonts: &UiFonts,
    metrics: Metrics,
    index: usize,
    url: &str,
) -> Entity {
    let lang = state.lobby.lang();
    let chosen = state.gateway_selected && *url == state.gateway;
    let words = row_words(url, state.probes.get(url), lang);
    let marked = words.warning.is_some();
    let control = button(
        commands,
        fonts,
        metrics,
        &format!("{}{}", if chosen { "✓  " } else { "" }, words.title),
        Press::SelectGateway(index),
        if chosen {
            palette::PANEL_HOT
        } else {
            palette::PANEL
        },
        !state.lobby.busy() && state.adding.is_none(),
    );
    commands
        .entity(control)
        .entry::<Node>()
        .and_modify(move |mut node| {
            node.flex_direction = FlexDirection::Column;
            node.align_items = AlignItems::Start;
            node.row_gap = px(2);
            node.flex_grow = 1.0;
            node.min_width = px(0);
            if marked {
                node.padding.right = px(metrics.tap);
            }
        });
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
    if let Some(address) = words.address {
        let address = commands
            .spawn((
                Text::new(address),
                tf(fonts, metrics.small),
                TextColor(palette::MUTED),
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(line).add_child(address);
    }
    let version = commands
        .spawn((
            Text::new(words.version),
            tf(fonts, metrics.small),
            TextColor(words.ink),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(line).add_child(version);
    commands.entity(control).add_child(line);

    let row = row(commands, metrics, false);
    commands.entity(row).add_child(control);
    if let Some(warning) = words.warning {
        let mark = commands
            .spawn((
                Node {
                    position_type: PositionType::Absolute,
                    right: px(0),
                    top: px(0),
                    bottom: px(0),
                    width: px(metrics.tap),
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
        commands.entity(mark).add_child(glyph);
        commands.entity(row).add_child(mark);
    }
    row
}

/// Font Awesome's triangle-exclamation.
const WARNING_GLYPH: char = '\u{f071}';

pub(super) fn panel(
    commands: &mut Commands,
    state: &LobbyState,
    fonts: &UiFonts,
    metrics: Metrics,
) -> Entity {
    let lang = state.lobby.lang();
    let panel = super::ui::surface(commands, metrics);
    commands.entity(panel).insert((
        super::dock::Dock(1),
        Node {
            width: percent(100),
            flex_direction: FlexDirection::Column,
            row_gap: px(metrics.gap),
            padding: UiRect::all(px(metrics.pad * 1.6)),
            min_width: px(0),
            ..default()
        },
    ));
    let eyebrow = note(commands, fonts, metrics, Phrase::GatewayStep.text(lang));
    let title = heading(commands, fonts, metrics, Phrase::ChooseGateway.text(lang));
    let hint = note(commands, fonts, metrics, Phrase::GatewayHint.text(lang));
    commands.entity(panel).add_children(&[eyebrow, title, hint]);
    for (index, url) in state.gateways.iter().enumerate() {
        let row = gateway_row(commands, state, fonts, metrics, index, url);
        commands.entity(panel).add_child(row);
    }
    let field = text_field(
        commands,
        fonts,
        metrics,
        Phrase::GatewayAddress.text(lang),
        &FieldLook {
            buffer: state.lobby.buffer(Field::Gateway),
            focused: state.lobby.focus() == Field::Gateway,
            mask: None,
            press: Press::Focus(Field::Gateway),
            lead: None,
            hint: Some("https://"),
            tail: None,
        },
    );
    let add = button(
        commands,
        fonts,
        metrics,
        Phrase::SaveGateway.text(lang),
        Press::AddGateway,
        palette::PANEL_LIT,
        !state.lobby.busy() && state.adding.is_none(),
    );
    commands.entity(panel).add_children(&[field, add]);
    let offline = button(
        commands,
        fonts,
        metrics,
        Phrase::PlayOffline.text(lang),
        Press::PlayOffline,
        palette::PANEL_LIT,
        true,
    );
    let offline_hint = note(commands, fonts, metrics, Phrase::OfflineBenefit.text(lang));
    let settings = button(
        commands,
        fonts,
        metrics,
        Phrase::Settings.text(lang),
        Press::OpenSettings,
        palette::PANEL,
        true,
    );
    commands
        .entity(panel)
        .add_children(&[offline, offline_hint, settings]);
    panel
}

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
