use gtk::prelude::StyleContextExt;

pub trait Serialize<T> {
    fn deserialize(data: &str) -> Option<T>;
    fn serialize(&self) -> String;
}

impl Serialize<gdk::Rectangle> for gdk::Rectangle {
    fn deserialize(data: &str) -> Option<gdk::Rectangle> {
        let mut a = data.split('_');
        if a.next()?.ne("rect") {
            return None;
        }
        Some(Self {
            x: a.next()?.parse::<i32>().ok()?,
            y: a.next()?.parse::<i32>().ok()?,
            width: a.next()?.parse::<i32>().ok()?,
            height: a.next()?.parse::<i32>().ok()?,
        })
    }

    fn serialize(&self) -> String {
        format!(
            "rect_{}_{}_{}_{}",
            self.x.to_string(),
            self.y.to_string(),
            self.width.to_string(),
            self.height.to_string()
        )
    }
}

pub trait GetColor {
    fn get_color(&self, is_background: bool, flags: gtk::StateFlags) -> Option<gdk::RGBA>;
}

impl GetColor for gtk::StyleContext {
    fn get_color(&self, is_background: bool, flags: gtk::StateFlags) -> Option<gdk::RGBA> {
        let ctx = self.clone();
        ctx.set_state(flags);
        let mut s = gtk::cairo::ImageSurface::create(gtk::cairo::Format::ARgb32, 2, 2).unwrap();
        let c = gtk::cairo::Context::new(&s).ok()?;
        if is_background {
            gtk::render_background(&ctx, &c, 0f64, 0f64, 1f64, 1f64);
        } else {
            gtk::render_line(&ctx, &c, 0f64, 0f64, 1f64, 1f64);
        }
        drop(c);

        let data = s.data().unwrap();
        let slice = &data[0..4];

        let transform = |input: f32| -> f32 {
            if slice[3] == 0 {
                0.
            } else {
                (input / (slice[3] as f32) * 255f32).floor() / 255f32
            }
        };

        Some(gdk::RGBA {
            red: transform(slice[2] as f32),
            green: transform(slice[1] as f32),
            blue: transform(slice[0] as f32),
            alpha: 1f32,
        })
    }
}

pub trait ColorCreator {
    fn brighter(&self, factor: f32) -> gdk::RGBA;
}

impl ColorCreator for gdk::RGBA {
    fn brighter(&self, factor: f32) -> gdk::RGBA {
        let (h, mut s, mut v) = gtk::rgb_to_hsv(self.red, self.green, self.blue);
        v *= factor / 100f32;
        if v > 1f32 {
            s -= v - 1f32;
            if s < 0f32 {
                s = 0f32;
            }
            v = 1f32;
        }

        let (red, green, blue) = gtk::hsv_to_rgb(h, s, v);
        gdk::RGBA { red, green, blue, alpha: self.alpha }
    }
}
