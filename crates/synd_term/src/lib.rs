#![allow(clippy::new_without_default)]
#![warn(rustdoc::broken_intra_doc_links)]

pub mod application;
pub mod auth;
pub mod cli;
pub mod client;
pub(crate) mod command;
pub mod config;
pub(crate) mod event;
pub mod filesystem;
pub mod interact;
pub mod job;
pub mod keymap;
pub mod local_api;
pub mod matcher;
pub(crate) mod operation;
pub mod terminal;
pub mod types;
pub mod ui;

#[cfg(feature = "integration")]
pub mod integration;
