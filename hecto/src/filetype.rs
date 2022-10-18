pub struct FileType {
    name: String,
    hl_opts: HighlightOptions,
}

#[derive(Default, Clone, Copy)]
pub struct HighlightOptions {
    avail_numbers: bool,
    avail_strings: bool,
    avail_characters: bool,
    avail_comment: bool,
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
        let mut file_type = Self::default();
        if let Some(extension) = filename.split('.').last() {
            if extension == "rs" {
                file_type.name = String::from("Rust");
                file_type.hl_opts.avail_numbers = true;
                file_type.hl_opts.avail_strings = true;
                file_type.hl_opts.avail_characters = true;
                file_type.hl_opts.avail_comment = true;
            }
        }
        file_type
    }

    pub fn highlight_options(&self) -> HighlightOptions {
        self.hl_opts
    }
}

impl HighlightOptions {
    pub fn numbers(self) -> bool {
        self.avail_numbers
    }
    pub fn strings(self) -> bool {
        self.avail_strings
    }
    pub fn characters(self) -> bool {
        self.avail_characters
    }
    pub fn comment(self) -> bool {
        self.avail_comment
    }
}