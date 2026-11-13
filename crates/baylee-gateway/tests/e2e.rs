//! The gateway's end-to-end tests, as one test program.
//!
//! Each file beside this one is a module of it. They were 68 programs, and
//! every one linked the gateway, the engine and every card: a changed card or
//! engine relinked all of them (about 118 MB each, and as much again for its
//! debug symbols). nextest, the gates' runner, still starts every test in a
//! process of its own, so a test that sets the environment or binds a port
//! is as alone as it was. Run one file's tests with
//! `cargo nextest run -p baylee-gateway --test e2e <file>::`.

#![allow(clippy::missing_docs_in_private_items)]

// Looks cards up, so this program holds the pool (`baylee-cards-pool`).
use baylee_cards_pool as _;

mod common;
mod stub_central;

mod e2e_account;
mod e2e_admin;
mod e2e_admission;
mod e2e_art;
mod e2e_banlists;
mod e2e_bounds;
mod e2e_card_blocks;
mod e2e_central;
mod e2e_central_barrier;
mod e2e_central_load;
mod e2e_chair_report;
mod e2e_chair_tickets;
mod e2e_clock;
mod e2e_command_line;
mod e2e_cors;
mod e2e_deck_list;
mod e2e_decks;
mod e2e_departure;
mod e2e_engine_lifecycle;
mod e2e_feedback;
mod e2e_guests;
mod e2e_health;
mod e2e_hosted_access;
mod e2e_hosted_fallback;
mod e2e_hosted_llm;
mod e2e_house_difficulty;
mod e2e_hvh;
mod e2e_images;
mod e2e_late_socket;
mod e2e_lobby_feed;
mod e2e_lobby_running;
mod e2e_lobby_stats;
mod e2e_local_agents;
mod e2e_lost_connection;
mod e2e_moderation;
mod e2e_participation;
mod e2e_play;
mod e2e_pool_facts;
mod e2e_processes;
mod e2e_protocol;
mod e2e_record_export;
mod e2e_registration;
mod e2e_rematch;
mod e2e_replays;
mod e2e_reports;
mod e2e_resume;
mod e2e_resume_discovery;
mod e2e_room_setup;
mod e2e_rooms;
mod e2e_seat_bridge;
mod e2e_seat_llm;
mod e2e_seat_rate;
mod e2e_session_doors;
mod e2e_settings;
mod e2e_social_depart;
mod e2e_social_friends;
mod e2e_social_groups;
mod e2e_social_invites;
mod e2e_social_keys;
mod e2e_social_messages;
mod e2e_social_presence;
mod e2e_social_reports;
mod e2e_social_table;
mod e2e_spectate;
mod e2e_terms;
mod e2e_text;
mod e2e_usernames;
mod e2e_ws_tickets;
mod load;
