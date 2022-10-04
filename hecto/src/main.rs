#![warn(clippy::all, clippy::pedantic)]
mod editor;
mod terminal;
mod document;
mod row;

use editor::Editor;
fn main() {
    let mut editor = Editor::default();
    editor.run();
}
