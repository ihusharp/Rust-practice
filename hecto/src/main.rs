#![warn(clippy::all, clippy::pedantic, clippy::restriction)]
#![allow(
    clippy::missing_docs_in_private_items,
    clippy::implicit_return,
    clippy::shadow_reuse,
    clippy::print_stdout,
    clippy::wildcard_enum_match_arm,
    clippy::else_if_without_else,
    clippy::integer_arithmetic,
)]
mod document;
mod editor;
mod row;
mod terminal;

use editor::Editor;
fn main() {
    let mut editor = Editor::default();
    editor.run();
}
