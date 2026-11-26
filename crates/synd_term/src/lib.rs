#![allow(clippy::new_without_default)]
#![warn(rustdoc::broken_intra_doc_links)]

pub use tuiframework::crossterm;

pub mod application;
pub mod auth;
pub mod client;
pub(crate) mod command;
pub mod config;
pub(crate) mod event;
pub mod external_tools;
pub mod keymap;
pub mod matcher;
pub(crate) mod operation;
mod reader;
pub mod types;
pub mod ui;

#[cfg(feature = "integration")]
pub mod integration;

#[cfg(any(test, feature = "integration"))]
pub mod test_support;

#[macro_export]
macro_rules! key {
    (backspace) => {
        $crate::crossterm::event::Event::Key($crate::crossterm::event::KeyEvent::new(
            $crate::crossterm::event::KeyCode::Backspace,
            $crate::crossterm::event::KeyModifiers::NONE,
        ))
    };
    (enter) => {
        $crate::crossterm::event::Event::Key($crate::crossterm::event::KeyEvent::new(
            $crate::crossterm::event::KeyCode::Enter,
            $crate::crossterm::event::KeyModifiers::NONE,
        ))
    };
    (tab) => {
        $crate::crossterm::event::Event::Key($crate::crossterm::event::KeyEvent::new(
            $crate::crossterm::event::KeyCode::Tab,
            $crate::crossterm::event::KeyModifiers::NONE,
        ))
    };
    (esc) => {
        $crate::crossterm::event::Event::Key($crate::crossterm::event::KeyEvent::new(
            $crate::crossterm::event::KeyCode::Esc,
            $crate::crossterm::event::KeyModifiers::NONE,
        ))
    };
    ($key:literal) => {
        $crate::crossterm::event::Event::Key($crate::crossterm::event::KeyEvent::new(
            $crate::crossterm::event::KeyCode::Char($key),
            $crate::crossterm::event::KeyModifiers::NONE,
        ))
    };
}

#[macro_export]
macro_rules! shift {
    ($key:literal) => {
        $crate::crossterm::event::Event::Key($crate::crossterm::event::KeyEvent::new(
            $crate::crossterm::event::KeyCode::Char($key),
            $crate::crossterm::event::KeyModifiers::SHIFT,
        ))
    };
}
