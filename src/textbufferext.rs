use crate::texttag::{ParFormat, Tag, TextTagExt2};
use crate::texttagtable::TextTagTable;

use gtk::TextBufferExt;
use gtk::TextIter;
use gtk::TextTagExt;

pub const LINK_START: &str = "LINK:";
pub const IMAGE_START: &str = "IMAGE:";

pub trait TextBufferExt2 {
    fn get_current_word_bounds(&self) -> Option<(TextIter, TextIter)>;

    fn get_insert_iter(&self) -> TextIter;

    // ToDo: duplicated code for image and link
    fn create_image_tag(&self, link: &str) -> gtk::TextTag;
    fn get_image_at_iter(&self, iter: &gtk::TextIter) -> Option<(String, gtk::TextTag)>;

    fn create_link_tag(&self, link: &str) -> gtk::TextTag;
    fn get_link_at_iter(&self, iter: &gtk::TextIter) -> Option<(String, gtk::TextTag)>;

    fn apply_paragraph_format(&self, format: Option<ParFormat>, start: &TextIter, end: &TextIter);
}

impl TextBufferExt2 for gtk::TextBuffer {
    // Current word for cursor at start or in word, NOT at the end.
    fn get_current_word_bounds(&self) -> Option<(TextIter, TextIter)> {
        let mut start = self.get_iter_at_mark(&self.get_insert());
        let mut end = start.clone();
        if start.starts_word() {
            end.forward_word_end();
        } else if start.inside_word() {
            start.backward_word_start();
            end.forward_word_end();
        } else {
            return None;
        }
        Some((start, end))
    }

    fn get_insert_iter(&self) -> TextIter {
        self.get_iter_at_mark(&self.get_insert())
    }

    fn create_image_tag(&self, link: &str) -> gtk::TextTag {
        // ToDo: this lookup might be slow
        let name = format!("{}{}", IMAGE_START, link);
        let table = &self.get_tag_table();
        if let Some(tag) = table.lookup(&name) {
            tag
        } else {
            static GREEN: gdk::RGBA =
                gdk::RGBA { red: 0f32, green: 0.75f32, blue: 0f32, alpha: 1f32 };
            let link_tag = TextTagTable::create_tag(&name, table);
            link_tag.set_property_underline(gtk::pango::Underline::Single);
            link_tag.set_property_foreground_rgba(Some(&GREEN));
            link_tag
        }
    }

    fn get_image_at_iter(&self, iter: &TextIter) -> Option<(String, gtk::TextTag)> {
        let tags = iter.get_tags();
        for tag in tags {
            if let Some(image) = tag.get_image() {
                return Some((image, tag));
            }
        }
        // the link should also be found with the cursor at the end of the tag
        let tags = iter.get_toggled_tags(false);
        for tag in tags {
            if let Some(image) = tag.get_image() {
                return Some((image, tag));
            }
        }

        None
    }

    fn create_link_tag(&self, link: &str) -> gtk::TextTag {
        // ToDo: this lookup might be slow
        let name = format!("{}{}", LINK_START, link);
        let table = &self.get_tag_table();
        if let Some(tag) = table.lookup(&name) {
            tag
        } else {
            static BLUE: gdk::RGBA = gdk::RGBA { red: 0f32, green: 0f32, blue: 1f32, alpha: 1f32 };
            let link_tag = TextTagTable::create_tag(&name, table);
            link_tag.set_property_underline(gtk::pango::Underline::Single);
            link_tag.set_property_foreground_rgba(Some(&BLUE));
            link_tag
        }
    }

    fn get_link_at_iter(&self, iter: &TextIter) -> Option<(String, gtk::TextTag)> {
        let tags = iter.get_tags();
        for tag in tags {
            if let Some(link) = tag.get_link() {
                return Some((link, tag));
            }
        }
        // the link should also be found with the cursor at the end of the tag
        let tags = iter.get_toggled_tags(false);
        for tag in tags {
            if let Some(link) = tag.get_link() {
                return Some((link, tag));
            }
        }

        None
    }

    fn apply_paragraph_format(&self, format: Option<ParFormat>, start: &TextIter, end: &TextIter) {
        self.begin_user_action();

        self.remove_all_tags(&start, &end);
        if let Some(f) = format {
            let tag_name = Tag::from(&f);
            let tag = &self.get_tag_table().lookup(tag_name).unwrap();
            self.apply_tag(tag, &start, &end);
        }

        self.end_user_action();
    }
}
