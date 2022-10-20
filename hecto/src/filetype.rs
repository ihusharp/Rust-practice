pub struct FileType {
    name: String,
    hl_opts: HighlightOptions,
}

#[derive(Default)]
pub struct HighlightOptions {
    avail_numbers: bool,
    avail_strings: bool,
    avail_characters: bool,
    avail_comment: bool,
    avail_multi_comment: bool,
    primary_keywords: Vec<String>,
    secondary_keywords: Vec<String>,
}

impl Default for FileType {
    fn default() -> Self {
        Self {
            name: String::from("No FileType"),
            hl_opts: HighlightOptions::default(),
        }
    }
}

impl FileType {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn from(filename: &str) -> Self {
        if let Some(extension) = filename.split('.').last() {
            if extension == "rs" {
                return Self {
                    name: String::from("Rust"),
                    hl_opts: HighlightOptions {
                        avail_numbers: true,
                        avail_strings: true,
                        avail_characters: true,
                        avail_comment: true,
                        avail_multi_comment: true,
                        primary_keywords: vec![
                            "as".to_string(),
                            "break".to_string(),
                            "const".to_string(),
                            "continue".to_string(),
                            "crate".to_string(),
                            "else".to_string(),
                            "enum".to_string(),
                            "extern".to_string(),
                            "false".to_string(),
                            "fn".to_string(),
                            "for".to_string(),
                            "if".to_string(),
                            "impl".to_string(),
                            "in".to_string(),
                            "let".to_string(),
                            "loop".to_string(),
                            "match".to_string(),
                            "mod".to_string(),
                            "move".to_string(),
                            "mut".to_string(),
                            "pub".to_string(),
                            "ref".to_string(),
                            "return".to_string(),
                            "self".to_string(),
                            "Self".to_string(),
                            "static".to_string(),
                            "struct".to_string(),
                            "super".to_string(),
                            "trait".to_string(),
                            "true".to_string(),
                            "type".to_string(),
                            "unsafe".to_string(),
                            "use".to_string(),
                            "where".to_string(),
                            "while".to_string(),
                            "dyn".to_string(),
                            "abstract".to_string(),
                            "become".to_string(),
                            "box".to_string(),
                            "do".to_string(),
                            "final".to_string(),
                            "macro".to_string(),
                            "override".to_string(),
                            "priv".to_string(),
                            "typeof".to_string(),
                            "unsized".to_string(),
                            "virtual".to_string(),
                            "yield".to_string(),
                            "async".to_string(),
                            "await".to_string(),
                            "try".to_string(),
                        ],
                        secondary_keywords: vec![
                            "bool".to_string(),
                            "char".to_string(),
                            "i8".to_string(),
                            "i16".to_string(),
                            "i32".to_string(),
                            "i64".to_string(),
                            "isize".to_string(),
                            "u8".to_string(),
                            "u16".to_string(),
                            "u32".to_string(),
                            "u64".to_string(),
                            "usize".to_string(),
                            "f32".to_string(),
                            "f64".to_string(),
                        ],
                    },
                };
            }
        }
        Self::default()
    }

    pub fn highlight_options(&self) -> &HighlightOptions {
        &self.hl_opts
    }
}

impl HighlightOptions {
    pub fn numbers(&self) -> bool {
        self.avail_numbers
    }
    pub fn strings(&self) -> bool {
        self.avail_strings
    }
    pub fn characters(&self) -> bool {
        self.avail_characters
    }
    pub fn comment(&self) -> bool {
        self.avail_comment
    }
    pub fn primary_keywords(&self) -> &Vec<String> {
        &self.primary_keywords
    }
    pub fn secondary_keywords(&self) -> &Vec<String> {
        &self.secondary_keywords
    }
    pub fn multi_comment(&self) -> bool {
        self.avail_multi_comment
    }
}
