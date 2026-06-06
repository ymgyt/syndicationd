#![allow(clippy::new_without_default)]
#![warn(rustdoc::broken_intra_doc_links)]

pub mod application;
pub mod auth;
pub mod client;
pub(crate) mod command;
pub mod config;
pub(crate) mod event;
pub mod interact;
pub mod job;
pub mod keymap;
pub mod matcher;
pub(crate) mod operation;
pub mod terminal;
pub mod types;
pub mod ui;

#[cfg(feature = "integration")]
pub mod integration;

#[cfg(any(test, feature = "integration"))]
pub mod test_support;

#[macro_export]
macro_rules! key {
    (backspace) => {
        crossterm::event::Event::Key(crossterm::event::KeyEvent::new(
            crossterm::event::KeyCode::Backspace,
            crossterm::event::KeyModifiers::NONE,
        ))
    };
    (enter) => {
        crossterm::event::Event::Key(crossterm::event::KeyEvent::new(
            crossterm::event::KeyCode::Enter,
            crossterm::event::KeyModifiers::NONE,
        ))
    };
    (tab) => {
        crossterm::event::Event::Key(crossterm::event::KeyEvent::new(
            crossterm::event::KeyCode::Tab,
            crossterm::event::KeyModifiers::NONE,
        ))
    };
    (esc) => {
        crossterm::event::Event::Key(crossterm::event::KeyEvent::new(
            crossterm::event::KeyCode::Esc,
            crossterm::event::KeyModifiers::NONE,
        ))
    };
    ($key:literal) => {
        crossterm::event::Event::Key(crossterm::event::KeyEvent::new(
            crossterm::event::KeyCode::Char($key),
            crossterm::event::KeyModifiers::NONE,
        ))
    };
}

#[macro_export]
macro_rules! shift {
    ($key:literal) => {
        crossterm::event::Event::Key(crossterm::event::KeyEvent::new(
            crossterm::event::KeyCode::Char($key),
            crossterm::event::KeyModifiers::SHIFT,
        ))
    };
}
