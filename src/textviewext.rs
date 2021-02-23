use crate::textbufferext::TextBufferExt2;
use gtk::TextViewExt;

pub trait TextViewExt2 {
    fn get_link_at_location(&self, x: f64, y: f64) -> Option<String>;

    fn tooltip(&self, x: i32, y: i32, _keyboard_mode: bool, tooltip: &gtk::Tooltip) -> bool;
}

impl TextViewExt2 for gtk::TextView {
    fn get_link_at_location(&self, x: f64, y: f64) -> Option<String> {
        let (bx, by) = self.window_to_buffer_coords(gtk::TextWindowType::Text, x as i32, y as i32);
        if let Some(iter) = self.get_iter_at_location(bx, by) {
            if let Some((name, _tag)) = self.get_buffer().get_link_at_iter(&iter) {
                return Some(name);
            }
        }
        None
    }

    fn tooltip(&self, x: i32, y: i32, _keyboard_mode: bool, tooltip: &gtk::Tooltip) -> bool {
        if let Some(link) = self.get_link_at_location(x as f64, y as f64) {
            tooltip.set_text(Some(link.as_str()));
            true
        } else {
            false
        }
    }
}
