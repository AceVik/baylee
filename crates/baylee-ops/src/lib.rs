//! The server's operating commands: `baylee-deploy`, `baylee-invite` and
//! `baylee-reboot`, each a thin binary over a module here. They were shell
//! scripts; they keep the scripts' behaviour command for command, because
//! that behaviour (what waits for which game, what runs under sudo, what
//! never reaches a command line) is what their tests pin.
pub mod deploy;
pub mod gateway;
pub mod invite;
pub mod reboot;
pub mod settings;
pub mod sh;
