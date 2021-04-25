use crate::texttag::{ParFormat, Tag, TextTagExt2};
use crate::texttagtable::TextTagTable;

use crate::textbuffermd::NEWLINE;
use gtk::TextBufferExt;
use gtk::TextIter;
use gtk::TextTagExt;
use std::path::PathBuf;

pub const LINK_START: &str = "LINK:";
pub const IMAGE_START: &str = "IMAGE:";

pub fn is_file(link: &str) -> bool {
    link.starts_with("file:///")
}

pub fn get_file_name(link: &str) -> String {
    let path = PathBuf::from(link);
    if let Some(file) = path.file_name() {
        percent_encoding::percent_decode_str(file.to_str().unwrap_or(link))
            .decode_utf8_lossy()
            .to_string()
    } else {
        link.to_string()
    }
}

pub trait TextBufferExt2 {
    fn clear(&self);

    fn get_current_word_bounds(&self) -> Option<(TextIter, TextIter)>;

    fn get_insert_iter(&self) -> TextIter;

    // ToDo: duplicated code for image and link
    fn create_image_tag(&self, link: &str) -> gtk::TextTag;
    fn get_image_at_iter(&self, iter: &gtk::TextIter) -> Option<(String, gtk::TextTag)>;

    fn apply_link_offset(&self, iter: &gtk::TextIter, link: &str, title: &str, start_offset: i32);
    fn create_link_tag(&self, link: &str) -> gtk::TextTag;
    fn get_link_at_iter(&self, iter: &gtk::TextIter) -> Option<(String, gtk::TextTag)>;

    fn apply_paragraph_format(&self, format: Option<ParFormat>, start: &TextIter, end: &TextIter);

    fn get_new_mark_at(
        &self,
        name: Option<&str>,
        left_gravity: bool,
        where_: &gtk::TextIter,
    ) -> gtk::TextMark;

    fn text_move(&self, up: bool);
}

impl TextBufferExt2 for gtk::TextBuffer {
    fn clear(&self) {
        self.begin_irreversible_action();
        self.delete(&mut self.start_iter(), &mut self.end_iter());
        self.end_irreversible_action();
    }

    // Current word for cursor at start or in word, NOT at the end.
    fn get_current_word_bounds(&self) -> Option<(TextIter, TextIter)> {
        let mut start = self.iter_at_mark(&self.get_insert());
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
        self.iter_at_mark(&self.get_insert())
    }

    fn create_image_tag(&self, link: &str) -> gtk::TextTag {
        // ToDo: this lookup might be slow
        let name = format!("{}{}", IMAGE_START, link);
        let table = &self.tag_table();
        if let Some(tag) = table.lookup(&name) {
            tag
        } else {
            static GREEN: gdk::RGBA =
                gdk::RGBA { red: 0f32, green: 0.75f32, blue: 0f32, alpha: 1f32 };
            let link_tag = TextTagTable::create_tag(&name, table);
            link_tag.set_underline(gtk::pango::Underline::Single);
            link_tag.set_foreground_rgba(Some(&GREEN));
            link_tag
        }
    }

    fn get_image_at_iter(&self, iter: &TextIter) -> Option<(String, gtk::TextTag)> {
        let tags = iter.tags();
        for tag in tags {
            if let Some(image) = tag.get_image() {
                return Some((image, tag));
            }
        }
        // the link should also be found with the cursor at the end of the tag
        let tags = iter.toggled_tags(false);
        for tag in tags {
            if let Some(image) = tag.get_image() {
                return Some((image, tag));
            }
        }

        None
    }

    fn apply_link_offset(&self, iter: &gtk::TextIter, link: &str, title: &str, start_offset: i32) {
        let mut start = iter.clone();
        start.backward_chars(iter.offset() - start_offset);
        let tag = if title.is_empty() {
            self.create_link_tag(link)
        } else {
            self.create_link_tag(format!("{} \"{}\"", link, title).as_str())
        };
        self.apply_tag(&tag, &start, &iter);
    }

    fn create_link_tag(&self, link: &str) -> gtk::TextTag {
        let name = format!("{}{}", LINK_START, link);
        let is_file = is_file(link);
        let table = &self.tag_table();
        // ToDo: this lookup might be slow
        if let Some(tag) = table.lookup(&name) {
            tag
        } else {
            static BLUE: gdk::RGBA = gdk::RGBA { red: 0f32, green: 0f32, blue: 1f32, alpha: 1f32 };
            static ORANGE: gdk::RGBA =
                gdk::RGBA { red: 0.9f32, green: 0.5f32, blue: 0f32, alpha: 1f32 };
            let link_tag = TextTagTable::create_tag(&name, table);
            link_tag.set_underline(gtk::pango::Underline::Single);
            link_tag.set_foreground_rgba(Some(if is_file { &ORANGE } else { &BLUE }));
            link_tag
        }
    }

    fn get_link_at_iter(&self, iter: &TextIter) -> Option<(String, gtk::TextTag)> {
        let tags = iter.tags();
        for tag in tags {
            if let Some(link) = tag.get_link() {
                return Some((link, tag));
            }
        }
        // the link should also be found with the cursor at the end of the tag
        let tags = iter.toggled_tags(false);
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
            let tag_name = Tag::from_par_format(&f);
            let tag = &self.tag_table().lookup(tag_name).unwrap();
            self.apply_tag(tag, &start, &end);
        }

        self.end_user_action();
    }

    fn get_new_mark_at(
        &self,
        name: Option<&str>,
        left_gravity: bool,
        where_: &gtk::TextIter,
    ) -> gtk::TextMark {
        let mark = gtk::TextMark::new(name, left_gravity);
        self.add_mark(&mark, where_);
        mark
    }

    // Moves complete lines up and down. An optional selection is extendend to the complete lines.
    // ToDo: A selection with an empty line at the end gets stripped of it at the end of the buffer.
    fn text_move(&self, up: bool) {
        let mut start = self.get_insert_iter();
        let mut end = start.clone();
        let mut has_selection = false;
        if let Some((s, e)) = self.selection_bounds() {
            start = s;
            end = e;
            if end.starts_line() {
                end.backward_line();
                if !end.ends_line() {
                    end.forward_to_line_end();
                }
            }
            has_selection = true;
        }

        let line_start = start.line();
        let line_end = end.line();
        if (line_start == 0 && up) || (line_end == self.line_count() - 1 && !up) {
            return;
        }

        start.set_line(line_start); // move start to line beginning
        let add_ending_nl = line_end == self.line_count() - 1; // only when moving up!
        let mut add_beginning_nl = false; // only when moving down!
        end.forward_line();

        // calculate where to insert the content (move target)
        let mut insert = start.clone();
        if up {
            insert.backward_line();
        } else {
            insert = end.clone();
            if !insert.ends_line() {
                insert.forward_to_line_end();
            }
            if insert.is_end() {
                add_beginning_nl = true;
            } else {
                insert.forward_line();
            }
        }
        let mark_insert = self.get_new_mark_at(None, true, &insert);

        // copy the content to another buffer
        let other = gtk::TextBuffer::new(Some(&self.tag_table()));
        let mut end_for_copy = end.clone();
        if add_beginning_nl {
            other.insert_at_cursor(NEWLINE);
            end_for_copy.backward_char();
        }
        other.insert_range(&mut other.end_iter(), &start, &end_for_copy);
        if add_ending_nl {
            other.insert_at_cursor(NEWLINE);
            start.backward_char();
        }

        // buffer modification, delete old content and insert at new position
        self.begin_user_action();
        self.delete(&mut start, &mut end);
        self.insert_range(
            &mut self.iter_at_mark(&mark_insert),
            &other.start_iter(),
            &other.end_iter(),
        );
        self.end_user_action();

        // move the cursor to the expected location and restore equivalent selection
        let mut cursor = self.iter_at_mark(&mark_insert);
        if add_beginning_nl {
            cursor.forward_line();
        }
        if has_selection {
            let mut sel_end = cursor.clone();
            sel_end.forward_lines(line_end - line_start + 1);
            self.select_range(&cursor, &sel_end);
        } else {
            self.place_cursor(&cursor);
        }
    }
}
