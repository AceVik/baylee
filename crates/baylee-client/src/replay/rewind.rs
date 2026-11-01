//! [`Duel::rewind`]: what a [`crate::host::HostMessage::Jump`] does to the
//! duel (§7.1, "`HostMessage::Jump` in `Duel`").
//!
//! The table is about to be told another moment of the game from its first
//! log line, so everything the duel derived from the frames it was told is
//! forgotten, and everything that is the player's (where the camera stands,
//! which panels are open, how the hand is ordered) stays.
//!
//! Every field is named below, on purpose without `..Default::default()`: a
//! field added to `Duel` does not compile here until somebody says whether a
//! jump keeps it or forgets it.

use std::mem::take;

use crate::Duel;

impl Duel {
    /// Forgets what the frames told, keeps what the player chose.
    #[allow(clippy::too_many_lines)]
    // one line per field of `Duel`, by design
    // `Default::default()` is what forgetting a field means here, whatever
    // its type; naming each type would say nothing more.
    #[allow(clippy::default_trait_access)]
    pub(crate) fn rewind(&mut self) {
        let old = take(self);
        *self = Duel {
            // ---- forgotten: the question and every answer being built.
            // A replay asks nothing; a jump from the end takes the end's
            // `GameOver` down with the interaction that held it.
            yes_batch: Default::default(),
            combat_auto: None,
            keep_targets: None,
            target_page: 0,
            target_filter: None,
            interaction: None,
            mana_run: None,
            pay_staged: Default::default(),
            pay_window: None,
            pay_reason: None,
            reachable: Default::default(),
            suspend_reach: Default::default(),
            foretell_reach: Default::default(),
            ability_reach: Default::default(),
            activatable: Default::default(),
            proposed: Default::default(),
            ability_menu: None,
            granted_menu: Default::default(),
            cast_menu: None,
            cast_answer: None,
            ability_pick: 0,
            ability_page: 0,
            ability_tap: None,
            armed: None,
            owed_plan: None,
            priority_legal: None,
            outbox: Vec::new(),
            subtype_filter: String::new(),
            subtype_group: 0,
            decision_fold: Default::default(),
            stack_selected: None,
            stack_stop_requested: false,
            autopilot: None,
            // ---- forgotten: what was read off the difference between two
            // views. Across a jump there is no difference to read: a life
            // total that moved by five turns is no flash, no cue.
            view: None,
            table_mana: [0.0; 5],
            board: None,
            life_flash: Default::default(),
            tally: Default::default(),
            cues: Default::default(),
            strikes: Vec::new(),
            clock: Default::default(),
            seat_clocks: Default::default(),
            lost_seats: Default::default(),
            // ---- forgotten: the log, told again from line 0, and the
            // reveals held up off it (a telling from line 0 reveals nothing).
            log: Default::default(),
            reveals: Default::default(),
            reveal_fold: Default::default(),
            hovered_log: None,
            // ---- forgotten: the end. A jump back takes the end sheet down.
            closed: None,
            last_error: None,
            ai_said: Vec::new(),
            // ---- forgotten: a hover names an object the next board may not
            // hold; the next pointer move finds it again.
            hovered: None,
            hovered_pile: None,
            hovered_at: None,
            // ---- forgotten: a tear is a passage between two seatings, and
            // a jump places the table where it lands.
            tear: None,
            // ---- kept: the table itself, the player's arrangement and
            // camera, laid out again from the next view.
            table_pattern: old.table_pattern,
            hand_order: old.hand_order,
            hand_groups: old.hand_groups,
            statics: old.statics,
            layout: old.layout,
            visiting: old.visiting,
            arrangement: old.arrangement,
            arrangement_game: old.arrangement_game,
            arrangement_menu: old.arrangement_menu,
            arrangement_remember: old.arrangement_remember,
            follow: old.follow,
            follow_pending: old.follow_pending,
            follow_since: old.follow_since,
            follow_now: old.follow_now,
            follow_settling: old.follow_settling,
            arrangement_menu_asked: old.arrangement_menu_asked,
            arrangement_latch: old.arrangement_latch,
            interest_laid: old.interest_laid,
            canvas_aspect: old.canvas_aspect,
            canvas_frame: old.canvas_frame,
            attack_list_folded: old.attack_list_folded,
            // ---- kept: panels and their places, the player's.
            hand_drawer: old.hand_drawer,
            hand_shown: old.hand_shown,
            hand_drawn_open: old.hand_drawn_open,
            hand_scroll: old.hand_scroll,
            rows: old.rows,
            resize_drag: old.resize_drag,
            tray_drag: old.tray_drag,
            tray_restore: old.tray_restore,
            browser: old.browser,
            game_menu: old.game_menu,
            log_open: old.log_open,
            // The chat's door is a press this frame; the panel's being the
            // chat window is the session's, which a jump does not change.
            chat_door: None,
            chat_panel: old.chat_panel,
            ai_log_open: old.ai_log_open,
            ai_log_heard: old.ai_log_heard,
            report_asked: old.report_asked,
            settings_asked: old.settings_asked,
            concede_armed: old.concede_armed,
            leave_armed: old.leave_armed,
            departing: old.departing,
            leave_asked: old.leave_asked,
            // ---- kept: facts of this table, not of a moment in it.
            ability_orders_applied: old.ability_orders_applied,
            known_cards: old.known_cards,
            curtain_up: old.curtain_up,
            ready_sent: old.ready_sent,
            own_deck: old.own_deck,
            watching: old.watching,
            spectators: old.spectators,
            link_note: old.link_note,
            replay: old.replay,
        };
    }
}
