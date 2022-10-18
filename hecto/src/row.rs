use std::cmp;

use termion::color;
use unicode_segmentation::UnicodeSegmentation;

use crate::{editor::SearchDirection, highlight, filetype::HighlightOptions};

#[derive(Default)]
pub struct Row {
    string: String,
    len: usize,
    highlight: Vec<highlight::Type>,
}

impl Row {
    pub fn from(string: &str) -> Self {
        let mut row = Self {
            string: String::from(string),
            len: string.graphemes(true).count(),
            highlight: Vec::new(),
        };
        row.update_len();
        row
    }

    // now return the biggest possible substring it can generate
    pub fn render(&self, start: usize, end: usize) -> String {
        let end = cmp::min(end, self.string.len());
        let start = cmp::min(start, end);
        let mut result = String::new();
        let mut current_highlight = &highlight::Type::None;
        for (index, grapheme) in self.string[..]
            .graphemes(true)
            .enumerate()
            .skip(start)
            .take(end - start)
        {
            if let Some(c) = grapheme.chars().next() {
                let highlight_type = self.
                    highlight.get(index).unwrap_or(&highlight::Type::None);
                if highlight_type != current_highlight {
                    current_highlight = highlight_type;
                    let start_highlight = format!(
                        "{}",
                        termion::color::Fg(highlight_type.to_color()),
                    );
                    result.push_str(&start_highlight);
                }
                if c == '\t' {
                    result.push(' ');
                } else { 
                    result.push(c);
                } 
            }
        }
        let end_highlight = format!(
            "{}{}",
            termion::color::Fg(color::Reset),
            termion::style::Reset
        );
        result.push_str(&end_highlight);  
        result
    }

    pub fn highlight(&mut self, opts: HighlightOptions, word: Option<&str>) {
        let mut highlighting = Vec::new();
        let chars = self.string.chars().collect::<Vec<char>>();
        let mut matches = Vec::new();
        let mut search_index = 0;

        if let Some(word) = word {
            while let Some(search_match) = self.find(word, search_index, SearchDirection::Forward) {
                matches.push(search_match);
                if let Some(next_index) = search_match.checked_add(word[..].graphemes(true).count()) {
                    search_index = next_index;
                } else {
                    break;
                }
            }
        }

        let mut prev_is_separator = true;
        let mut in_string = false;
        let mut index = 0;
        while let Some(c) = chars.get(index) {
            if let Some(word) = word {
                if matches.contains(&index) {
                    // add word len
                    for _ in word[..].graphemes(true) {
                        index += 1;
                        highlighting.push(highlight::Type::Match);
                    }
                    continue;
                }
            }
            let prev_highlight = if index > 0 {
                highlighting
                    .get(index - 1)
                    .unwrap_or(&highlight::Type::None)
            } else {
                &highlight::Type::None
            };
            if opts.characters() && !in_string && *c == '\'' {
                prev_is_separator = true;
                if let Some(next_char) = chars.get(index.saturating_add(1)) {
                    let closing_index = if *next_char == '\\' {
                        index.saturating_add(3)
                    } else {
                        index.saturating_add(2)
                    };
                    if let Some(closing_char) = chars.get(closing_index) {
                        if *closing_char == '\'' {
                            for _ in 0..=closing_index.saturating_sub(index) {
                                highlighting.push(highlight::Type::Character);
                                index += 1;
                            }
                            continue;
                        }
                    }
                }
                highlighting.push(highlight::Type::None);
                index += 1;
                continue;
            }
            if opts.comment() && *c == '/' {
                if let Some(next_ch) = chars.get(index.saturating_add(1)) {
                    if *next_ch == '/' {
                        for _ in index..chars.len() {
                            highlighting.push(highlight::Type::Comment);
                        }
                        break;
                    }
                }
            }

            if opts.strings() {
                if in_string {
                    highlighting.push(highlight::Type::String);
                    if *c == '\\' && index + 1 < self.len() {
                        highlighting.push(highlight::Type::String);
                        index += 2;
                        continue;
                    }
                    if *c == '"'{
                        in_string = false;
                        prev_is_separator = true;
                    } else {
                        prev_is_separator = false;
                    }
                    index += 1;
                    continue;
                } else if *c == '"' && prev_is_separator {
                    highlighting.push(highlight::Type::String);
                    in_string = true;
                    prev_is_separator = true;
                    index += 1;
                    continue;
                }
            }
            if opts.numbers() {
                if (c.is_ascii_digit() && (prev_is_separator || *prev_highlight == highlight::Type::Number))
                || (*c == '.' && *prev_highlight == highlight::Type::Number) {
                    highlighting.push(highlight::Type::Number);
                } else {
                highlighting.push(highlight::Type::None);
            }
            } else {
                highlighting.push(highlight::Type::None);
            }
            prev_is_separator = c.is_ascii_punctuation() || c.is_ascii_whitespace();
            index += 1;
        }
        self.highlight = highlighting;
    }

    pub fn len(&self) -> usize {
        self.string[..].graphemes(true).count()
    }

    pub fn insert(&mut self, at: usize, c: char) {
        if at >= self.len() {
            self.string.push(c);
            self.len += 1;
            return;
        } 
        let mut result = String::new();
        let mut length = 0;
        for (index, grapheme) in self.string[..].graphemes(true).enumerate() {
            length += 1;
            if index == at {
                length += 1;
                result.push(c);
            }
            result.push_str(grapheme);
        }
        self.len = length;
        self.string = result;
    }

    pub fn delete(&mut self, at: usize) {
        if at >= self.len() {
            return;
        }
        let mut result = String::new();
        let mut length = 0;
        for (index, grapheme) in self.string[..].graphemes(true).enumerate() {
            if index != at {
                length += 1;
                result.push_str(grapheme);
            }
        }
        self.len = length;
        self.string = result;
        
        self.update_len();
    }

    pub fn append(&mut self, new: &Self) {
        self.string = format!("{}{}", self.string, new.string);
        self.update_len();
    }

    pub fn split(&mut self, at: usize) -> Self {
        let mut row = String::new();
        let mut length = 0;
        let mut splitted_row = String::new();
        let mut splitted_length = 0;
        for (index, grapheme) in self.string[..].graphemes(true).enumerate() {
            if index < at {
                length += 1;
                row.push_str(grapheme);
            } else {
                splitted_length += 1;
                splitted_row.push_str(grapheme);
            }
        }
        self.string = row;
        self.len = length;
        Self {
            string: splitted_row,
            len: splitted_length,
            highlight: Vec::new(),
        }
    }

    pub fn find(&self, query: &str, at: usize, direction: SearchDirection) -> Option<usize> {
        if at > self.len || query.is_empty() {
            return None;
        }
        let start = if direction == SearchDirection::Forward {
            at
        } else {
            0
        };
        let end = if direction == SearchDirection::Forward {
            self.len
        } else {
            at
        };
        
        let substring = self.string[..].graphemes(true).skip(start).take(end - start).collect::<String>();
        let matching_byte_index = if direction == SearchDirection::Forward {
            substring.find(query)
        } else {
            substring.rfind(query)
        };
        if let Some(match_index) = matching_byte_index {
            for (index, (byte_index, _)) in substring[..].grapheme_indices(true).enumerate() {
                if match_index == byte_index {
                    return Some(start + index);
                }
            }
        }
        None
    }

    fn update_len(&mut self) {
        self.len = self.len();
    }
    pub fn as_bytes(&self) -> &[u8] {
        self.string.as_bytes()
    }
}
