use std::cmp;

use unicode_segmentation::UnicodeSegmentation;

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
            .take(end - start) {
                if grapheme == "\t" {
                    result.push_str(" ");
                } else {
                    result.push_str(grapheme);
                }
            }
        result
    }

    pub fn len(&self) -> usize {
        self.string[..].graphemes(true).count()
    }

    fn update_len(&mut self) {
        self.len = self.len();
    }
}