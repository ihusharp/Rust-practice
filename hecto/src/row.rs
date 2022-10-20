use std::cmp;

use termion::color;
use unicode_segmentation::UnicodeSegmentation;

use crate::{editor::SearchDirection, filetype::HighlightOptions, highlight};

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
                let highlight_type = self.highlight.get(index).unwrap_or(&highlight::Type::None);
                if highlight_type != current_highlight {
                    current_highlight = highlight_type;
                    let start_highlight =
                        format!("{}", termion::color::Fg(highlight_type.to_color()),);
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

        let substring = self.string[..]
            .graphemes(true)
            .skip(start)
            .take(end - start)
            .collect::<String>();
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

    fn highlight_match(&mut self, word: Option<&str>) {
        if let Some(word) = word {
            if word.is_empty() {
                return;
            }
            let mut index = 0;
            while let Some(search_match) = self.find(word, index, SearchDirection::Forward) {
                if let Some(next_index) = search_match.checked_add(word[..].graphemes(true).count())
                {
                    for i in search_match..next_index {
                        self.highlight[i] = highlight::Type::Match;
                    }
                    index = next_index;
                } else {
                    break;
                }
            }
        }
    }

    fn highlight_char(
        &mut self,
        index: &mut usize,
        opts: &HighlightOptions,
        c: char,
        chars: &[char],
    ) -> bool {
        if opts.characters() && c == '\'' {
            if let Some(next_char) = chars.get(index.saturating_add(1)) {
                let closing_index = if *next_char == '\\' {
                    index.saturating_add(3)
                } else {
                    index.saturating_add(2)
                };
                if let Some(closing_char) = chars.get(closing_index) {
                    if *closing_char == '\'' {
                        for _ in 0..=closing_index.saturating_sub(*index) {
                            self.highlight.push(highlight::Type::Character);
                            *index += 1;
                        }
                        return true;
                    }
                }
            }
        }
        false
    }

    fn highlight_comment(
        &mut self,
        index: &mut usize,
        opts: &HighlightOptions,
        c: char,
        chars: &[char],
    ) -> bool {
        if opts.comment() && c == '/' {
            if let Some(next_ch) = chars.get(index.saturating_add(1)) {
                if *next_ch == '/' {
                    for _ in *index..chars.len() {
                        self.highlight.push(highlight::Type::Comment);
                        *index += 1;
                    }
                    return true;
                }
            }
        }
        false
    }

    fn highlight_multiline_comment(
        &mut self,
        index: &mut usize,
        opts: &HighlightOptions,
        c: char,
        chars: &[char],
    ) -> bool {
        if opts.multi_comment() && c == '/' && *index < chars.len() {
            if let Some(next_ch) = chars.get(index.saturating_add(1)) {
                if *next_ch == '*' {
                    let closing_index = if let Some(closing_index) =
                        self.string[index.saturating_add(2)..].find("*/") {
                            *index + closing_index + 4
                        } else {
                            chars.len()
                        };
                    for _ in *index..closing_index {
                        self.highlight.push(highlight::Type::MultiComment);
                        *index += 1;
                    }
                    return true;
                }
            }
        }
        false
    }

    fn highlight_string(
        &mut self,
        index: &mut usize,
        opts: &HighlightOptions,
        c: char,
        chars: &[char],
    ) -> bool {
        if opts.strings() && c == '"' {
            loop {
                self.highlight.push(highlight::Type::String);
                *index += 1;
                if let Some(next_char) = chars.get(*index) {
                    if *next_char == '"' {
                        break;
                    }
                } else {
                    break;
                }
            }
            self.highlight.push(highlight::Type::String);
            *index += 1;
            return true;
        }
        false
    }

    fn highlight_number(
        &mut self,
        index: &mut usize,
        opts: &HighlightOptions,
        c: char,
        chars: &[char],
    ) -> bool {
        if opts.numbers() && c.is_ascii_digit() {
            if *index > 0 {
                let prev_ch = chars[*index - 1];
                if !prev_ch.is_ascii_punctuation() && !prev_ch.is_ascii_whitespace() {
                    return false;
                }
            }
            loop {
                self.highlight.push(highlight::Type::Number);
                *index += 1;
                if let Some(next_char) = chars.get(*index) {
                    if !next_char.is_ascii_digit() {
                        break;
                    }
                } else {
                    break;
                }
            }
            return true;
        }
        false
    }

    fn highlight_word(
        &mut self,
        index: &mut usize,
        word: &str,
        chars: &[char],
        highlight_type: highlight::Type,
    ) -> bool {
        if word.is_empty() {
            return false;
        }
        for (word_index, ch) in word.chars().enumerate() {
            if let Some(next_char) = chars.get(index.saturating_add(word_index)) {
                if *next_char != ch {
                    return false;
                }
            } else {
                return false;
            }
        }
        for _ in 0..word.len() {
            self.highlight.push(highlight_type);
            *index += 1;
        }
        true
    }

    fn highlight_keywords(
        &mut self,
        index: &mut usize,
        keywords: &Vec<String>,
        chars: &[char],
        highlight_type: highlight::Type,
    ) -> bool {
        if *index > 0 {
            let prev_ch = chars[*index - 1];
            if !prev_ch.is_ascii_punctuation() && !prev_ch.is_ascii_whitespace() {
                return false;
            }
        }
        for word in keywords {
            if *index < chars.len().saturating_sub(word.len()) {
                let next_ch = chars[*index + word.len()];
                if !next_ch.is_ascii_punctuation() && !next_ch.is_ascii_whitespace() {
                    continue;
                }
            }
            if self.highlight_word(index, word, chars, highlight_type) {
                return true;
            }
        }
        false
    }

    fn highlight_primary_keywords(
        &mut self,
        index: &mut usize,
        opts: &HighlightOptions,
        chars: &[char],
    ) -> bool {
        return self.highlight_keywords(
            index,
            opts.primary_keywords(),
            chars,
            highlight::Type::PrimaryKeyword,
        );
    }

    fn highlight_secondary_keywords(
        &mut self,
        index: &mut usize,
        opts: &HighlightOptions,
        chars: &[char],
    ) -> bool {
        return self.highlight_keywords(
            index,
            opts.secondary_keywords(),
            chars,
            highlight::Type::SecondaryKeyword,
        );
    }

    pub fn highlight(
        &mut self,
        opts: &HighlightOptions,
        word: Option<&str>,
        start_with_comment: bool,
    ) -> bool {
        let chars = self.string.chars().collect::<Vec<char>>();
        let mut index = 0;
        let mut in_ml_comment = start_with_comment;
        if in_ml_comment {
            let closing_index = if let Some(closing_index) = self.string.find("*/") {
                closing_index + 2
            } else {
                chars.len()
            };
            for _ in 0..closing_index {
                self.highlight.push(highlight::Type::MultiComment);
            }
            index = closing_index;
        }
        while let Some(c) = chars.get(index) {
            if self.highlight_multiline_comment(&mut index, opts, *c, &chars) {
                in_ml_comment = true;
                continue;
            }
            in_ml_comment = false;
            if self.highlight_char(&mut index, opts, *c, &chars)
                || self.highlight_comment(&mut index, opts, *c, &chars)
                || self.highlight_primary_keywords(&mut index, opts, &chars)
                || self.highlight_secondary_keywords(&mut index, opts, &chars)
                || self.highlight_string(&mut index, opts, *c, &chars)
                || self.highlight_number(&mut index, opts, *c, &chars)
            {
                continue;
            }
            self.highlight.push(highlight::Type::None);
            index += 1;
        }
        self.highlight_match(word);
        if in_ml_comment && &self.string[self.string.len().saturating_sub(2)..] != "*/" {
            return true;
        }
        false
    }
}

#[cfg(test)]
mod test_super {
    use super::*;

    #[test]
    fn test_highlight_find() {
        let mut row = Row::from("1testtest");
        row.highlight = vec![
            highlight::Type::Number,
            highlight::Type::None,
            highlight::Type::None,
            highlight::Type::None,
            highlight::Type::None,
            highlight::Type::None,
            highlight::Type::None,
            highlight::Type::None,
            highlight::Type::None,
        ];
        row.highlight_match(Some("t"));
        assert_eq!(
            vec![
                highlight::Type::Number,
                highlight::Type::Match,
                highlight::Type::None,
                highlight::Type::None,
                highlight::Type::Match,
                highlight::Type::Match,
                highlight::Type::None,
                highlight::Type::None,
                highlight::Type::Match
            ],
            row.highlight
        )
    }

    #[test]
    fn test_find() {
        let row = Row::from("1testtest");
        assert_eq!(row.find("t", 0, SearchDirection::Forward), Some(1));
        assert_eq!(row.find("t", 2, SearchDirection::Forward), Some(4));
        assert_eq!(row.find("t", 5, SearchDirection::Forward), Some(5));
    }
}
