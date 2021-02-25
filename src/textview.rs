use crate::textbufferext::TextBufferExt2;
use crate::textbuffermd::TextBufferMd;
use crate::texttag::{CharFormat, ParFormat, Tag, TextTagExt2};
use crate::texttagmanager::{TextEdit, TextTagManager};
use crate::textviewext::TextViewExt2;
use crate::{builder_get, connect, connect_fwd1};

extern crate html_escape;

use gdk4_x11::glib;
use glib::signal::Inhibit;
use gtk::glib::Value;
use gtk::prelude::*;
use gtk::prelude::{Cast, ObjectExt};
use gtk::ButtonExt;
use gtk::EventControllerKey;
use gtk::GestureSingleExt;
use gtk::TextBufferExt;
use gtk::TextViewExt;
use gtk::WidgetExt;

use regex::Regex;

use std::cell::RefCell;
use std::ops::Deref;
use std::rc::Rc;
use std::thread;

mod keys {
    pub use gdk::keys::constants::*;
}

pub struct LinkData {
    text: String,
    link: String,
    is_image: bool,
}

type OpenLinkCb = Rc<RefCell<Box<dyn Fn(&str)>>>;
type AcceptLinkCb = Rc<RefCell<Box<dyn Fn(Option<&LinkData>)>>>;

fn blocking_get(url: &str) -> Result<reqwest::blocking::Response, reqwest::Error> {
    // ToDo: this client should not be created every time!
    let custom = reqwest::redirect::Policy::custom(|attempt| attempt.follow());
    let client =
        reqwest::blocking::Client::builder().redirect(custom).user_agent("Wget/1.21.1").build()?;
    Ok(client.get(url).send()?)
}

fn fetch_title<F: Fn(&str) + 'static>(url: &str, and_then: F) {
    // See: https://coaxion.net/blog/2019/02/mpsc-channel-api-for-painless-usage-of-threads-with-gtk-in-rust/
    // Create a new sender/receiver pair with default priority
    let (sender, receiver) = gtk::glib::MainContext::channel::<String>(glib::PRIORITY_DEFAULT);

    // Spawn the thread and move the sender in there
    let u = String::from(url);
    thread::spawn(move || {
        if let Ok(res) = blocking_get(u.as_str()) {
            if let Ok(text) = res.text() {
                lazy_static! {
                    // ToDo: this is most likely not helpful with the async setup
                    static ref RE: Regex = Regex::new(r"<title[^>]*>(.*)</title").unwrap();
                }
                if let Some(caps) = RE.captures(&text) {
                    if let Some(c) = caps.get(1) {
                        let decoded = String::from(html_escape::decode_html_entities(c.as_str()));
                        // Sending fails if the receiver is closed
                        let _ = sender.send(decoded);
                    }
                }
            }
        }
    });

    // Attach the receiver to the default main context (None)
    receiver.attach(None, move |msg| {
        and_then(msg.as_str());
        // Returning false here would close the receiver and have senders fail
        gtk::glib::Continue(true)
    });
}

#[derive(Clone)]
struct LinkEdit {
    link_edit_bar: gtk::SearchBar,
    edt_link_name: gtk::Entry,
    edt_link_target: gtk::Entry,
    btn_accept_link: gtk::Button,
    btn_cancel_link: gtk::Button,
    btn_fetch_title: gtk::Button,
    btn_is_image: gtk::ToggleButton,
    accept_link_cb: AcceptLinkCb,
}

impl LinkEdit {
    pub fn new(b: &gtk::Builder) -> Self {
        let this = Self {
            link_edit_bar: builder_get!(b("link_edit_bar")),
            edt_link_name: builder_get!(b("edt_link_name")),
            edt_link_target: builder_get!(b("edt_link_target")),
            btn_accept_link: builder_get!(b("btn_accept_link")),
            btn_cancel_link: builder_get!(b("btn_cancel_link")),
            btn_fetch_title: builder_get!(b("btn_fetch_title")),
            btn_is_image: builder_get!(b("btn_is_image")),
            accept_link_cb: Rc::new(RefCell::new(Box::new(|_| {}))),
        };
        this.btn_accept_link.connect_clicked(connect!(this.accept()));
        this.btn_cancel_link.connect_clicked(connect!(this.reject()));
        this.btn_fetch_title.connect_clicked(connect!(this.fetch_title()));
        this
    }

    pub fn set_accept_link_cb<F: Fn(Option<&LinkData>) + 'static>(&self, accept_link_cb: F) {
        *self.accept_link_cb.borrow_mut() = Box::new(accept_link_cb);
    }

    fn edit_link(&self, link_data: &LinkData) {
        self.edt_link_name.set_text(&link_data.text);
        self.edt_link_target.set_text(&link_data.link);
        self.link_edit_bar.set_search_mode(true);
        self.btn_is_image.set_active(link_data.is_image);

        if link_data.link.is_empty() {
            lazy_static! {
                static ref RE: Regex = Regex::new(r"^\w+://.*").unwrap();
            }
            if RE.is_match(&link_data.text) {
                self.edt_link_target.set_text(&link_data.text);
                self.edt_link_name.grab_focus();
                self.fetch_title();
            } else {
                self.edt_link_target.grab_focus();
            }
        } else {
            self.edt_link_name.grab_focus();
        }
    }

    pub fn accept(&self) {
        self.hide();
        let link_data = LinkData {
            text: self
                .edt_link_name
                .get_text()
                .as_str()
                .split_whitespace()
                .collect::<Vec<&str>>()
                .join(" "),
            link: String::from(self.edt_link_target.get_text().as_str().trim()),
            is_image: self.btn_is_image.get_active(),
        };
        (self.accept_link_cb.borrow())(Some(&link_data));
    }

    pub fn reject(&self) {
        self.hide();
        (self.accept_link_cb.borrow())(None);
    }

    pub fn hide(&self) {
        self.link_edit_bar.set_search_mode(false);
    }

    fn fetch_title(&self) {
        fetch_title(self.edt_link_target.get_text().as_str(), {
            let s = self.clone();
            move |decoded| {
                s.edt_link_name.set_text(decoded);
                if s.link_edit_bar.get_search_mode() {
                    s.edt_link_name.grab_focus();
                }
            }
        })
    }
}

#[derive(Clone)]
pub struct TextView {
    buffer: gtk::TextBuffer,
    tags: Rc<TextTagManager>, // Rc needed for clones in closures
    textview: gtk::TextView,
    link_edit: Rc<LinkEdit>,
    activate_link_cb: OpenLinkCb,
    top_level: gtk::Widget,
    is_editable: Rc<RefCell<bool>>,
    link_start: gtk::TextMark,
    link_end: gtk::TextMark,
}

impl TextView {
    pub fn new() -> Self {
        let ui_src = include_str!("textview.ui");
        let b = gtk::Builder::new();
        b.add_from_string(ui_src).expect("Couldn't add from string");

        let margin = 10;

        let tags = Rc::new(TextTagManager::new());
        let buffer = gtk::TextBuffer::new(Some(tags.table()));

        let textview: gtk::TextView = builder_get!(b("textview"));
        textview.set_top_margin(margin);
        textview.set_bottom_margin(margin);
        textview.set_left_margin(margin);
        textview.set_right_margin(margin);
        textview.set_wrap_mode(gtk::WrapMode::Word);
        textview.set_pixels_above_lines(2);
        textview.set_pixels_below_lines(2);
        textview.set_pixels_inside_wrap(1);
        textview.set_has_tooltip(true);

        let link_edit = Rc::new(LinkEdit::new(&b));

        let b: gtk::Box = builder_get!(b("container"));
        let top_level = b.upcast::<gtk::Widget>();

        let activate_link_cb: OpenLinkCb = Rc::new(RefCell::new(Box::new(|_: &str| {})));

        let link_start = buffer.create_mark(None, &buffer.get_start_iter(), true);
        let link_end = buffer.create_mark(None, &buffer.get_start_iter(), false);

        let this = Self {
            buffer,
            tags,
            textview,
            link_edit,
            top_level,
            activate_link_cb,
            is_editable: Rc::new(RefCell::from(true)),
            link_start,
            link_end,
        };
        this.top_level.add_controller(&this.get_key_press_handler_background());
        this.textview.add_controller(&this.get_key_press_handler());
        this.textview.add_controller(&this.get_mouse_release_handler());
        this.textview.add_controller(&this.get_drag_handler());
        this.textview.set_buffer(Some(&this.buffer));

        this.textview.connect_query_tooltip({
            |t, x, y, keyboard_mode, tooltip| t.tooltip(x, y, keyboard_mode, tooltip)
        });
        this.textview.connect_move_cursor({
            let tags = this.tags.clone();
            move |textview, _, _, _| tags.move_cursor(textview)
        });

        this.link_edit.set_accept_link_cb(connect_fwd1!(this.accept_link()));

        this.buffer
            .connect_local("insert-text", true, connect_fwd1!(this.buffer_do_insert_text()))
            .unwrap();

        this
    }

    pub fn get_widget(&self) -> &gtk::Widget {
        &self.top_level
    }

    pub fn get_modified(&self) -> bool {
        self.buffer.get_modified()
    }

    pub fn set_not_modified(&self) {
        self.buffer.set_modified(false)
    }

    pub fn grab_focus(&self) {
        self.textview.grab_focus();
    }

    pub fn set_activate_link_cb<F: Fn(&str) + 'static>(&self, activate_link_cb: F) {
        *self.activate_link_cb.borrow_mut() = Box::new(activate_link_cb);
    }

    fn set_editable(&self, editable: bool) {
        // ToDo: all formatting needs to be disabled, too
        *self.is_editable.borrow_mut() = editable;
        self.textview.set_editable(editable);
        if editable {
            self.grab_focus();
        }
    }

    fn is_editable(&self) -> bool {
        *self.is_editable.borrow().deref()
    }

    fn buffer_do_insert_text(&self, values: &[Value]) -> Option<Value> {
        let buffer = &values[0].get::<gtk::TextBuffer>().unwrap().unwrap();
        let iter = &values[1].get::<gtk::TextIter>().unwrap().unwrap();
        let _text = values[2].get::<&str>().unwrap().unwrap();
        let count = values[3].get::<i32>().unwrap().unwrap();

        let mut start = iter.clone();
        start.backward_chars(count);
        self.tags.for_each_edit_tag(|tag: &gtk::TextTag| buffer.apply_tag(tag, iter, &start));
        None
    }

    fn get_key_press_handler(&self) -> EventControllerKey {
        let controller = EventControllerKey::new();
        controller.connect_key_pressed({
            let this = self.clone();
            move |_controller: &EventControllerKey,
                  key: gdk::keys::Key,
                  _code: u32,
                  modifier: gdk::ModifierType| {
                if modifier == gdk::ModifierType::CONTROL_MASK {
                    match key {
                        keys::_0 => this.par_format(None),
                        keys::_1 => this.par_format(Some(ParFormat::H1)),
                        keys::_2 => this.par_format(Some(ParFormat::H2)),
                        keys::_3 => this.par_format(Some(ParFormat::H3)),
                        keys::_4 => this.par_format(Some(ParFormat::H4)),
                        keys::_5 => this.par_format(Some(ParFormat::H5)),
                        keys::_6 => this.par_format(Some(ParFormat::H6)),
                        keys::b => this.char_format(CharFormat::BOLD),
                        keys::d => this.char_format(CharFormat::STRIKE),
                        keys::i => this.char_format(CharFormat::ITALIC),
                        keys::l => this.edit_link(),
                        keys::n => this.apply_text_clear(),
                        keys::t => this.char_format(CharFormat::MONO),
                        keys::y => this.redo(),
                        keys::z => {
                            if (modifier & gdk::ModifierType::SHIFT_MASK).is_empty() {
                                this.undo();
                            } else {
                                this.redo();
                            }
                        }
                        _ => {
                            println!("Unmapped key {} mod {} code {}.", key, modifier, _code);
                            return Inhibit(false);
                        }
                    }
                    return Inhibit(true);
                } else if modifier.is_empty() {
                    match key {
                        keys::F1 => this.char_format(CharFormat::GREEN),
                        keys::F2 => this.char_format(CharFormat::RED),
                        keys::F3 => this.char_format(CharFormat::YELLOW),
                        keys::F4 => this.char_format(CharFormat::BLUE),
                        keys::F7 => this.dump(),
                        keys::F8 => this.turnaround(),
                        keys::KP_Enter | keys::Return => {
                            this.tags.text_edit(TextEdit::NewLine);
                            return Inhibit(false);
                        }
                        _ => return Inhibit(false),
                    }
                    return Inhibit(true);
                }
                Inhibit(false)
            }
        });
        controller
    }

    fn get_key_press_handler_background(&self) -> EventControllerKey {
        let controller = EventControllerKey::new();
        controller.connect_key_pressed({
            let this = self.clone();
            move |_controller: &EventControllerKey,
                  key: gdk::keys::Key,
                  _code: u32,
                  _modifier: gdk::ModifierType| {
                // just Enter/Return is swallowed by the entries, but this enables every modifier
                // to make them work
                match key {
                    keys::Escape => this.link_edit.reject(),
                    keys::KP_Enter | keys::Return => this.link_edit.accept(),
                    _ => return Inhibit(false),
                }
                Inhibit(true)
            }
        });
        controller
    }

    fn get_mouse_release_handler(&self) -> gtk::GestureClick {
        let gesture = gtk::GestureClick::new();
        gesture.connect_released({
            let this = self.clone();
            move |gesture, _n_press, x, y| {
                if this.buffer.get_has_selection()
                    || gesture.clone().upcast::<gtk::GestureSingle>().get_button() > 1
                {
                    return;
                }

                if let Some(link) = this.textview.get_link_at_location(x, y) {
                    (this.activate_link_cb.as_ref().borrow().deref())(link.as_str());
                }
            }
        });
        gesture
    }

    fn get_drag_handler(&self) -> gtk::DragSource {
        let drag = gtk::DragSource::new();
        drag.connect_prepare({
            let this = self.clone();
            move |drag_source: &gtk::DragSource, x, y| -> Option<gdk::ContentProvider> {
                if let Some(link) = this.textview.get_link_at_location(x, y) {
                    // a drag leaves a one char selection, this should be deleted
                    let cursor = this.buffer.get_iter_at_mark(&this.buffer.get_insert());
                    this.buffer.select_range(&cursor, &cursor);

                    let bytes = glib::Bytes::from(link.as_bytes());
                    // https://www.iana.org/assignments/media-types/text/uri-list
                    let content = gdk::ContentProvider::new_for_bytes("text/uri-list", &bytes);
                    return Some(content);
                }
                drag_source.drag_cancel();
                None
            }
        });
        drag
    }

    pub fn par_format(&self, format: Option<ParFormat>) {
        let mut start = self.buffer.get_iter_at_mark(&self.buffer.get_insert());
        start.set_line(start.get_line());
        let mut end = start.clone();
        // ToDo: this might be a problem for empty lines
        end.forward_to_line_end();

        self.buffer.apply_paragraph_format(format, &start, &end);
    }

    // Changing the format of the current selection/the current word/the cursor
    // Decide if the format is applied or cleared:
    // * for a selection the first character shall decide
    // * for a word the format at the cursor shall decide
    // * for the cursor, the tag manager knows the current format
    // The MONO format is used as long as the selection doesn't consist of complete lines
    // ToDo: The implementation is far from complete
    pub fn char_format(&self, format: CharFormat) {
        let tag_str = match format {
            CharFormat::BOLD => Tag::BOLD,
            CharFormat::ITALIC => Tag::ITALIC,
            CharFormat::MONO => Tag::MONO,
            CharFormat::STRIKE => Tag::STRIKE,
            CharFormat::RED => Tag::RED,
            CharFormat::GREEN => Tag::GREEN,
            CharFormat::BLUE => Tag::BLUE,
            CharFormat::YELLOW => Tag::YELLOW,
        };

        let b = &self.buffer;

        let toggle_tag = |start: &gtk::TextIter, end: &gtk::TextIter| {
            let tag = b.get_tag_table().lookup(tag_str).unwrap();
            b.begin_user_action();
            if start.has_tag(&tag) {
                b.remove_tag(&tag, &start, &end);
            } else {
                b.apply_tag(&tag, &start, &end);
            }
            b.end_user_action();
        };

        if let Some((start, mut end)) = b.get_selection_bounds() {
            if end.starts_line() {
                end.backward_char();
            }
            if format == CharFormat::MONO && start.starts_line() && end.ends_line() {
                b.apply_paragraph_format(Some(ParFormat::CODE), &start, &end);
            } else {
                toggle_tag(&start, &end);
            }
        } else if let Some((start, end)) = b.get_current_word_bounds() {
            toggle_tag(&start, &end);
        } else {
            self.tags.toggle_tag(tag_str);
        }
    }

    pub fn apply_text_clear(&self) {
        let clear = |start: &gtk::TextIter, end: &gtk::TextIter| {
            // Remove overlapping paragraph tags on the whole paragraph
            for line in start.get_line()..end.get_line() + 1 {
                if let Some(line_iter) = self.buffer.get_iter_at_line(line) {
                    for tag in line_iter.get_tags() {
                        if tag.is_par_format() {
                            let mut line_end = line_iter.clone();
                            line_end.forward_to_line_end();
                            line_end.forward_char();
                            self.buffer.remove_tag(&tag, &line_iter, &line_end);
                        }
                    }
                }
            }

            self.buffer.remove_all_tags(&start, &end);
        };

        if let Some((start, end)) = self.buffer.get_selection_bounds() {
            clear(&start, &end);
        } else if let Some((start, end)) = self.buffer.get_current_word_bounds() {
            clear(&start, &end);
        }
    }

    fn accept_link(&self, link_data: Option<&LinkData>) {
        self.set_editable(true);
        if let Some(data) = link_data {
            let buffer = &self.buffer;

            let mut start = buffer.get_iter_at_mark(&self.link_start);
            let mut end = buffer.get_iter_at_mark(&self.link_end);
            buffer.delete(&mut start, &mut end);
            buffer.insert(&mut end, &data.text);
            start = buffer.get_iter_at_mark(&self.link_start);

            let tag = if data.is_image {
                buffer.create_image_tag(&data.link)
            } else {
                buffer.create_link_tag(&data.link)
            };
            buffer.apply_tag(&tag, &start, &end);
        }
    }

    pub fn edit_link(&self) {
        if !self.is_editable() {
            return;
        }

        let mut start = self.buffer.get_iter_at_mark(&self.buffer.get_insert());
        let mut end = start.clone();
        let mut link = String::new();
        let mut is_image = false;
        if let Some((l, tag)) = self.buffer.get_link_at_iter(&start) {
            link = l;
            if !start.starts_tag(Some(&tag)) {
                start.backward_to_tag_toggle(Some(&tag));
            }
            if !end.ends_tag(Some(&tag)) {
                end.forward_to_tag_toggle(Some(&tag));
            }
        } else if let Some((l, tag)) = self.buffer.get_image_at_iter(&start) {
            link = l;
            if !start.starts_tag(Some(&tag)) {
                start.backward_to_tag_toggle(Some(&tag));
            }
            if !end.ends_tag(Some(&tag)) {
                end.forward_to_tag_toggle(Some(&tag));
            }
            is_image = true;
        } else if let Some((s, e)) = self.buffer.get_selection_bounds() {
            start = s;
            end = e;
        } else {
            // select current non-whitespace clock, to capture complete links
            while start.backward_char() {
                if start.get_char().is_whitespace() {
                    start.forward_char();
                    break;
                }
            }
            if !end.get_char().is_whitespace() {
                while end.forward_char() {
                    if end.get_char().is_whitespace() {
                        break;
                    }
                }
            }
        }

        self.buffer.move_mark(&self.link_start, &start);
        self.buffer.move_mark(&self.link_end, &end);
        let text = String::from(self.buffer.get_text(&start, &end, false).as_str());

        let old_link = LinkData { text, link, is_image };
        self.link_edit.edit_link(&old_link);
        self.set_editable(false);
    }

    pub fn undo(&self) {
        self.buffer.undo();
    }

    pub fn redo(&self) {
        self.buffer.redo();
    }

    pub fn to_markdown(&self) -> String {
        self.buffer.to_markdown()
    }

    pub fn clear(&self) {
        self.buffer.delete(&mut self.buffer.get_start_iter(), &mut self.buffer.get_end_iter());
    }

    pub fn insert_markdown(&self, markdown: &str, clear: bool) {
        self.buffer.begin_irreversible_action();
        if clear {
            self.clear();
        }
        self.buffer.insert_markdown(&mut self.buffer.get_insert_iter(), markdown);
        self.buffer.end_irreversible_action();
    }

    pub fn new_content_markdown(&self, markdown: &str) {
        self.buffer.begin_irreversible_action();
        self.buffer.assign_markdown(markdown, false);
        self.buffer.end_irreversible_action();
    }

    fn dump(&self) {
        println!("Dump! begin -------------------------------------------------------------------");
        println!("{}", self.to_markdown().as_str());
        println!("Dump! end   -------------------------------------------------------------------");
    }

    fn turnaround(&self) {
        self.buffer.begin_irreversible_action();
        let markdown = self.to_markdown();
        self.buffer.assign_markdown(&markdown, true);
        self.buffer.end_irreversible_action();
    }
}
