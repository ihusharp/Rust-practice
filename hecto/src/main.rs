#![warn(clippy::all, clippy::pedantic)]
mod editor;
mod terminal;
mod document;

use editor::Editor;
fn main() {
    let mut editor = Editor::new();
    editor.run();
}
