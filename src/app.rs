use std::rc::Rc;

use gtk::prelude::*;

use crate::data::Data;
use crate::mainwindow::MainWindow;
use crate::res::APP_ID;
use crate::settings::Settings;

pub struct App {
    app: gtk::Application,
}

impl App {
    pub fn new() -> Self {
        let app = gtk::Application::new(
            Some(APP_ID),
            gtk::gio::ApplicationFlags::HANDLES_OPEN | gtk::gio::ApplicationFlags::NON_UNIQUE,
        )
        .expect("Initialization failed...");

        app.connect_activate(|app| {
            let w = App::create_window(app);
            w.prepare_show();
            w.show();
        });

        app.connect_open(App::open);
        Self { app }
    }

    fn open(app: &gtk::Application, files: &[gtk::gio::File], _hint: &str) {
        let window = App::create_window(app);
        // ToDo: handle multiple files
        window.enqueue_file(files[0].get_path().unwrap());
        window.prepare_show();
        window.show();
    }

    fn create_window(app: &gtk::Application) -> MainWindow {
        let data = Rc::new(Data::new());
        let settings = Rc::new(Settings::new());
        MainWindow::new(&app, &data, &settings)
    }

    pub fn run(&self, argv: &[String]) -> i32 {
        self.app.run(argv)
    }
}
