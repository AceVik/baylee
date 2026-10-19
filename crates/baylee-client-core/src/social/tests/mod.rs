//! The social machine's tests (§16).

mod fakegw;
mod smoke;

// Flows through the fake gateway (one writer).
mod archive;
mod flows;
mod history_disk;

// Pure units (another writer).
mod commands;
mod compose;
mod contacts;
mod keys;
mod notify;
mod select;
mod tabs;
mod text;
