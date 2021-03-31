extern crate x11;
use gtk::gio::SimpleAction;
use gtk::glib;
use gtk::prelude::*;
use gtk::EventControllerKey;
use gtk::WidgetExt;
use gtk::{FileChooserAction, FileChooserDialog, ResponseType};

use crate::data::Data;
use crate::gdk_x11_glue::WindowGeometry;
use crate::res::APP_NAME;
use crate::settings::Settings;
use crate::texttag::{CharFormat, ParFormat};
use crate::textview::TextView;
use crate::{builder_get, connect, connect_action_plain};

use std::cell::RefCell;
use std::fs;
use std::fs::File;
use std::io::{BufReader, Read};
use std::ops::Deref;
use std::path::{Path, PathBuf};
use std::rc::Rc;

const CSS: &str = r#"
textview {
    font-size: 12pt;
}
menubutton {
  font-weight: bold;
}
"#;

struct Ui {
    window: gtk::ApplicationWindow,
    text_view_container: gtk::Box,
    text_view: TextView,
    btn_save: gtk::Button,
    btn_clear: gtk::Button,
    btn_bold: gtk::Button,
    btn_italic: gtk::Button,
    btn_code: gtk::Button,
    btn_strike: gtk::Button,
    btn_link: gtk::Button,
    btn_undo: gtk::Button,
    btn_redo: gtk::Button,
    btn_search: gtk::Button,
    btn_open_menu: gtk::MenuButton,
    outline_widget: gtk::Box,
    outline_view: gtk::TreeView,
    outline_splitter: gtk::Paned,
    outline_maxlevel: gtk::ComboBox,
    btn_outline_top: gtk::Button,
    btn_outline_bottom: gtk::Button,
    dlg_md: gtk::Dialog,
}

#[derive(Clone)]
pub struct MainWindow {
    settings: Rc<Settings>,
    data: Rc<Data>,
    ui: Rc<Ui>,
    css: gtk::CssProvider,
    file: Rc<RefCell<Option<PathBuf>>>,
}

impl MainWindow {
    pub fn new(app: &gtk::Application, data: &Rc<Data>, settings: &Rc<Settings>) -> Self {
        let ui_src = include_str!("mainwindow.ui");
        let b = gtk::Builder::new();
        b.add_from_string(ui_src).expect("Couldn't add from string");

        let ui = Rc::new(Ui {
            window: builder_get!(b("window")),
            text_view_container: builder_get!(b("text_view_container")),
            text_view: TextView::new(),
            btn_save: builder_get!(b("btn_save")),
            btn_clear: builder_get!(b("btn_clear")),
            btn_bold: builder_get!(b("btn_bold")),
            btn_italic: builder_get!(b("btn_italic")),
            btn_code: builder_get!(b("btn_code")),
            btn_strike: builder_get!(b("btn_strike")),
            btn_link: builder_get!(b("btn_link")),
            btn_undo: builder_get!(b("btn_undo")),
            btn_redo: builder_get!(b("btn_redo")),
            btn_search: builder_get!(b("btn_search")),
            btn_open_menu: builder_get!(b("btn_open_menu")),
            outline_widget: builder_get!(b("outline_widget")),
            outline_view: builder_get!(b("outline_view")),
            outline_splitter: builder_get!(b("outline_splitter")),
            outline_maxlevel: builder_get!(b("outline_maxlevel")),
            btn_outline_top: builder_get!(b("btn_outline_top")),
            btn_outline_bottom: builder_get!(b("btn_outline_bottom")),
            dlg_md: builder_get!(b("dlg_md")),
        });
        ui.text_view_container.append(ui.text_view.get_widget());

        let css = gtk::CssProvider::new();

        let this = Self {
            settings: settings.clone(),
            data: data.clone(),
            ui,
            css,
            file: Rc::new(RefCell::new(None)),
        };

        this.ui.text_view.set_activate_link_cb({
            let w = this.ui.window.clone();
            move |link| {
                gtk::show_uri(Some(&w), link, gdk::CURRENT_TIME);
            }
        });

        this.ui.btn_save.connect_clicked(connect!(this.btn_save_clicked()));

        let t = &this.ui.text_view;
        this.ui.btn_clear.connect_clicked(connect!(t.apply_text_clear()));
        this.ui.btn_bold.connect_clicked(connect!(t.char_format(CharFormat::BOLD)));
        this.ui.btn_italic.connect_clicked(connect!(t.char_format(CharFormat::ITALIC)));
        this.ui.btn_code.connect_clicked(connect!(t.char_format(CharFormat::MONO)));
        this.ui.btn_strike.connect_clicked(connect!(t.char_format(CharFormat::STRIKE)));
        this.ui.btn_link.connect_clicked(connect!(t.edit_link()));
        this.ui.btn_undo.connect_clicked(connect!(t.undo()));
        this.ui.btn_redo.connect_clicked(connect!(t.redo()));
        this.ui.btn_search.connect_clicked(connect!(t.open_search()));
        this.ui.btn_outline_top.connect_clicked(connect!(t.scroll_to_top_bottom(true)));
        this.ui.btn_outline_bottom.connect_clicked(connect!(t.scroll_to_top_bottom(false)));

        this.ui.outline_maxlevel.connect_changed(connect!(this.update_outline()));
        this.ui.outline_view.connect_row_activated({
            let t = this.ui.text_view.clone();
            move |s, path, _col| {
                let model = s.get_model().unwrap();
                if let Some(iter) = model.get_iter(path) {
                    let line = model.get_value(&iter, 1).get::<i32>().unwrap().unwrap();
                    t.scroll_to(line);
                }
            }
        });

        this.ui.window.set_application(Some(app));
        this.ui.window.add_controller(&this.get_window_key_press_handler());
        this.ui.window.connect_close_request(connect!(this.close_response()));
        this.set_title();

        this.setup_action("new", connect_action_plain!(this.btn_new_clicked()));
        this.setup_action("home", connect_action_plain!(this.act_open_startpage()));
        this.setup_action("open", connect_action_plain!(this.btn_open_clicked()));

        this.setup_action("header_1", connect_action_plain!(t.par_format(Some(ParFormat::H1))));
        this.setup_action("header_2", connect_action_plain!(t.par_format(Some(ParFormat::H2))));
        this.setup_action("header_3", connect_action_plain!(t.par_format(Some(ParFormat::H3))));
        this.setup_action("header_4", connect_action_plain!(t.par_format(Some(ParFormat::H4))));
        this.setup_action("header_5", connect_action_plain!(t.par_format(Some(ParFormat::H5))));
        this.setup_action("header_6", connect_action_plain!(t.par_format(Some(ParFormat::H6))));

        this.setup_action("green", connect_action_plain!(t.char_format(CharFormat::GREEN)));
        this.setup_action("red", connect_action_plain!(t.char_format(CharFormat::RED)));
        this.setup_action("yellow", connect_action_plain!(t.char_format(CharFormat::YELLOW)));
        this.setup_action("blue", connect_action_plain!(t.char_format(CharFormat::BLUE)));

        this.setup_action("clear_startpage", connect_action_plain!(this.act_clear_startpage()));
        this.setup_action("set_startpage", connect_action_plain!(this.act_set_startpage()));
        this.setup_action("add_bookmark", connect_action_plain!(this.act_add_bookmark()));
        this.setup_action("remove_bookmark", connect_action_plain!(this.act_remove_bookmark()));
        this.setup_action("inspector", connect_action_plain!(this.act_inspector()));
        this.setup_action("quit", connect_action_plain!(this.close()));
        this.setup_action("save_as", connect_action_plain!(this.act_save_as()));
        this.setup_action("store_geometry", connect_action_plain!(this.store_geometry()));

        this.update_menu();

        this.setup_md_dialog(&b);
        this.setup_action("markdown", connect_action_plain!(this.act_markdown_dlg()));

        this
    }

    pub fn prepare_show(&self) {
        self.ui.window.realize();
        self.restore_geometry();

        let css = gtk::CssProvider::new();
        css.load_from_data(CSS.as_ref());

        fn apply_css<P: IsA<gtk::StyleProvider>, W: IsA<gtk::Widget>>(
            widget: &W,
            provider: &P,
            priority: u32,
        ) {
            widget.get_style_context().add_provider(provider, priority);

            let mut child = widget.get_first_child();
            while let Some(c) = &child {
                apply_css(c, provider, priority);
                child = c.get_next_sibling();
            }
        }
        apply_css(&self.ui.window, &css, u32::max_value());

        // the combobox should look like a button, since it contains one we style it like the others
        fn css_combo_to_flat<W: IsA<gtk::Widget>>(widget: &W) {
            if widget.get_css_classes().contains(&glib::GString::from("combo")) {
                widget.remove_css_class("combo");
                widget.add_css_class("flat");
            }

            let mut child = widget.get_first_child();
            while let Some(c) = &child {
                css_combo_to_flat(c);
                child = c.get_next_sibling();
            }
        }
        css_combo_to_flat(&self.ui.outline_widget);
    }

    pub fn show(&self) {
        self.ui.window.show();

        self.ui.text_view.grab_focus();

        let p = self.file.borrow().clone();
        if let Some(filename) = p {
            self.open_file(&filename);
        } else {
            self.act_open_startpage();
        }

        // ToDo: still not working 100% reliably...
        // we call restore_geometry again, since sometimes it doesn't work before show...
        self.restore_geometry();
    }

    pub fn enqueue_file(&self, filename: PathBuf) {
        self.set_filename(&filename);
    }

    fn open_file(&self, filename: &PathBuf) {
        self.close_file(Rc::new({
            let f = filename.clone();
            move |s: &MainWindow| {
                if let Ok(file) = File::open(f.as_path()) {
                    let mut reader = BufReader::new(file);
                    let mut contents = String::new();
                    if reader.read_to_string(&mut contents).is_ok() {
                        s.ui.text_view.new_content_markdown(&contents);
                        s.set_filename(&f);
                        s.update_outline();
                    }
                } else {
                    let dlg = gtk::MessageDialog::new(
                        Some(&s.ui.window),
                        gtk::DialogFlags::MODAL
                            | gtk::DialogFlags::DESTROY_WITH_PARENT
                            | gtk::DialogFlags::USE_HEADER_BAR,
                        gtk::MessageType::Warning,
                        gtk::ButtonsType::Ok,
                        format!("Could not open file: {}", f.to_str().unwrap()).as_str(),
                    );
                    dlg.connect_response(|d, _| d.hide());
                    dlg.show();
                }
            }
        }));
    }

    fn set_title(&self) {
        if let Some(filename) = self.file.borrow().deref() {
            self.ui
                .window
                .set_title(Some(format!("{} - {}", APP_NAME, filename.to_str().unwrap()).as_str()));
        } else {
            self.ui.window.set_title(Some(APP_NAME));
        }
    }

    fn btn_open_clicked(&self) {
        let dlg = FileChooserDialog::new(
            Some("Open File"),
            Some(&self.ui.window),
            FileChooserAction::Open,
            &[("Open", ResponseType::Ok), ("Cancel", ResponseType::Cancel)],
        );
        let md_filter = gtk::FileFilter::new();
        md_filter.set_name(Some("Markdown (*.md, *.markdown)"));
        md_filter.add_pattern("*.md");
        md_filter.add_pattern("*.markdown");
        dlg.add_filter(&md_filter);
        let all_filter = gtk::FileFilter::new();
        all_filter.set_name(Some("All files (*.*)"));
        all_filter.add_pattern("*.*");
        dlg.add_filter(&all_filter);

        dlg.connect_response({
            let s = self.clone();
            move |dlg: &FileChooserDialog, response: ResponseType| {
                s.settings.store_geometry(dlg, "file_dlg_geometry");
                if response == ResponseType::Ok {
                    let file = dlg.get_file().expect("Couldn't get file");
                    s.open_file(&file.get_path().expect("Couldn't get file path"));
                }
                dlg.close();
            }
        });
        dlg.realize();
        self.settings.restore_geometry(&dlg, "file_dlg_geometry");
        dlg.show();
    }

    fn btn_save_clicked(&self) {
        self.save_file(Rc::new(|_: &MainWindow| {}));
    }

    fn save_file<F: Fn(&Self) + 'static>(&self, and_then: Rc<F>) {
        // and_then might need the file.borrow...
        let p = self.file.borrow().clone();
        if let Some(filename) = p {
            if self.write_file(&filename).is_ok() {
                and_then(self);
            }
        } else {
            self.save_file_as(and_then);
        }
    }

    fn save_file_as<F: Fn(&Self) + 'static>(&self, and_then: Rc<F>) {
        let dlg = FileChooserDialog::new(
            Some("Save File As"),
            Some(&self.ui.window),
            FileChooserAction::Save,
            &[("Save", ResponseType::Ok), ("Cancel", ResponseType::Cancel)],
        );
        let md_filter = gtk::FileFilter::new();
        md_filter.set_name(Some("Markdown (*.md, *.markdown)"));
        md_filter.add_pattern("*.md");
        md_filter.add_pattern("*.markdown");
        dlg.add_filter(&md_filter);
        let all_filter = gtk::FileFilter::new();
        all_filter.set_name(Some("All files (*.*)"));
        all_filter.add_pattern("*.*");
        dlg.add_filter(&all_filter);

        dlg.connect_response({
            let s = self.clone();
            move |dlg: &FileChooserDialog, response: ResponseType| {
                s.settings.store_geometry(dlg, "file_dlg_geometry");
                if response == ResponseType::Ok {
                    if let Some(file) = dlg.get_file() {
                        let filename = file.get_path().expect("Couldn't get file path");
                        if s.write_file(&filename).is_ok() {
                            s.set_filename(&filename);
                            and_then(&s);
                        }
                    }
                }
                dlg.close();
            }
        });
        dlg.realize();
        self.settings.restore_geometry(&dlg, "file_dlg_geometry");
        dlg.show();
    }

    fn act_save_as(&self) {
        self.save_file_as(Rc::new(|_: &MainWindow| {}));
    }

    fn write_file(&self, filename: &Path) -> std::io::Result<()> {
        let res = fs::write(filename, self.ui.text_view.to_markdown());
        if res.is_ok() {
            self.ui.text_view.set_not_modified();
        }
        res
    }

    fn close_file<F: Fn(&Self) + 'static>(&self, and_then: Rc<F>) {
        if !self.ui.text_view.get_modified() {
            and_then(self);
            return;
        }

        let dlg = gtk::MessageDialog::new(
            Some(&self.ui.window),
            gtk::DialogFlags::MODAL
                | gtk::DialogFlags::DESTROY_WITH_PARENT
                | gtk::DialogFlags::USE_HEADER_BAR,
            gtk::MessageType::Question,
            gtk::ButtonsType::None,
            "Save current document?",
        );
        let b = dlg.add_button("Discard", gtk::ResponseType::No).downcast::<gtk::Button>().unwrap();
        b.set_icon_name("user-trash-symbolic");
        b.set_css_classes(vec!["destructive-action"].as_ref());
        dlg.add_button("Cancel", gtk::ResponseType::Cancel);
        let b = dlg.add_button("Save", gtk::ResponseType::Yes).downcast::<gtk::Button>().unwrap();
        b.set_css_classes(vec!["suggested-action"].as_ref());
        b.grab_focus();

        dlg.connect_response({
            let s = self.clone();
            move |dlg, r| {
                match r {
                    ResponseType::Cancel => {}
                    ResponseType::Yes => s.save_file(and_then.to_owned()),
                    ResponseType::No => and_then(&s),
                    _ => {}
                }
                dlg.close();
            }
        });
        dlg.show();
    }

    fn set_filename(&self, filename: &PathBuf) {
        self.file.replace(Some(filename.clone()));
        self.set_title();
    }

    fn clear_file(&self) {
        self.ui.text_view.clear();
        self.file.replace(None);
        self.set_title();
        self.ui.text_view.set_not_modified();
    }

    fn btn_new_clicked(&self) {
        self.close_file(Rc::new(|s: &MainWindow| s.clear_file()));
    }

    fn act_inspector(&self) {
        gtk::Window::set_interactive_debugging(true);
    }

    fn get_window_key_press_handler(&self) -> EventControllerKey {
        let window_controller = EventControllerKey::new();
        window_controller.connect_key_pressed({
            let this = self.clone();
            move |_controller: &EventControllerKey,
                  key: gdk::keys::Key,
                  _code: u32,
                  modifier: gdk::ModifierType| {
                if !(modifier & gdk::ModifierType::CONTROL_MASK).is_empty() {
                    match key {
                        gdk::keys::constants::q => {
                            this.ui.window.close();
                            return glib::signal::Inhibit(true);
                        }
                        gdk::keys::constants::e => println!("{}", this.ui.text_view.to_markdown()),
                        gdk::keys::constants::m => this.act_markdown_dlg(),
                        gdk::keys::constants::o => this.toggle_outline(),
                        gdk::keys::constants::p => this.toggle_dark_theme(),
                        gdk::keys::constants::s => this.btn_save_clicked(),
                        _ => {}
                    }
                }
                glib::signal::Inhibit(false)
            }
        });
        window_controller
    }

    fn store_geometry(&self) {
        self.settings.store_geometry(&self.ui.window, "geometry");
        self.settings.store_geometry_property(
            &self.ui.window,
            "outline_splitter",
            self.ui.outline_splitter.get_position().to_string().as_str(),
        );
        self.settings.store_geometry_property(
            &self.ui.window,
            "outline_visible",
            self.ui.outline_widget.get_visible().to_string().as_str(),
        );
        let level = self.ui.outline_maxlevel.get_active().unwrap().to_string();
        let _ = self.settings.store("config", "outline_maxlevel", level.as_str());
    }

    fn restore_geometry(&self) {
        self.settings.restore_geometry(&self.ui.window, "geometry");
        if let Some(string) =
            self.settings.read_geometry_property(&self.ui.window, "outline_visible")
        {
            if let Ok(visible) = string.parse::<bool>() {
                self.ui.outline_widget.set_visible(visible);
            }
        }
        if let Some(string) =
            self.settings.read_geometry_property(&self.ui.window, "outline_splitter")
        {
            if let Ok(pos) = string.parse::<i32>() {
                self.ui.outline_splitter.set_position(pos);
            }
        }
        if let Some(string) = self.settings.get("config", "outline_maxlevel") {
            if let Ok(level) = string.parse::<u32>() {
                self.ui.outline_maxlevel.set_active(Some(level));
            }
        }
    }

    fn close(&self) {
        self.close_file(Rc::new(|s: &MainWindow| s.ui.window.get_application().unwrap().quit()));
    }

    fn close_response(&self) -> gtk::glib::signal::Inhibit {
        self.close();
        gtk::glib::signal::Inhibit(true)
    }

    fn setup_action<F: Fn(&SimpleAction, Option<&glib::Variant>) + 'static>(&self, id: &str, f: F) {
        let a = SimpleAction::new(id, None);
        a.connect_activate(f);
        self.ui.window.add_action(&a);
    }

    fn setup_md_dialog(&self, b: &gtk::Builder) {
        self.ui.dlg_md.set_hide_on_close(true);
        self.ui.dlg_md.get_style_context().add_provider(&self.css, u32::max_value());

        let textview_md: gtk::TextView = builder_get!(b("textview_md"));

        let btn_dlg_md_close: gtk::Button = builder_get!(b("btn_dlg_md_close"));
        btn_dlg_md_close.connect_clicked(connect!(self.ui.dlg_md.hide()));

        let btn_dlg_md_insert_markdown: gtk::Button = builder_get!(b("btn_dlg_md_insert_markdown"));
        btn_dlg_md_insert_markdown.connect_clicked({
            let s = self.clone();
            let t = textview_md.clone();
            move |_| {
                let buffer = t.get_buffer();
                let text = buffer.get_text(&buffer.get_start_iter(), &buffer.get_end_iter(), false);
                s.ui.text_view.insert_markdown(text.as_str(), false);
            }
        });

        let btn_dlg_md_replace_markdown: gtk::Button =
            builder_get!(b("btn_dlg_md_replace_markdown"));
        btn_dlg_md_replace_markdown.connect_clicked({
            let d = self.ui.dlg_md.clone();
            let s = self.clone();
            let t = textview_md.clone();
            move |_| {
                let buffer = t.get_buffer();
                let text = buffer.get_text(&buffer.get_start_iter(), &buffer.get_end_iter(), false);
                s.ui.text_view.insert_markdown(text.as_str(), true);
                d.hide();
            }
        });

        let btn_dlg_md_load_markdown: gtk::Button = builder_get!(b("btn_dlg_md_load_markdown"));
        btn_dlg_md_load_markdown.connect_clicked({
            let s = self.clone();
            let t = textview_md;
            move |_| {
                let text = s.ui.text_view.to_markdown();
                let buffer = t.get_buffer();
                buffer.get_text(&buffer.get_start_iter(), &buffer.get_end_iter(), false);
                buffer.delete(&mut buffer.get_start_iter(), &mut buffer.get_end_iter());
                buffer.insert(&mut buffer.get_start_iter(), text.as_str());
            }
        });
    }

    fn act_markdown_dlg(&self) {
        if let Some(geometry) = self.ui.window.get_window_geometry() {
            self.ui.dlg_md.set_property_default_height(geometry.height - 80);
            self.ui.dlg_md.set_property_default_width(geometry.width - 60);
        }
        self.ui.dlg_md.show();
    }

    fn act_set_startpage(&self) {
        if let Some(filename) = self.file.borrow().deref() {
            self.settings.set("config", "startpage", filename.to_str().unwrap());
            self.settings.write().unwrap();
        }
    }

    fn act_clear_startpage(&self) {
        self.settings.set("config", "startpage", "");
        self.settings.write().unwrap();
    }

    fn act_open_startpage(&self) {
        if let Some(startpage) = self.settings.get("config", "startpage") {
            if !startpage.is_empty() {
                let path = PathBuf::from(startpage);
                self.open_file(&path);
            }
        }
    }

    fn act_add_bookmark(&self) {
        if let Some(filename) = self.file.borrow().deref() {
            let _ = self.settings.add_bookmark(filename.to_str().unwrap());
        }
        self.update_menu();
    }

    fn act_remove_bookmark(&self) {
        if let Some(filename) = self.file.borrow().deref() {
            let _ = self.settings.remove_bookmark(filename.to_str().unwrap());
        }
        self.update_menu();
    }

    fn update_menu(&self) {
        let mut i = 0;
        while self.ui.window.has_action(format!("{}", i).as_str()) {
            self.ui.window.remove_action(format!("{}", i).as_str());
            i += 1;
        }

        let menu_model = self.ui.btn_open_menu.get_menu_model().unwrap();
        if let Ok(menu) = menu_model.downcast::<gtk::gio::Menu>() {
            menu.remove(2);
            let bookmarks = gtk::gio::Menu::new();
            for (i, item) in self.settings.get_bookmarks().iter().enumerate() {
                let path = PathBuf::from(item);
                self.setup_action(
                    format!("{}", i).as_str(),
                    connect_action_plain!(self.open_file(&path)),
                );
                bookmarks.append(Some(item.as_str()), Some(format!("win.{}", i).as_str()));
            }
            menu.append_section(None, &bookmarks);
        }
    }

    fn toggle_outline(&self) {
        self.update_outline();
        self.ui.outline_widget.set_visible(!self.ui.outline_widget.get_visible());
    }

    fn update_outline(&self) {
        let level = self.ui.outline_maxlevel.get_active().unwrap() + 1;
        self.ui.outline_view.set_model(Some(&self.ui.text_view.get_outline_model(level)));
    }

    fn toggle_dark_theme(&self) {
        if let Some(settings) = gtk::Settings::get_default() {
            settings.set_property_gtk_theme_name(Some("Adwaita"));

            let current = settings.get_property_gtk_application_prefer_dark_theme();
            settings.set_property_gtk_application_prefer_dark_theme(!current);
            self.ui.text_view.update_colors(!current);
            self.update_outline();
        }
    }
}
