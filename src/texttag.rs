use crate::textbufferext::{IMAGE_START, LINK_START};
use gtk::TextTagExt;

#[derive(Debug, PartialEq, Eq)]
pub enum CharFormat {
    BOLD,
    ITALIC,
    MONO,
    STRIKE,

    RED,
    GREEN,
    BLUE,
    YELLOW,
}

#[derive(Debug)]
pub enum ParFormat {
    H1,
    H2,
    H3,
    H4,
    H5,
    H6,
    CODE,
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

    pub const RULE: &'static str = "rule";
    pub const MD_RULE: &'static str = "--- ---- ----- ------- ----- ---- ---";

    pub fn from(par_format: &ParFormat) -> &str {
        match par_format {
            ParFormat::H1 => Tag::H1,
            ParFormat::H2 => Tag::H2,
            ParFormat::H3 => Tag::H3,
            ParFormat::H4 => Tag::H4,
            ParFormat::H5 => Tag::H5,
            ParFormat::H6 => Tag::H6,
            ParFormat::CODE => Tag::CODE,
        }
    }
}

pub trait TextTagExt2 {
    fn get_name(&self) -> String;

    fn get_image(&self) -> Option<String>;
    fn get_link(&self) -> Option<String>;

    fn is_par_format(&self) -> bool;
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

    fn is_par_format(&self) -> bool {
        match self.get_name().as_str() {
            Tag::H1 => Some(ParFormat::H1),
            Tag::H2 => Some(ParFormat::H2),
            Tag::H3 => Some(ParFormat::H3),
            Tag::H4 => Some(ParFormat::H4),
            Tag::H5 => Some(ParFormat::H5),
            Tag::H6 => Some(ParFormat::H6),
            Tag::CODE => Some(ParFormat::CODE),
            _ => None,
        }
        .is_some()
    }
}
