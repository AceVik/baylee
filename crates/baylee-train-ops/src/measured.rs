//! What the trained AI plays when it is measured: the thinking depth and the rule
//! groups over the tactical layer that the evaluations against `sharp` and the
//! ladders are taken at. `seat4` plays them by default, so the seat players
//! meet is the one the numbers are about, and `ladder4` passes them for its
//! `measured` mode (`docs/trained-ai.md` §"Rule groups over the tactical
//! layer"). A change here is a change of the champion's play: it comes with
//! its paired measurement, never alone.

/// Fixed inference depth `K`.
pub const THINKING_STEPS: u8 = 2;

/// The bundle of 6 November: every group but `gift` and `press`.
pub const RULE_GROUPS: &str = "race,trip,trade,wall,chump,keep,land,ping,curve,aim,void,side,mass,burn,trick,walker,spend,react";
