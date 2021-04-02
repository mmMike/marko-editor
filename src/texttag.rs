use crate::textbufferext::{IMAGE_START, LINK_START};
use gtk::TextTagExt;

#[derive(Debug, PartialEq, Eq)]
pub enum CharFormat {
    Bold,
    Italic,
    Mono,
    Strike,

    Red,
    Green,
    Blue,
    Yellow,
}

pub const COLORS: [CharFormat; 4] =
    [CharFormat::Red, CharFormat::Green, CharFormat::Blue, CharFormat::Yellow];

#[derive(Debug)]
pub enum ParFormat {
    H1,
    H2,
    H3,
    H4,
    H5,
    H6,
    Code,
}

pub struct Tag {}
impl Tag {
    pub const H1: &'static str = "h1";
    pub const H2: &'static str = "h2";
    pub const H3: &'static str = "h3";
    pub const H4: &'static str = "h4";
    pub const H5: &'static str = "h5";
    pub const H6: &'static str = "h6";

    pub const CODE: &'static str = "code";

    pub const BOLD: &'static str = "weight=700";
    pub const ITALIC: &'static str = "style=2";
    pub const MONO: &'static str = "family=Monospace";
    pub const STRIKE: &'static str = "strikethrough=1";

    // inspired from critics markdown
    pub const RED: &'static str = "red"; // removal
    pub const GREEN: &'static str = "green"; // addition
    pub const BLUE: &'static str = "blue"; // comment
    pub const YELLOW: &'static str = "yellow"; // highlight

    pub const SEARCH: &'static str = "search"; // highlight for search results

    pub const RULE: &'static str = "rule";
    pub const MD_RULE: &'static str = "--- ---- ----- ------- ----- ---- ---";

    pub fn from_char_format(format: &CharFormat) -> &'static str {
        match format {
            CharFormat::Bold => Tag::BOLD,
            CharFormat::Italic => Tag::ITALIC,
            CharFormat::Mono => Tag::MONO,
            CharFormat::Strike => Tag::STRIKE,
            CharFormat::Red => Tag::RED,
            CharFormat::Green => Tag::GREEN,
            CharFormat::Blue => Tag::BLUE,
            CharFormat::Yellow => Tag::YELLOW,
        }
    }

    pub fn from_par_format(format: &ParFormat) -> &'static str {
        match format {
            ParFormat::H1 => Tag::H1,
            ParFormat::H2 => Tag::H2,
            ParFormat::H3 => Tag::H3,
            ParFormat::H4 => Tag::H4,
            ParFormat::H5 => Tag::H5,
            ParFormat::H6 => Tag::H6,
            ParFormat::Code => Tag::CODE,
        }
    }

    pub fn header_level(par_format: &ParFormat) -> Option<u32> {
        match par_format {
            ParFormat::H1 => Some(1),
            ParFormat::H2 => Some(2),
            ParFormat::H3 => Some(3),
            ParFormat::H4 => Some(4),
            ParFormat::H5 => Some(5),
            ParFormat::H6 => Some(6),
            _ => None,
        }
    }
}

pub trait TextTagExt2 {
    fn get_name(&self) -> String;

    fn get_image(&self) -> Option<String>;
    fn get_link(&self) -> Option<String>;

    fn get_par_format(&self) -> Option<ParFormat>;
}

impl TextTagExt2 for gtk::TextTag {
    fn get_name(&self) -> String {
        String::from(self.get_property_name().unwrap().as_str())
    }

    fn get_image(&self) -> Option<String> {
        let mut name = self.get_name();
        if name.starts_with(IMAGE_START) {
            name.replace_range(..6, "");
            Some(name)
        } else {
            None
        }
    }

    fn get_link(&self) -> Option<String> {
        let mut name = self.get_name();
        if name.starts_with(LINK_START) {
            name.replace_range(..5, "");
            Some(name)
        } else {
            None
        }
    }

    fn get_par_format(&self) -> Option<ParFormat> {
        match self.get_name().as_str() {
            Tag::H1 => Some(ParFormat::H1),
            Tag::H2 => Some(ParFormat::H2),
            Tag::H3 => Some(ParFormat::H3),
            Tag::H4 => Some(ParFormat::H4),
            Tag::H5 => Some(ParFormat::H5),
            Tag::H6 => Some(ParFormat::H6),
            Tag::CODE => Some(ParFormat::Code),
            _ => None,
        }
    }
}
