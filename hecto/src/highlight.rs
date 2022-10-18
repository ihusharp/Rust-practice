use termion::color;

#[derive(PartialEq, Eq)]
pub enum Type {
    None,
    Number,
    Match,
    String,
    Character,
    Comment,
}

impl Type {
    pub fn to_color(&self) -> impl color::Color {
        match self {
            Type::Number => color::Rgb(220, 163, 163),
            Type::Match => color::Rgb(38, 139, 210),
            Type::String => color::Rgb(181, 137, 0),
            Type::Character => color::Rgb(108, 113, 196),
            Type::Comment => color::Rgb(147, 161, 110),
            Type::None => color::Rgb(255, 255, 255),
        }
    }
}