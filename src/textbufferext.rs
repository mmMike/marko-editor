use crate::texttag::{ParFormat, Tag};
use crate::texttagtable::LINK_DIVIDER;

use gtk::TextBufferExt;
use gtk::TextIter;
use gtk::TextTagExt;

pub trait TextBufferExt2 {
    fn get_current_word_bounds(&self) -> Option<(TextIter, TextIter)>;

    fn get_insert_iter(&self) -> TextIter;

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

    fn get_link_at_iter(&self, iter: &TextIter) -> Option<(String, gtk::TextTag)> {
        let tags = iter.get_tags();
        for tag in tags {
            let name = String::from(tag.get_property_name().unwrap().as_str());
            if let Some(idx) = name.find(LINK_DIVIDER) {
                return Some((String::from(&name[idx + 1..]), tag));
            }
        }
        // the link should also be found with the cursor at the end of the tag
        let tags = iter.get_toggled_tags(false);
        for tag in tags {
            let name = String::from(tag.get_property_name().unwrap().as_str());
            if let Some(idx) = name.find(LINK_DIVIDER) {
                return Some((String::from(&name[idx + 1..]), tag));
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
