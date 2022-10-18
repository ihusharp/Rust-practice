use std::{
    fs,
    io::{Error, Write},
};

use crate::{editor::{Position, SearchDirection}, row::Row, filetype::FileType};

#[derive(Default)]
pub struct Document {
    pub rows: Vec<Row>,
    pub file_name: Option<String>,
    dirty: bool,
    file_type: FileType,
}

impl Document {
    pub fn open(filename: &str) -> Result<Self, std::io::Error> {
        let contents = fs::read_to_string(filename)?;
        let mut rows = Vec::new();
        let file_type = FileType::from(filename);
        for value in contents.lines() {
            let mut row = Row::from(value);
            row.highlight(file_type.highlight_options(), None);
            rows.push(row);
        }
        Ok(Self {
            rows,
            file_name: Some(filename.to_string()),
            dirty: false,
            file_type: file_type,
        })
    }

    pub fn save(&mut self) -> Result<(), Error> {
        if let Some(file_name) = &self.file_name {
            let mut file = fs::File::create(file_name)?;
            self.file_type = FileType::from(file_name);
            for row in &mut self.rows {
                file.write_all(row.as_bytes())?;
                file.write_all(b"\n")?;
                row.highlight(self.file_type.highlight_options(), None);
            }
            self.dirty = false;
        }
        Ok(())
    }

    pub fn row(&self, index: usize) -> Option<&Row> {
        self.rows.get(index)
    }

    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    pub fn len(&self) -> usize {
        self.rows.len()
    }

    pub fn insert_newline(&mut self, at: &Position) {
        // need to keep message status bar at the bottom
        if at.y == self.len() {
            self.rows.insert(at.y, Row::default());
        } else {
            let current_row = self.rows.get_mut(at.y).unwrap();
            let mut new_row = current_row.split(at.x);
            current_row.highlight(self.file_type.highlight_options(), None);
            new_row.highlight(self.file_type.highlight_options(), None);
            self.rows.insert(at.y + 1, new_row);
        }
    }

    pub fn insert(&mut self, at: &Position, c: char) {
        if at.y > self.len() {
            return;
        }
        self.dirty = true;
        if c == '\n' {
            self.insert_newline(at);
            return;
        }
        if at.y == self.len() {
            let mut row = Row::default();
            row.insert(0, c);
            row.highlight(self.file_type.highlight_options(), None);
            self.rows.push(row);
        } else {
            let row = self.rows.get_mut(at.y).unwrap();
            row.insert(at.x, c);
            row.highlight(self.file_type.highlight_options(), None);
        }
    }

    pub fn delete(&mut self, at: &Position) {
        if at.y >= self.len() {
            return;
        }
        self.dirty = true;
        // Check if we at the end of a line
        if at.x == self.rows.get_mut(at.y).unwrap().len() && (at.y + 1 < self.len()) {
            let next_row = self.rows.remove(at.y + 1);
            let row = self.rows.get_mut(at.y).unwrap();
            row.append(&next_row);
            row.highlight(self.file_type.highlight_options(), None);
        } else {
            let row = self.rows.get_mut(at.y).unwrap();
            row.delete(at.x);
            row.highlight(self.file_type.highlight_options(), None);
        }
    }

    pub fn find(&self, query: &str, at: &Position, direction: SearchDirection) -> Option<Position> {
        if at.y >= self.rows.len() {
            return None;
        }
        let mut position = Position { x: at.x, y: at.y };
        let start = if direction == SearchDirection::Forward {
            at.y
        } else {
            0
        };
        let end = if direction == SearchDirection::Forward {
            self.rows.len()
        } else {
            at.y + 1
        };

        for _ in start..end {
            if let Some(row) = self.rows.get(position.y) {
                if let Some(index) = row.find(query, position.x, direction) {
                    position.x = index;
                    return Some(position);
                }
                if direction == SearchDirection::Forward {
                    position.y = position.y.saturating_add(1);
                    position.x = 0;
                } else {
                    position.y = position.y.saturating_sub(1);
                    position.x = self.rows.get(position.y).unwrap().len();
                }
            } else {
                return None;
            }
        }
        None
    }

    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    pub fn highlight(&mut self, word: Option<&str>) {
        for row in &mut self.rows {
            row.highlight(self.file_type.highlight_options(), word);
        }
    }

    pub fn file_type(&self) -> &str {
        self.file_type.name()
    }

}
