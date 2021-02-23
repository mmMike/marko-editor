use crate::texttagtable::TextTagTable;
use std::cell::RefCell;
use std::collections::BTreeSet;
use std::ops::Deref;

#[derive(Debug)]
pub enum TextEdit {
    NewLine,
}

// No clone, data is a singleton!
pub struct TextTagManager {
    table: TextTagTable,

    edit_tags: RefCell<BTreeSet<&'static str>>,
}

impl TextTagManager {
    pub fn new() -> Self {
        let table = TextTagTable::new();

        let edit_tags: RefCell<BTreeSet<&'static str>> = RefCell::new(BTreeSet::new());

        Self { table, edit_tags }
    }

    pub fn table(&self) -> &gtk::TextTagTable {
        self.table.get_tag_table()
    }

    pub fn for_each_edit_tag<F>(&self, callback: F)
    where
        F: Fn(&gtk::TextTag),
    {
        // println!("for_each_edit_tag tag count before {}", self.edit_tags.borrow().len());
        for tag in self.edit_tags.borrow().deref() {
            callback(&self.table.get_tag(*tag).unwrap());
        }
    }

    pub fn move_cursor(&self, _textview: &gtk::TextView) {
        //ToDo needs to be a different signal, moves by mouse click are not covered
        self.edit_tags.borrow_mut().clear();
    }

    pub fn text_edit(&self, edit: TextEdit) {
        // println!("Text edit: {:?}", &edit);
        match edit {
            TextEdit::NewLine => self.edit_tags.borrow_mut().clear(),
        }
    }

    pub fn toggle_tag(&self, tag: &'static str) {
        let set = &mut self.edit_tags.borrow_mut();
        // println!("toggle_tag {} count before {}", tag, set.len());
        if set.contains(tag) {
            set.remove(tag);
        } else {
            set.insert(tag);
        }
        // println!("tag count after {}", set.len());
    }
}
