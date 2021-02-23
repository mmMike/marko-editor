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
