//! Local gateway selection, independent of account-owned preferences.
#[allow(clippy::wildcard_imports)] // the lobby widget vocabulary
use super::*;
use baylee_client_core::lobby::gateway_info::Warning;
use baylee_client_core::lobby::gateway_list;

/// Accept only an HTTP(S) origin/base path, with no embedded credentials.
/// The live gateway's former address is today's (`gateway_list::moved`).
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
    Some(gateway_list::moved(url.as_str().trim_end_matches('/')).to_string())
}

impl LobbyState {
    pub(super) fn select_gateway(&mut self, index: usize) -> bool {
        if self.central_pending() {
            return false;
        }
        let Some(url) = self.displayed_gateways().get(index).cloned() else {
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
        let gateway_field = self.lobby.field(Field::Gateway).to_string();
        let settings_open = self.lobby.settings_open();
        let central_me = self
            .central_ready()
            .then(|| self.lobby.me().filter(|me| !me.guest).cloned())
            .flatten();
        let central_decks = self.central_ready().then(|| self.lobby.decks().to_vec());
        let central_house = self
            .central_ready()
            .then(|| self.lobby.library().house.clone());
        self.lobby = Lobby::new();
        self.lobby.set_lang(lang);
        self.lobby.set_settings_open(settings_open);
        self.lobby.set_field(Field::Username, &username);
        self.lobby.set_field(Field::Gateway, &gateway_field);
        if let Some(me) = central_me {
            self.lobby.set_me(me);
        }
        if let Some(decks) = central_decks
            && !decks.is_empty()
        {
            let _ = self.lobby.apply(LobbyEvent::Decks(decks));
        }
        if let Some(house) = central_house
            && !house.is_empty()
        {
            let _ = self.lobby.apply(LobbyEvent::Library(
                baylee_client_core::lobby::library::Reply::House(house),
            ));
        }
        self.art_cache = false;
        self.gateway = url;
        self.gateway_selected = true;
        if self.central_ready() {
            self.hub = Hub::Gateways;
        }
        self.left_central = LeftCentral::No;
        self.gateway_cursor = None;
        self.front_menu = false;
        self.lobby
            .keep_guest(self.guests.get(&self.gateway).cloned());
        self.lobby.set_gateway_ready(true);
        self.lobby.set_registration(Registration::Off);
        true
    }

    /// What the chosen gateway's `GET /auth/config` said about who it lets
    /// in. One that takes Baylee Central sign-ins turns a native client's
    /// account form into Central's (`central::bound`); every other gateway
    /// keeps its own, with the sign-up and guest doors it offers.
    pub(super) fn auth_config(
        &mut self,
        registration: Registration,
        art_cache: bool,
        guests: bool,
        central: bool,
    ) {
        if self.central_pending() && !self.central_bound() {
            return;
        }
        self.lobby.set_registration(registration);
        self.lobby.set_guests_enabled(guests);
        self.art_cache = art_cache;
        #[cfg(not(target_arch = "wasm32"))]
        {
            if central {
                super::central::bound(self);
            }
            if let Some(account) = &mut self.central {
                account.no_central_here = !central;
            }
        }
        #[cfg(target_arch = "wasm32")]
        let _ = central;
    }

    /// Back from the create-account or guest face (`‹ Back`, Esc); `false`
    /// when none is up. Signed in at Central the guest's face came off the
    /// account's list, so that is what it goes back to.
    pub(super) fn back_a_face(&mut self) -> bool {
        let back = self.lobby.back_to_sign_in();
        #[cfg(not(target_arch = "wasm32"))]
        if back && self.central_ready() {
            super::central::back_to_list(self);
        }
        back
    }

    /// Back from the account form to the gateway form.
    ///
    /// The epoch moves as it does on a new choice, so nothing the gateway
    /// left behind still answers into the form. Choosing the same gateway
    /// again is one tap, and the address typed into the form survives it.
    /// From Central's sign-in, which a gateway asked for, it is the way back
    /// to the device's own gateways (`central::leave`).
    pub(super) fn leave_gateway(&mut self) {
        if self.central_pending() {
            #[cfg(not(target_arch = "wasm32"))]
            super::central::leave(self);
            return;
        }
        self.gateway_epoch = self.gateway_epoch.wrapping_add(1);
        self.gateway_selected = false;
        self.left_central = LeftCentral::No;
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
    /// The pinned gateway stays (`gateway_list::forget` refuses it).
    pub(super) fn forget_gateway(&mut self, url: &str) {
        if !gateway_list::forget(&mut self.gateways, url) {
            return;
        }
        self.probes.remove(url);
        self.uses.forget(url);
        self.gateway_cursor = None;
    }

    /// The indices in [`Self::displayed_gateways`] currently visible on screen,
    /// in the grouped and sorted order the list draws them.
    #[must_use]
    pub(crate) fn visible_gateway_indices(&self) -> Vec<usize> {
        let gateways = self.displayed_gateways();
        let query = self.lobby.field(Field::GatewaySearch);
        let mut visible = Vec::new();
        for group in gateway_list::GatewayGroup::ALL {
            if !self.gateway_filter.matches(group) {
                continue;
            }
            if self.gateway_filter == gateway_list::GatewayFilter::All
                && self.gateway_collapsed[group.index()]
            {
                continue;
            }
            let mut indices: Vec<usize> = gateways
                .iter()
                .enumerate()
                .filter(|(_, url)| {
                    let probe = self.probes.get(*url);
                    group_of(probe) == group && gateway_list::matches_query(url, probe, query)
                })
                .map(|(index, _)| index)
                .collect();
            gateway_list::sort_indices(
                &mut indices,
                gateways,
                |u| self.probes.get(u),
                self.gateway_sort,
            );
            visible.extend(indices);
        }
        visible
    }

    /// Moves the arrow keys' row one down, or up, across the currently
    /// visible (unfolded and matching) gateways. From none, down lands on
    /// the first visible row and up on the last; at either end it stays.
    pub(super) fn move_gateway_cursor(&mut self, down: bool) {
        let visible = self.visible_gateway_indices();
        let Some(last) = visible.len().checked_sub(1) else {
            self.gateway_cursor = None;
            return;
        };
        let pos = self
            .gateway_cursor
            .and_then(|cursor| visible.iter().position(|&idx| idx == cursor));
        let next = match (pos, down) {
            (None, true) => 0,
            (None, false) => last,
            (Some(at), true) => (at + 1).min(last),
            (Some(at), false) => at.saturating_sub(1),
        };
        self.gateway_cursor = Some(visible[next]);
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
        self.ask_gateway(url)
    }

    /// Starts asking an address about itself, typed or proposed by a link
    /// the player said yes to (`links`): the same question, the same
    /// answer, saved by [`Self::gateway_answered`] only when a gateway
    /// answered. Nothing while another answer is owed.
    pub(super) fn ask_gateway(&mut self, url: String) -> Option<String> {
        if self.adding.is_some() {
            return None;
        }
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
        // Emptied when it held this address; a link's gateway leaves what
        // the player was typing alone.
        if normalize(self.lobby.field(Field::Gateway)).as_ref() == Some(&url) {
            self.lobby.set_field(Field::Gateway, "");
        }
        if self.gateways.contains(&url) {
            return false;
        }
        self.gateways.push(url);
        true
    }

    /// The saved addresses nobody has asked yet, marked as being asked.
    pub(super) fn unasked_gateways(&mut self) -> Vec<String> {
        if self.central_pending() {
            return Vec::new();
        }
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
pub(super) struct RowWords {
    /// The operator's name, or the address when there is none.
    pub(super) title: String,
    /// The address, under a name; nothing when the title already is it.
    pub(super) address: Option<String>,
    /// Short status note when the gateway is offline or being checked.
    pub(super) detail: String,
    /// Protocol and view version line shown inside the version tag (`P3 · V40`).
    pub(super) protocol_tag: Option<String>,
    /// Live `(players_online, rooms_waiting, games_running)` counts shown behind the URL.
    pub(super) stats: Option<(u32, u32, u32)>,
    /// The version as a row has room for, `v0.1.1-beta.3b`, or what stands in for
    /// one.
    pub(super) version: String,
    /// What the version says in full, for the hint over it.
    pub(super) version_in_full: String,
    /// The version's ink: what it means for this client.
    pub(super) ink: Color,
    /// Whether the gateway answered, as the dot's colour.
    pub(super) reach: Color,
    /// What the dot says, pointed at.
    pub(super) reach_said: String,
    /// What is wrong with it for this client, which is the warning mark.
    /// Only a version can be: a gateway that is not answering says so with
    /// its dot, and a mark as well would be one fact said twice.
    pub(super) warning: Option<Warning>,
}

pub(super) fn row_words(url: &str, probe: Option<&Probe>, lang: Lang) -> RowWords {
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
    let (name, version, version_in_full, detail, protocol_tag, stats) = match probe {
        Some(Probe::Known(info)) => {
            let proto = format!("P{} · V{}", info.protocol_version, info.view_version);
            let live_stats = match (info.games_running, info.rooms_waiting) {
                (Some(running), Some(waiting)) => {
                    Some((info.players_online.unwrap_or(0), waiting, running))
                }
                _ => None,
            };
            (
                info.name.clone(),
                info.short_version(),
                info.version.clone(),
                String::new(),
                Some(proto),
                live_stats,
            )
        }
        Some(Probe::Older) => (
            None,
            "v?".to_string(),
            Phrase::GatewayVersionUnknown.text(lang).to_string(),
            Phrase::GatewayVersionUnknown.text(lang).to_string(),
            None,
            None,
        ),
        Some(Probe::Silent) => (
            None,
            "—".to_string(),
            Phrase::GatewayNotAnswering.text(lang).to_string(),
            Phrase::GatewayNotAnswering.text(lang).to_string(),
            None,
            None,
        ),
        Some(Probe::Asking) | None => (
            None,
            "…".to_string(),
            Phrase::GatewayCheckingShort.text(lang).to_string(),
            Phrase::GatewayCheckingShort.text(lang).to_string(),
            None,
            None,
        ),
    };
    let name = name
        .or_else(|| gateway_list::is_pinned(url).then(|| gateway_list::PINNED_NAME.to_string()));
    let (title, address) = match name {
        Some(name) => (name, Some(url.to_string())),
        None => (url.to_string(), None),
    };
    RowWords {
        title,
        address,
        detail,
        protocol_tag,
        stats,
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
    if metrics.frame == Frame::Compact {
        metrics.tap + 12.0
    } else {
        44.0
    }
}

/// Height of a collapsible compatibility group header.
const GROUP_HEADER_HEIGHT: f32 = 28.0;

/// Extra vertical breathing room before a non-first compatibility section.
const GROUP_SECTION_GAP: f32 = 8.0;

/// The room between two rows.
const ROW_GAP: f32 = 4.0;

/// How many rows are in sight before the list scrolls.
pub(super) const ROWS_IN_SIGHT: usize = 4;

/// Extra items mounted above and below the visible viewport in the lazy
/// gateway list so wheel/thumb scrolling never exposes an unmounted gap.
const OVERSCAN_ITEMS: usize = 6;

/// The side of a row's bin.
fn bin_side(metrics: Metrics) -> f32 {
    if metrics.frame == Frame::Compact {
        30.0
    } else {
        24.0
    }
}

/// One item in the virtualized gateway list.
#[derive(Clone, Debug, PartialEq, Eq)]
enum VirtualGatewayItem {
    GroupHeader {
        group: gateway_list::GatewayGroup,
        count: usize,
        collapsed: bool,
        first: bool,
    },
    Row {
        index: usize,
        url: String,
    },
}

/// Lazy/virtualized mounting state for [`List::Gateways`]: only the slice of
/// items intersecting the scroll viewport (plus [`OVERSCAN_ITEMS`]) is
/// mounted in `window_box`, while `top_spacer` and `bottom_spacer` preserve
/// the exact total scroll height for the native scrollbar.
#[derive(Component)]
pub(super) struct GatewayVirtualList {
    items: Vec<VirtualGatewayItem>,
    tops: Vec<f32>,
    heights: Vec<f32>,
    viewport_height: f32,
    metrics: Metrics,
    window_start: usize,
    window_end: usize,
    top_spacer: Entity,
    window_box: Entity,
    bottom_spacer: Entity,
}

fn group_of(probe: Option<&Probe>) -> gateway_list::GatewayGroup {
    gateway_list::GatewayGroup::of(
        probe,
        baylee_protocol::PROTOCOL_VERSION,
        baylee_view::VIEW_VERSION,
    )
}

fn group_phrase(group: gateway_list::GatewayGroup) -> Phrase {
    match group {
        gateway_list::GatewayGroup::Compatible => Phrase::GatewayGroupCompatible,
        gateway_list::GatewayGroup::Incompatible => Phrase::GatewayGroupIncompatible,
        gateway_list::GatewayGroup::Offline => Phrase::GatewayGroupOffline,
    }
}

fn group_stop_id(group: gateway_list::GatewayGroup) -> &'static str {
    match group {
        gateway_list::GatewayGroup::Compatible => "group-compatible",
        gateway_list::GatewayGroup::Incompatible => "group-incompatible",
        gateway_list::GatewayGroup::Offline => "group-offline",
    }
}

fn group_dot_color(group: gateway_list::GatewayGroup) -> Color {
    match group {
        gateway_list::GatewayGroup::Compatible => palette::HEAL,
        gateway_list::GatewayGroup::Incompatible => palette::ACTIVE,
        gateway_list::GatewayGroup::Offline => palette::DANGER,
    }
}

fn sort_phrase(sort: gateway_list::GatewaySort) -> Phrase {
    match sort {
        gateway_list::GatewaySort::LastUsed => Phrase::GatewaySortLastUsed,
        gateway_list::GatewaySort::Name => Phrase::GatewaySortName,
        gateway_list::GatewaySort::Address => Phrase::GatewaySortAddress,
        gateway_list::GatewaySort::Version => Phrase::GatewaySortVersion,
    }
}

fn sort_glyph(sort: gateway_list::GatewaySort) -> char {
    match sort {
        gateway_list::GatewaySort::LastUsed => '\u{f017}',
        gateway_list::GatewaySort::Name => '\u{f15d}',
        gateway_list::GatewaySort::Address => '\u{f1eb}',
        gateway_list::GatewaySort::Version => '\u{f162}',
    }
}

fn toolbar_icon_btn(
    commands: &mut Commands,
    kit: crate::shellkit::controls::Kit,
    glyph: char,
    enabled: bool,
    hint: String,
    action: impl Bundle,
) -> Entity {
    use crate::shellkit::metrics::px_fixed;
    use crate::shellkit::tokens;

    let side = kit.m.control;
    let rest_bg = tokens::CONTROL.with_alpha(0.82);
    let hot_bg = crate::ambience::lighter(rest_bg, 0.18);
    let btn = commands
        .spawn((
            Node {
                width: px_fixed(side),
                height: px_fixed(side),
                flex_shrink: 0.0,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                border: UiRect::all(px_fixed(1.0)),
                border_radius: BorderRadius::all(px_fixed(tokens::RADIUS_CONTROL)),
                ..default()
            },
            BackgroundColor(rest_bg),
            BorderColor::all(tokens::BORDER),
            super::hint::HoverHint(hint),
        ))
        .id();
    if enabled {
        commands.entity(btn).insert((
            Pickable::default(),
            bevy::picking::hover::PickingInteraction::default(),
            crate::ambience::Feel::tinting_to(rest_bg, hot_bg),
            crate::ambience::FeelBorder {
                rest: tokens::BORDER,
                hot: tokens::PRIMARY_EDGE,
                press: tokens::ACCENT,
            },
            action,
        ));
    } else {
        commands.entity(btn).insert(Pickable::IGNORE);
    }
    let icon = commands
        .spawn((
            Text::new(glyph.to_string()),
            crate::hud::icon_tf(kit.fonts, kit.m.text),
            TextColor(if enabled {
                tokens::INK
            } else {
                tokens::DISABLED
            }),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(btn).add_child(icon);
    btn
}

/// Search box, compact icon-only sort selector, and refresh button above the
/// gateway list.
pub(super) fn toolbar(
    commands: &mut Commands,
    state: &LobbyState,
    kit: crate::shellkit::controls::Kit,
) -> Entity {
    use super::field::{FieldStops, text_field_with};
    use super::front::keys;
    use crate::shellkit::metrics::px_fixed;

    let lang = state.lobby.lang();
    let metrics = super::front::faces::field_metrics(kit);
    let query = state.lobby.field(Field::GatewaySearch);

    let top_row = commands
        .spawn((
            Node {
                width: percent(100),
                align_items: AlignItems::Center,
                column_gap: kit.m.px(6.0),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();

    let look = FieldLook {
        buffer: state.lobby.buffer(Field::GatewaySearch),
        focused: state.lobby.caret_in(Field::GatewaySearch),
        mask: None,
        press: Press::Shared(SharedPress::Focus(Field::GatewaySearch)),
        lead: Some('\u{f002}'),
        hint: Some(Phrase::GatewaySearchPlaceholder.text(lang)),
        tail: None,
    };
    let stops = keys::field_stop(state, Field::GatewaySearch).map(|field| FieldStops {
        field,
        typed: Some(Field::GatewaySearch),
        eye: None,
    });
    let search_box = text_field_with(commands, kit.fonts, metrics, "", &look, stops);
    commands
        .entity(search_box)
        .entry::<Node>()
        .and_modify(|mut node| {
            node.flex_grow = 1.0;
            node.flex_basis = px_fixed(0.0);
            node.min_width = px_fixed(120.0);
        });
    commands.entity(top_row).add_child(search_box);

    if !query.is_empty() {
        let clear = toolbar_icon_btn(
            commands,
            kit,
            '\u{f00d}',
            true,
            Phrase::GatewayClearSearch.text(lang).to_string(),
            (
                Press::Front(FrontPress::ClearGatewaySearch),
                keys::stop(state, "gateway-clear"),
            ),
        );
        commands.entity(top_row).add_child(clear);
    }

    let sort_btn = toolbar_icon_btn(
        commands,
        kit,
        sort_glyph(state.gateway_sort),
        true,
        sort_phrase(state.gateway_sort).text(lang).to_string(),
        (
            Press::Front(FrontPress::CycleGatewaySort),
            keys::stop(state, "gateway-sort"),
        ),
    );
    let refresh_btn = toolbar_icon_btn(
        commands,
        kit,
        '\u{f2f1}',
        !state.lobby.busy() && state.adding.is_none(),
        Phrase::GatewayRefresh.text(lang).to_string(),
        (
            Press::Front(FrontPress::RefreshGateways),
            keys::stop(state, "gateway-refresh"),
        ),
    );
    commands
        .entity(top_row)
        .add_children(&[sort_btn, refresh_btn]);
    top_row
}

fn build_virtual_items(
    state: &LobbyState,
    metrics: Metrics,
) -> (Vec<VirtualGatewayItem>, Vec<f32>, Vec<f32>, usize) {
    let gateways = state.displayed_gateways();
    let query = state.lobby.field(Field::GatewaySearch);
    let row_h = row_height(metrics);
    let mut items = Vec::new();
    let mut tops = Vec::new();
    let mut heights = Vec::new();
    let mut y = 0.0f32;
    let mut visible_rows = 0usize;
    let mut has_any = false;

    let mut push_item = |item: VirtualGatewayItem, h: f32, extra_gap: f32| {
        if !items.is_empty() {
            y += ROW_GAP + extra_gap;
        }
        tops.push(y);
        heights.push(h);
        y += h;
        items.push(item);
    };

    for group in gateway_list::GatewayGroup::ALL {
        if !state.gateway_filter.matches(group) {
            continue;
        }
        let mut indices: Vec<usize> = gateways
            .iter()
            .enumerate()
            .filter(|(_, url)| {
                let probe = state.probes.get(*url);
                group_of(probe) == group && gateway_list::matches_query(url, probe, query)
            })
            .map(|(index, _)| index)
            .collect();
        if indices.is_empty() {
            continue;
        }
        gateway_list::sort_indices(
            &mut indices,
            gateways,
            |u| state.probes.get(u),
            state.gateway_sort,
        );
        let collapsed = state.gateway_filter == gateway_list::GatewayFilter::All
            && state.gateway_collapsed[group.index()];
        let first = !has_any;
        has_any = true;
        push_item(
            VirtualGatewayItem::GroupHeader {
                group,
                count: indices.len(),
                collapsed,
                first,
            },
            GROUP_HEADER_HEIGHT,
            if first { 0.0 } else { GROUP_SECTION_GAP },
        );
        if !collapsed {
            for index in indices {
                visible_rows += 1;
                push_item(
                    VirtualGatewayItem::Row {
                        index,
                        url: gateways[index].clone(),
                    },
                    row_h,
                    0.0,
                );
            }
        }
    }

    (items, tops, heights, visible_rows)
}

fn compute_window_bounds(
    tops: &[f32],
    heights: &[f32],
    offset: f32,
    viewport_height: f32,
) -> (usize, usize, f32, f32) {
    let n = tops.len();
    if n == 0 {
        return (0, 0, 0.0, 0.0);
    }
    let from = offset.max(0.0);
    let to = (from + viewport_height).max(from);
    let first_vis = tops
        .iter()
        .zip(heights)
        .position(|(t, h)| t + h > from)
        .unwrap_or(0);
    let last_vis = tops.iter().position(|t| *t >= to).unwrap_or(n);
    let start = first_vis.saturating_sub(OVERSCAN_ITEMS);
    let end = (last_vis + OVERSCAN_ITEMS).min(n).max(start);

    let total_height = tops.last().zip(heights.last()).map_or(0.0, |(t, h)| t + h);
    let top_pad = if start > 0 { tops[start] } else { 0.0 };
    let bottom_pad = if end < n {
        let last_mounted_bottom = if end > 0 {
            tops[end - 1] + heights[end - 1]
        } else {
            0.0
        };
        (total_height - last_mounted_bottom).max(0.0)
    } else {
        0.0
    };
    (start, end, top_pad, bottom_pad)
}

fn spawn_virtual_item(
    commands: &mut Commands,
    state: &LobbyState,
    fonts: &UiFonts,
    metrics: Metrics,
    item: &VirtualGatewayItem,
) -> Entity {
    match item {
        VirtualGatewayItem::GroupHeader {
            group,
            count,
            collapsed,
            first,
        } => group_header(
            commands, state, fonts, metrics, *group, *count, *collapsed, *first,
        ),
        VirtualGatewayItem::Row { index, url } => {
            gateway_row(commands, state, fonts, metrics, *index, url)
        }
    }
}

/// Updates the mounted slice of [`GatewayVirtualList`] when the list's
/// [`ScrollPosition`] moves outside the previously mounted window.
pub(super) fn sync_virtual_rows(
    mut commands: Commands,
    state: Res<LobbyState>,
    fonts: Option<Res<UiFonts>>,
    mut lists: Query<(&ScrollPosition, &mut GatewayVirtualList), Changed<ScrollPosition>>,
    mut nodes: Query<&mut Node>,
    children_q: Query<&Children>,
) {
    let Some(fonts) = fonts else {
        return;
    };
    for (scroll, mut vlist) in &mut lists {
        let (start, end, top_pad, bottom_pad) =
            compute_window_bounds(&vlist.tops, &vlist.heights, scroll.y, vlist.viewport_height);
        if start == vlist.window_start && end == vlist.window_end {
            continue;
        }
        vlist.window_start = start;
        vlist.window_end = end;
        if let Ok(mut node) = nodes.get_mut(vlist.top_spacer) {
            node.height = px(top_pad);
        }
        if let Ok(mut node) = nodes.get_mut(vlist.bottom_spacer) {
            node.height = px(bottom_pad);
        }
        if let Ok(existing) = children_q.get(vlist.window_box) {
            let old: Vec<Entity> = existing.iter().collect();
            for child in old {
                commands.entity(child).despawn();
            }
        }
        let mut mounted = Vec::with_capacity(end.saturating_sub(start));
        for item in &vlist.items[start..end] {
            mounted.push(spawn_virtual_item(
                &mut commands,
                &state,
                &fonts,
                vlist.metrics,
                item,
            ));
        }
        commands.entity(vlist.window_box).add_children(&mounted);
    }
}

/// The saved gateways, grouped by compatibility and online status.
///
/// Four rows are in sight. A fifth and more scroll lazily inside the same
/// height, with a native scrollbar beside them, so a long list cannot grow
/// the card it stands on; the wheel, a finger and the arrow keys all move
/// it, and the row the arrows are on is kept in sight.
pub(super) fn list(
    commands: &mut Commands,
    state: &LobbyState,
    fonts: &UiFonts,
    metrics: Metrics,
    scrolled_to: &Scrolled,
) -> Entity {
    let row = row_height(metrics);
    let in_sight = ROWS_IN_SIGHT as f32;
    let height = in_sight * row + (in_sight - 1.0) * ROW_GAP + GROUP_HEADER_HEIGHT + ROW_GAP;
    let (items, tops, heights, visible_rows) = build_virtual_items(state, metrics);
    let scrolls = visible_rows > ROWS_IN_SIGHT;
    let offset = in_view(
        scrolled_to.get(List::Gateways),
        state.gateway_cursor,
        &items,
        &tops,
        &heights,
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
                row_gap: px(0),
                overflow: Overflow::scroll_y(),
                ..default()
            },
        ))
        .id();

    if items.is_empty() {
        let empty = empty_matches(commands, state, fonts, metrics);
        commands.entity(list).add_child(empty);
        return list;
    }

    let (start, end, top_pad, bottom_pad) = compute_window_bounds(&tops, &heights, offset, height);
    let top_spacer = vspacer(commands, top_pad);
    let window_box = commands
        .spawn((
            Node {
                width: percent(100),
                flex_direction: FlexDirection::Column,
                row_gap: px(ROW_GAP),
                flex_shrink: 0.0,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let bottom_spacer = vspacer(commands, bottom_pad);

    let mut mounted = Vec::with_capacity(end.saturating_sub(start));
    for item in &items[start..end] {
        mounted.push(spawn_virtual_item(commands, state, fonts, metrics, item));
    }
    commands.entity(window_box).add_children(&mounted);
    commands
        .entity(list)
        .add_children(&[top_spacer, window_box, bottom_spacer])
        .insert(GatewayVirtualList {
            items,
            tops,
            heights,
            viewport_height: height,
            metrics,
            window_start: start,
            window_end: end,
            top_spacer,
            window_box,
            bottom_spacer,
        });

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
    let _ = super::scrollbars::attach(commands, frame, list, metrics);
    frame
}

fn vspacer(commands: &mut Commands, height: f32) -> Entity {
    commands
        .spawn((
            Node {
                width: percent(100),
                height: px(height),
                flex_shrink: 0.0,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id()
}

/// The list's scroll offset with the row at `cursor` in sight: unchanged
/// when it already is, else just far enough to show the whole row (and the
/// top group header when `cursor` reaches the very first visible row).
fn in_view(
    offset: f32,
    cursor: Option<usize>,
    items: &[VirtualGatewayItem],
    tops: &[f32],
    heights: &[f32],
    height: f32,
) -> f32 {
    let Some(target) = cursor else {
        return offset;
    };
    let first_row = items
        .iter()
        .position(|item| matches!(item, VirtualGatewayItem::Row { .. }));
    let Some(pos) = items
        .iter()
        .position(|item| matches!(item, VirtualGatewayItem::Row { index, .. } if *index == target))
    else {
        return offset;
    };
    let top = if Some(pos) == first_row {
        0.0
    } else {
        tops[pos]
    };
    let bottom = tops[pos] + heights[pos];
    if top < offset {
        top
    } else if bottom > offset + height {
        bottom - height
    } else {
        offset
    }
}

fn empty_matches(
    commands: &mut Commands,
    state: &LobbyState,
    fonts: &UiFonts,
    metrics: Metrics,
) -> Entity {
    let lang = state.lobby.lang();
    let no_gateways = state.displayed_gateways().is_empty();
    let box_node = commands
        .spawn((
            Node {
                width: percent(100),
                padding: UiRect::axes(px(18), px(18)),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                row_gap: px(6),
                border: UiRect::all(px(1)),
                border_radius: BorderRadius::all(px(10)),
                ..default()
            },
            BackgroundColor(palette::PANEL.with_alpha(0.65)),
            BorderColor::all(crate::shellkit::tokens::BORDER.with_alpha(0.75)),
            Pickable::IGNORE,
        ))
        .id();

    let badge = commands
        .spawn((
            Node {
                width: px(36),
                height: px(36),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                border_radius: BorderRadius::all(px(18)),
                border: UiRect::all(px(1)),
                margin: UiRect::bottom(px(2)),
                ..default()
            },
            BackgroundColor(palette::ACCENT.with_alpha(0.14)),
            BorderColor::all(palette::ACCENT.with_alpha(0.35)),
            Pickable::IGNORE,
        ))
        .id();
    let glyph = if no_gateways { '\u{f1eb}' } else { '\u{f002}' };
    let icon = commands
        .spawn((
            Text::new(glyph.to_string()),
            crate::hud::icon_tf(fonts, metrics.text),
            TextColor(palette::ACCENT),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(badge).add_child(icon);

    let (heading_str, sub_str) = if no_gateways {
        (
            Phrase::GatewayNoneSaved.text(lang),
            Phrase::GatewayNoneSavedHint.text(lang),
        )
    } else {
        (
            Phrase::GatewayNoMatches.text(lang),
            Phrase::GatewayClearSearch.text(lang),
        )
    };

    let heading = commands
        .spawn((
            Text::new(heading_str.to_string()),
            crate::hud::tf_bold(fonts, metrics.text),
            TextColor(palette::INK),
            TextLayout::justify(Justify::Center),
            Pickable::IGNORE,
        ))
        .id();
    let sub = commands
        .spawn((
            Text::new(sub_str.to_string()),
            tf(fonts, metrics.small),
            TextColor(palette::MUTED),
            TextLayout::justify(Justify::Center),
            Pickable::IGNORE,
        ))
        .id();
    commands
        .entity(box_node)
        .add_children(&[badge, heading, sub]);
    box_node
}

/// Collapsible section header for one [`gateway_list::GatewayGroup`].
#[allow(clippy::too_many_arguments)]
fn group_header(
    commands: &mut Commands,
    state: &LobbyState,
    fonts: &UiFonts,
    metrics: Metrics,
    group: gateway_list::GatewayGroup,
    count: usize,
    collapsed: bool,
    first: bool,
) -> Entity {
    let lang = state.lobby.lang();
    let chevron = if collapsed { "\u{f054}" } else { "\u{f078}" };
    let dot_color = group_dot_color(group);
    let header = commands
        .spawn((
            Node {
                width: percent(100),
                height: px(GROUP_HEADER_HEIGHT),
                flex_shrink: 0.0,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::SpaceBetween,
                padding: UiRect::axes(px(10), px(4)),
                margin: if first {
                    UiRect::ZERO
                } else {
                    UiRect::top(px(GROUP_SECTION_GAP))
                },
                border_radius: BorderRadius::all(px(6)),
                column_gap: px(8),
                ..default()
            },
            BackgroundColor(palette::PANEL.with_alpha(0.45)),
            Press::Front(FrontPress::ToggleGatewayGroup(group)),
            super::front::keys::stop(state, group_stop_id(group)),
            crate::ambience::Feel::tinting_to(palette::PANEL.with_alpha(0.45), palette::PANEL_HOT),
        ))
        .id();
    let left = commands
        .spawn((
            Node {
                align_items: AlignItems::Center,
                column_gap: px(8),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let arrow = commands
        .spawn((
            Text::new(chevron),
            crate::hud::icon_tf(fonts, metrics.small),
            TextColor(palette::MUTED),
            Pickable::IGNORE,
        ))
        .id();
    let status_dot = commands
        .spawn((
            Node {
                width: px(7),
                height: px(7),
                border_radius: BorderRadius::all(px(4)),
                ..default()
            },
            BackgroundColor(dot_color),
            Pickable::IGNORE,
        ))
        .id();
    let label = commands
        .spawn((
            Text::new(group_phrase(group).text(lang).to_string()),
            crate::hud::tf_bold(fonts, metrics.small),
            TextColor(palette::INK),
            Pickable::IGNORE,
        ))
        .id();
    commands
        .entity(left)
        .add_children(&[arrow, status_dot, label]);

    let badge = commands
        .spawn((
            Node {
                min_width: px(24),
                height: px(18),
                padding: UiRect::axes(px(7), px(1)),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                border: UiRect::all(px(1)),
                border_radius: BorderRadius::all(px(9)),
                ..default()
            },
            BackgroundColor(dot_color.with_alpha(0.14)),
            BorderColor::all(dot_color.with_alpha(0.35)),
            Pickable::IGNORE,
        ))
        .id();
    let count_text = commands
        .spawn((
            Text::new(count.to_string()),
            crate::hud::tf_bold(fonts, (metrics.small - 1.0).max(10.0)),
            TextColor(palette::INK),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(badge).add_child(count_text);
    commands.entity(header).add_children(&[left, badge]);
    header
}

/// One saved gateway:
/// ```text
///  ● [!] Name [Last used]                        [v0.1.1-beta.3b] [bin]
///        address · 2 Games · 1 Open              [   P3 · V40   ]
/// ```
/// A slimmer, structured two-line button showing the warning prefix, operator's
/// name, and recency badge on the top line, the address and live stats on the
/// subtitle line, and the two-line version/protocol pill + centered delete
/// button on the right.
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
    let busy = if state.central_ready() {
        state.central_admitting()
    } else {
        state.lobby.busy()
    };
    let enabled = !busy && state.adding.is_none();
    let marked = state.gateway_cursor == Some(index);
    let last_used = index == 0 && state.uses.of(url).count > 0;
    let right_pad = bin_side(metrics) + 16.0;

    let rest_bg = if marked {
        Color::srgba(0.16, 0.25, 0.35, 0.96)
    } else {
        Color::srgba(0.10, 0.15, 0.22, 0.92)
    };
    let hot_bg = Color::srgba(0.20, 0.31, 0.43, 0.98);
    let rest_border = if marked {
        palette::ACCENT
    } else {
        crate::shellkit::tokens::BORDER.with_alpha(0.85)
    };

    let control = button(
        commands,
        fonts,
        metrics,
        "",
        Press::Front(FrontPress::SelectGateway(index)),
        rest_bg,
        enabled,
    );
    commands
        .entity(control)
        .entry::<Node>()
        .and_modify(move |mut node| {
            node.flex_grow = 1.0;
            node.min_width = px(0);
            node.min_height = px(row_height(metrics));
            node.align_items = AlignItems::Center;
            node.column_gap = px(10);
            node.border = UiRect::all(px(1));
            node.border_radius = BorderRadius::all(px(8));
            node.padding = UiRect {
                left: px(12),
                right: px(right_pad),
                top: px(5),
                bottom: px(5),
            };
        });
    commands.entity(control).insert((
        BackgroundColor(rest_bg),
        BorderColor::all(rest_border),
        crate::ambience::Feel::tinting_to(rest_bg, hot_bg),
        crate::ambience::FeelBorder {
            rest: rest_border,
            hot: palette::ACCENT.with_alpha(0.95),
            press: palette::ACCENT,
        },
    ));

    let dot = reach_dot(commands, &words);
    commands.entity(dot).insert(passes_presses());
    let status = commands
        .spawn((
            Node {
                flex_shrink: 0.0,
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(status).add_child(dot);

    let texts = row_texts(commands, fonts, metrics, lang, &words, last_used);
    let side = version_pill(commands, fonts, metrics, &words);
    commands
        .entity(control)
        .add_children(&[status, texts, side]);

    let row = commands
        .spawn((
            Node {
                width: percent(100),
                flex_shrink: 0.0,
                align_items: AlignItems::Center,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let delete_btn = bin(commands, fonts, metrics, lang, index, enabled);
    commands.entity(row).add_children(&[control, delete_btn]);
    row
}

fn version_pill(
    commands: &mut Commands,
    fonts: &UiFonts,
    metrics: Metrics,
    words: &RowWords,
) -> Entity {
    let side = commands
        .spawn((
            Node {
                flex_shrink: 0.0,
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                row_gap: px(1),
                padding: UiRect::axes(px(8), px(3)),
                border: UiRect::all(px(1)),
                border_radius: BorderRadius::all(px(8)),
                ..default()
            },
            BackgroundColor(words.ink.with_alpha(0.12)),
            BorderColor::all(words.ink.with_alpha(0.35)),
            super::hint::HoverHint(words.version_in_full.clone()),
            passes_presses(),
        ))
        .id();
    let version = commands
        .spawn((
            Text::new(words.version.clone()),
            crate::hud::tf_bold(fonts, (metrics.small - 0.5).max(10.5)),
            TextColor(words.ink),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(side).add_child(version);
    if let Some(proto) = &words.protocol_tag {
        let proto_line = commands
            .spawn((
                Text::new(proto.clone()),
                tf(fonts, (metrics.small - 2.0).max(9.0)),
                TextColor(words.ink.with_alpha(0.72)),
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(side).add_child(proto_line);
    }
    side
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

fn row_badge(
    commands: &mut Commands,
    fonts: &UiFonts,
    metrics: Metrics,
    text: &str,
    ink: Color,
) -> Entity {
    let pill = commands
        .spawn((
            Node {
                padding: UiRect::axes(px(6), px(1)),
                border_radius: BorderRadius::all(px(8)),
                flex_shrink: 0.0,
                ..default()
            },
            BackgroundColor(ink.with_alpha(0.16)),
            Pickable::IGNORE,
        ))
        .id();
    let label = commands
        .spawn((
            Text::new(text.to_string()),
            tf(fonts, (metrics.small - 1.0).max(10.0)),
            TextColor(ink),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(pill).add_child(label);
    pill
}

/// A row's warning prefix, name, and recency badge over its address and live stats.
fn row_texts(
    commands: &mut Commands,
    fonts: &UiFonts,
    metrics: Metrics,
    lang: Lang,
    words: &RowWords,
    last_used: bool,
) -> Entity {
    let texts = commands
        .spawn((
            Node {
                flex_grow: 1.0,
                min_width: px(0),
                flex_direction: FlexDirection::Column,
                row_gap: px(1),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let top_line = commands
        .spawn((
            Node {
                align_items: AlignItems::Center,
                column_gap: px(6),
                flex_wrap: FlexWrap::Wrap,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    if let Some(mark) = mark_cell(commands, words, fonts, metrics, lang) {
        commands.entity(top_line).add_child(mark);
    }
    let title = commands
        .spawn((
            Text::new(words.title.clone()),
            crate::hud::tf_bold(fonts, metrics.text),
            TextColor(palette::INK),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(top_line).add_child(title);
    if last_used {
        let used = row_badge(
            commands,
            fonts,
            metrics,
            Phrase::GatewayLastUsedBadge.text(lang),
            palette::MUTED,
        );
        commands.entity(top_line).add_child(used);
    }
    commands.entity(texts).add_child(top_line);

    let sub_line = commands
        .spawn((
            Node {
                align_items: AlignItems::Center,
                column_gap: px(6),
                flex_wrap: FlexWrap::Wrap,
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
        commands.entity(sub_line).add_child(address);
    }
    if let Some((online, waiting, running)) = words.stats {
        let stats_text = format!(
            "· Online: {online} · {}: {waiting} · {}: {running}",
            Phrase::GatewayWaitingTables.text(lang),
            Phrase::GatewayRunningGames.text(lang)
        );
        let stats_label = commands
            .spawn((
                Text::new(stats_text),
                tf(fonts, (metrics.small - 0.5).max(10.0)),
                TextColor(palette::INK.with_alpha(0.75)),
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(sub_line).add_child(stats_label);
    } else if !words.detail.is_empty() {
        let detail = commands
            .spawn((
                Text::new(words.detail.clone()),
                tf(fonts, metrics.small),
                TextColor(words.ink.with_alpha(0.9)),
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(sub_line).add_child(detail);
    }
    commands.entity(texts).add_child(sub_line);
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

/// A row's bin, vertically centered on the right edge of the row: asks, then
/// removes the gateway from this device's list.
fn bin(
    commands: &mut Commands,
    fonts: &UiFonts,
    metrics: Metrics,
    lang: Lang,
    index: usize,
    enabled: bool,
) -> Entity {
    let side = bin_side(metrics);
    let holder = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                top: px(0),
                bottom: px(0),
                right: px(8),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let bin = commands
        .spawn((
            Node {
                width: px(side),
                height: px(side),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                border_radius: BorderRadius::all(px(5)),
                ..default()
            },
            BackgroundColor(Color::NONE),
            super::hint::HoverHint(Phrase::ForgetGateway.text(lang).to_string()),
        ))
        .id();
    if enabled {
        commands.entity(bin).insert((
            Press::Front(FrontPress::ForgetGateway(index)),
            crate::ambience::Feel::tinting_to(Color::NONE, palette::DANGER.with_alpha(0.22)),
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
    commands.entity(holder).add_child(bin);
    holder
}

/// The warning mark prefix before the gateway name, or nothing when
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
                min_width: px(16),
                min_height: px(16),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            super::hint::HoverHint(warning.explain(lang)),
            passes_presses(),
        ))
        .id();
    let glyph = commands
        .spawn((
            Text::new(WARNING_GLYPH.to_string()),
            crate::hud::icon_tf(fonts, metrics.small),
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

    /// The live gateway's former address, typed, is saved by today's.
    #[test]
    fn a_typed_former_address_is_the_live_gateway_s() {
        use baylee_client_core::lobby::gateway_list::{KEYED, PINNED};
        assert_eq!(normalize(&format!("{KEYED}/")), Some(PINNED.to_string()));
    }
}
