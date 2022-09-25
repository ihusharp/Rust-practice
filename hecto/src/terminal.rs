use std::io::{self, stdout, Write};

use termion::{
    event::Key,
    input::TermRead,
    raw::{IntoRawMode, RawTerminal},
};

use crate::editor::Position;

pub struct Terminal {
    size: Size,
}

pub struct Size {
    pub width: u16,
    pub height: u16,
    _stdout: RawTerminal<std::io::Stdout>,
}

impl Terminal {
    pub fn new() -> Result<Self, io::Error> {
        let size = termion::terminal_size()?;
        Ok(Self {
            size: Size {
                width: size.0,
                height: size.1,
                _stdout: stdout().into_raw_mode()?,
            },
        })
    }

    pub fn size(&self) -> &Size {
        &self.size
    }

    pub fn clear_screen() {
        print!("{}", termion::clear::All);
    }

    pub fn flush() -> Result<(), io::Error> {
        stdout().flush()
    }

    pub fn cursor_position(pos: &Position) {
        let x = pos.x.saturating_add(1) as u16;
        let y = pos.y.saturating_add(1) as u16;
        print!("{}", termion::cursor::Goto(x, y));
    }

    pub fn read_key() -> Result<Key, io::Error> {
        loop {
            if let Some(key) = io::stdin().lock().keys().next() {
                return key;
            }
        }
    }

    pub fn cursor_hide() {
        print!("{}", termion::cursor::Hide);
    }
    pub fn cursor_show() {
        print!("{}", termion::cursor::Show);
    }
    pub fn clear_current_line() {
        print!("{}", termion::clear::CurrentLine);
    }
}
