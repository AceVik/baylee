//! Local gateway selection, independent of account-owned preferences.
#[allow(clippy::wildcard_imports)] // the lobby widget vocabulary
use super::*;

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

    pub(super) fn add_gateway(&mut self) -> bool {
        let Some(url) = normalize(self.lobby.field(Field::Gateway)) else {
            self.lobby.tell_refusal(Phrase::GatewayUrlInvalid, &[]);
            return false;
        };
        if !self.gateways.contains(&url) {
            self.gateways.push(url);
        }
        self.lobby.set_field(Field::Gateway, "");
        true
    }
}

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
        let chosen = state.gateway_selected && *url == state.gateway;
        let control = button(
            commands,
            fonts,
            metrics,
            &format!("{}{}", if chosen { "✓  " } else { "" }, url),
            Press::SelectGateway(index),
            if chosen {
                palette::PANEL_HOT
            } else {
                palette::PANEL
            },
            !state.lobby.busy(),
        );
        commands.entity(panel).add_child(control);
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
        !state.lobby.busy(),
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
