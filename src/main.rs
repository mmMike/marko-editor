mod app;
mod data;
mod gdk_glue;
mod gdk_x11_glue;
mod gtk_macros;
mod mainwindow;
mod res;
mod settings;
mod textbufferext;
mod textbuffermd;
mod texttag;
mod texttagmanager;
mod texttagtable;
mod textview;
mod textviewext;

#[macro_use]
extern crate lazy_static;

use crate::app::App;

use std::env::args;

fn main() {
    let app = App::new();
    app.run(&args().collect::<Vec<_>>());
}
