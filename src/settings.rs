extern crate configparser;
use configparser::ini::Ini;

use anyhow::{anyhow, Result};

use crate::gdk_glue::Serialize;
use crate::gdk_x11_glue::WindowGeometry;
use crate::res::APP_NAME;
use gtk::glib;
use gtk::glib::IsA;
use std::cell::{Ref, RefCell};
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

const BOOKMARKS: &str = "bookmarks";

pub struct Settings {
    config: RefCell<Ini>,
    settings_file: PathBuf,
}

impl Settings {
    pub fn new() -> Self {
        let mut settings_file = glib::get_user_config_dir();
        settings_file.push(APP_NAME);
        fs::create_dir_all(settings_file.as_path()).unwrap_or_else(|why| {
            println!("! {:?}", why.kind());
        });
        settings_file.push("settings.ini");

        let config = RefCell::new(Ini::new());
        if let Err(err) = config.borrow_mut().load(settings_file.as_path().to_str().unwrap()) {
            println!("Error while reading settings: {}", err)
        }

        Self { config, settings_file }
    }

    pub fn get(&self, section: &str, key: &str) -> Option<String> {
        self.config.borrow().get(section, key)
    }
    #[allow(dead_code)]
    pub fn get_or(&self, section: &str, key: &str, default_value: &str) -> String {
        match self.config.borrow().get(section, key) {
            None => String::from(default_value),
            Some(v) => v,
        }
    }

    pub fn set(&self, section: &str, key: &str, value: &str) {
        self.config.borrow_mut().set(section, key, Some(value.parse().unwrap()));
    }

    pub fn write(&self) -> Result<()> {
        self.write_internal(&self.config.borrow())
    }

    fn write_internal(&self, config: &Ref<Ini>) -> Result<()> {
        Ok(config.write(self.settings_file.as_path().to_str().unwrap())?)
    }

    pub fn store_geometry<W: IsA<gtk::Window> + IsA<gtk::Native>>(&self, window: &W, key: &str) {
        if let Some(rect) = window.get_window_geometry() {
            if let Some(screen) = window.get_window_screen() {
                self.set(key, screen.serialize().as_str(), rect.serialize().as_str());
                self.write().unwrap();
            }
        }
    }

    pub fn restore_geometry<W: IsA<gtk::Window> + IsA<gtk::Native>>(&self, window: &W, key: &str) {
        if let Some(screen) = window.get_window_screen() {
            match self.get(key, screen.serialize().as_str()) {
                None => {}
                Some(data) => {
                    if let Some(rect) = gdk::Rectangle::deserialize(&*data) {
                        if let Some(current) = window.get_window_geometry() {
                            if current != rect {
                                window.set_window_geometry(&rect);
                            }
                        }
                    }
                }
            }
        }
    }

    pub fn add_bookmark(&self, link: &str) -> Result<()> {
        {
            let config = self.config.borrow();
            let map = config.get_map_ref();
            if let Some(bookmarks) = map.get(BOOKMARKS) {
                for (_, i) in bookmarks {
                    if let Some(value) = i {
                        if value == link {
                            return Err(anyhow!("Bookmark already set for: {}", link));
                        }
                    }
                }
            }
        }

        if let Ok(now) = SystemTime::now().duration_since(UNIX_EPOCH) {
            let key = format!("{}", now.as_millis());
            self.set(BOOKMARKS, key.as_str(), link);
            self.write()
        } else {
            Err(anyhow!("Could not calculate key for bookmark."))
        }
    }

    pub fn remove_bookmark(&self, link: &str) -> Result<()> {
        let mut key: Option<String> = None;
        let mut config = self.config.borrow_mut();
        if let Some(bookmarks) = config.get_mut_map().get_mut(BOOKMARKS) {
            for (k, i) in bookmarks.iter() {
                if let Some(value) = i {
                    if value == link {
                        key = Some(k.to_string());
                        break;
                    }
                }
            }
            if let Some(k) = key {
                bookmarks.remove(&k);
                return Ok(config.write(self.settings_file.as_path().to_str().unwrap())?);
            }
        }
        Ok(())
    }

    pub fn get_bookmarks(&self) -> Vec<String> {
        let mut res: Vec<String> = Vec::new();
        if let Some(bookmarks) = self.config.borrow().get_map_ref().get(BOOKMARKS) {
            for (_, i) in bookmarks {
                if let Some(value) = i {
                    res.push(value.to_string());
                }
            }
        }
        res
    }
}
