use std::cmp;

use unicode_segmentation::UnicodeSegmentation;

#[derive(Default)]
pub struct Row {
    string: String,
    len: usize,
}

impl Row {
    pub fn from(string: &str) -> Self {
        let mut row = Self {
            string: String::from(string),
            len: 0,
        };
        row.update_len();
        row
    }

    // now return the biggest possible substring it can generate
    pub fn render(&self, start: usize, end: usize) -> String {
        let end = cmp::min(end, self.string.len());
        let start = cmp::min(start, end);
        let mut result = String::new();
        for grapheme in self.string[..]
            .graphemes(true)
            .skip(start)
            .take(end - start)
        {
            if grapheme == "\t" {
                result.push(' ');
            } else {
                result.push_str(grapheme);
            }
        }
        result
    }

    pub fn len(&self) -> usize {
        self.string[..].graphemes(true).count()
    }

    pub fn insert(&mut self, at: usize, c: char) {
        if at >= self.len() {
            self.string.push(c);
        } else {
            let mut result: String = self.string[..].graphemes(true).take(at).collect();
            let remainder: String = self.string[..].graphemes(true).skip(at).collect();
            result.push(c);
            result.push_str(remainder.as_str());
            self.string = result;
        }
        self.update_len();
    }

    pub fn delete(&mut self, at: usize) {
        if at < self.len() {
            let mut result: String = self.string[..].graphemes(true).take(at).collect();
            let remainder: String = self.string[..].graphemes(true).skip(at + 1).collect();
            result.push_str(remainder.as_str());
            self.string = result;
        } else {
            return;
        }
        self.update_len();
    }

    pub fn append(&mut self, new: &Self) {
        self.string = format!("{}{}", self.string, new.string);
        self.update_len();
    }

    pub fn split(&mut self, at: usize) -> Self {
        let beginning: String = self.string[..].graphemes(true).take(at).collect();
        let remainder: String = self.string[..].graphemes(true).skip(at).collect();
        self.string = beginning;
        self.update_len();
        Self::from(remainder.as_str())
    }

    fn update_len(&mut self) {
        self.len = self.len();
    }
    pub fn as_bytes(&self) -> &[u8] {
        self.string.as_bytes()
    }
}
