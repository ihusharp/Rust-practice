use std::io::{self, stdout};
use termion::{event::Key, raw::IntoRawMode};

use crate::terminal::Terminal;

const VERSION: &str = env!("CARGO_PKG_VERSION");

pub struct Position {
    pub x: usize,
    pub y: usize,
}

pub struct Editor {
    should_exit: bool,
    terminal: Terminal,
    cursor_position: Position,
}

impl Editor {
    pub fn run(&mut self) {
        let _stdout = stdout().into_raw_mode().unwrap();

        loop {
            if let Err(err) = self.refresh_screen() {
                die(&err);
            }
            if self.should_exit {
                break;
            }
            if let Err(err) = self.process_keypress() {
                die(&err);
            }
        }
    }

    pub fn new() -> Self {
        Self {
            should_exit: false,
            terminal: Terminal::new().expect("Failed to initialize terminal"),
            cursor_position: Position { x: 0, y: 0 },
        }
    }

    fn draw_welcome_msg(&self) {
        let mut welcome_message = format!("Hecto editor -- version {}", VERSION);
        let width = self.terminal.size().width as usize;
        let len = welcome_message.len();
        let padding = width.saturating_sub(len) / 2;
        let spaces = " ".repeat(padding.saturating_sub(1));
        welcome_message = format!("~{}{}", spaces, welcome_message);
        welcome_message.truncate(width);
        println!("{}\r", welcome_message);
    }

    fn draw_rows(&self) {
        let height = self.terminal.size().height;
        for row in 0..height - 1 {
            Terminal::clear_current_line();
            if row == height / 3 {            
                self.draw_welcome_msg();
            } else {
                println!("~\r");
            }
        }
    }

    fn process_keypress(&mut self) -> Result<(), io::Error> {
        let pressed_key = Terminal::read_key()?;
        match pressed_key {
            Key::Ctrl('c') => self.should_exit = true,
            Key::Up 
            | Key::Down 
            | Key::Left 
            | Key::Right 
            | Key::Home
            | Key::End
            | Key::PageUp
            | Key::PageDown => self.move_cursor(pressed_key),
            _ => (),
        }
        Ok(())
    }

    fn move_cursor(&mut self, key: Key) {
        let Position { mut y, mut x } = self.cursor_position;
        let height = self.terminal.size().height as usize;
        let width = self.terminal.size().width as usize;
        match key {
            Key::Up => y = y.saturating_sub(1),
            Key::Down => {
                if y < height {
                    y = y.saturating_add(1)
                }
            },
            Key::Left => x = x.saturating_sub(1),
            Key::Right => {
                if x < width {
                    x = x.saturating_add(1)
                }
            },
            Key::Home => x = 0,
            Key::End => x = width,
            Key::PageUp => y = 0,
            Key::PageDown => y = height,
            _ => (),
        }
        self.cursor_position = Position { x, y };
    }

    fn refresh_screen(&self) -> Result<(), io::Error> {
        Terminal::cursor_hide();
        Terminal::clear_screen();
        Terminal::cursor_position(&Position { x: 0, y: 0 });
        if self.should_exit {
            Terminal::clear_current_line();
            print!("Goodbye.");
        } else {
            self.draw_rows();
            Terminal::cursor_position(&self.cursor_position);
        }
        Terminal::cursor_show();
        Terminal::flush()
    }
}

fn die(e: &io::Error) {
    Terminal::clear_screen();
    panic!("{}", e)
}
