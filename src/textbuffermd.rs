use crate::textbufferext::TextBufferExt2;
use crate::texttag::{Tag, TextTagExt2};
use crate::texttagtable::TextTagTable;
use gtk::TextBufferExt;
use pulldown_cmark::{CodeBlockKind, Event, Options, Parser};

type CTag<'a> = pulldown_cmark::Tag<'a>;

// Todo: Check if the newline handling work on other platforms as expected (e.g. Windows)
pub const NEWLINE: &str = "\n";
const NEWLINE_CHAR: char = '\n';
const BREAK: &str = "<br/>";
const BREAK_NEWLINE: &str = "<br/>\n";
// ToDo: escaping is far from complete
const ESCAPES_EVERYWHERE: [char; 2] = ['`', '_'];
const ESCAPES_ONLY_IN_BLOCK: [char; 1] = ['*'];

pub trait TextBufferMd {
    fn to_markdown(&self) -> String;
    fn insert_markdown(&self, iter: &mut gtk::TextIter, markdown: &str);

    fn assign_markup(&self, markup: &str) -> &gtk::TextBuffer;
    fn assign_markdown(&self, markdown: &str, buffer_is_modified: bool) -> &gtk::TextBuffer;
    fn apply_tag_offset(&self, iter: &mut gtk::TextIter, tag_name: &str, start_offset: i32);
    // ToDo: duplicated code for image and link
    fn apply_image_offset(&self, iter: &gtk::TextIter, image: &str, title: &str, start_offset: i32);

    fn convert_colors(&self, tag: &str, pos_start: i32);
}

impl TextBufferMd for gtk::TextBuffer {
    fn to_markdown(&self) -> String {
        // add newline at end if needed
        let mut end = self.end_iter();
        let mut start = end.clone();
        if start.backward_char() && self.get_text(&start, &end, false).ne(NEWLINE) {
            self.insert(&mut end, NEWLINE);
        }

        // resulting string
        let mut s = String::new();

        // currently open tags
        let mut open: Vec<String> = vec![];
        // next_open catches unnecessary open tags from the overflow list - they get only written,
        // if the next char is not the corresponding closing tag.
        let mut next_open: Vec<&str> = vec![];

        let mut newline_count = 0; // empty consecutive newlines in the editor
        let mut has_image = false;
        let mut has_link = false;
        let mut in_code_block = false;
        let mut formatted = true;
        let mut is_start_of_line = true; // after newlines and possible white space

        let mut it = self.start_iter();
        let mut c = it.char();
        while c != char::from(0) {
            // newline handling
            if c == NEWLINE_CHAR {
                newline_count += 1;
                if newline_count > 1 && !in_code_block {
                    if newline_count > 2 {
                        s += NEWLINE;
                    }
                    s += BREAK;
                    it.forward_char();
                    c = it.char();
                    continue;
                }
            } else {
                if newline_count > 1 && !in_code_block {
                    s += NEWLINE;
                    s += NEWLINE;
                }
                if newline_count > 0 {
                    is_start_of_line = true;
                }
                newline_count = 0;
            }

            // closing tags before new opening tags
            let off_tags = it.get_toggled_tags(false);
            if has_image {
                for tag in &off_tags {
                    if let Some(image) = tag.get_image() {
                        has_image = false;
                        s += format!("]({})", image.as_str()).as_ref();
                        continue;
                    }
                }
            }
            if has_link {
                for tag in &off_tags {
                    if let Some(link) = tag.get_link() {
                        has_link = false;
                        s += format!("]({})", link.as_str()).as_ref();
                        continue;
                    }
                }
            }
            for tag in off_tags.iter().rev() {
                // reverse to keep multiple tags in order
                let name = tag.get_name();
                if name.eq(Tag::CODE) {
                    in_code_block = false;
                    formatted = true;
                } else if name.eq(Tag::MONO) {
                    formatted = true;
                }
                if TextTagTable::md_end_tag(name.as_str()).is_some() {
                    let mut overflow: Vec<String> = vec![];
                    let mut top = open.pop();
                    while let Some(top_name) = top.clone() {
                        let matching_start = TextTagTable::md_start_tag(top_name.as_str()).unwrap();
                        if let Some(index) = next_open.iter().position(|i| i.eq(&matching_start)) {
                            next_open.remove(index);
                            if let Some(index) = open.iter().position(|i| i.eq(&top_name)) {
                                open.remove(index);
                            } else {
                                // the candidate was top, which is already removed from open
                                // ToDo: this assert fails for "{++{==Hallo **Welt!**==}++}\n"
                                //assert_eq!(name, top_name);
                            }
                        } else {
                            // it should be ok, to not write out the remaining open tags here
                            s += TextTagTable::md_end_tag(top_name.as_str()).unwrap();
                        }
                        next_open.clear();
                        if top_name.ne(&name) {
                            overflow.push(top_name);
                            top = open.pop();
                        } else {
                            break;
                        }
                    }
                    for value in overflow.iter().rev() {
                        next_open.push(TextTagTable::md_start_tag(value).unwrap());
                        open.push(value.clone());
                    }
                }
            }

            let on_tags = it.get_toggled_tags(true);
            let mut handle_image = false;
            let mut handle_link = false;
            // check first if we enter an unformatted block
            let mut stop_formatting_here = false;
            for tag in on_tags.iter().rev() {
                // reverse to keep multiple tags in order
                let name = tag.get_name();
                if name.eq(Tag::CODE) {
                    in_code_block = true;
                    stop_formatting_here = formatted;
                    formatted = false;
                    continue;
                } else if name.eq(Tag::MONO) {
                    stop_formatting_here = formatted;
                    formatted = false;
                    continue;
                }
            }
            if stop_formatting_here {
                // ToDo: close all open tags!
            }

            // reverse loop to keep multiple tags in order
            for tag in on_tags.iter().rev() {
                let name = tag.get_name();
                if formatted || name.eq(Tag::MONO) || name.eq(Tag::CODE) {
                    if let Some(diff) = TextTagTable::md_start_tag(name.as_str()) {
                        open.push(name);
                        next_open.push(diff);
                    } else if tag.get_image().is_some() {
                        has_image = true;
                        handle_image = true;
                    } else if tag.get_link().is_some() {
                        has_link = true;
                        handle_link = true;
                    }
                }
            }

            for t in next_open.drain(..) {
                s += t;
            }
            if handle_image {
                s += "![";
            }
            if handle_link {
                s += "[";
            }

            // newlines in regular lines the editor become paragraphs in markdown
            if c == NEWLINE_CHAR && !in_code_block {
                s += NEWLINE;
            }
            if formatted
                && (ESCAPES_EVERYWHERE.contains(&c)
                    || (!is_start_of_line) && ESCAPES_ONLY_IN_BLOCK.contains(&c))
            {
                s.push('\\');
            }
            s.push(c);

            is_start_of_line = is_start_of_line && c.is_whitespace();

            it.forward_char();
            c = it.char();
        }

        // close all open tags
        for t in next_open.drain(..) {
            s += t;
        }
        if !open.is_empty() {
            while s.ends_with(NEWLINE) {
                s.pop();
            }
            for value in open.iter().rev() {
                s += TextTagTable::md_end_tag(value).unwrap();
            }
        }

        // exactly one newline at the end
        while s.ends_with(NEWLINE) {
            s.pop();
        }
        s += NEWLINE;

        s
    }

    fn insert_markdown(&self, iter: &mut gtk::TextIter, markdown: &str) {
        let pos_start = iter.offset();

        let mut options = Options::empty();
        options.insert(Options::ENABLE_STRIKETHROUGH);
        let parser = Parser::new_ext(markdown, options);

        let mut pos_heading = 0;
        let mut pos_image = 0;
        let mut pos_link = 0;
        let mut pos_bold = 0;
        let mut pos_italic = 0;
        let mut pos_mono = 0;
        let mut pos_strike = 0;

        let mut list_ident = 0;
        let mut list_number: Vec<Option<u64>> = vec![];
        let mut list_item_empty = false; // needed for paragraphs in lists

        for event in parser {
            //println!("\nEvent:{:?}", &event);
            match event {
                Event::Start(tag) => match tag {
                    CTag::Heading(_) => pos_heading = iter.offset(),
                    CTag::Paragraph => {
                        if !iter.starts_line() && (list_ident == 0 || !list_item_empty) {
                            self.insert(iter, NEWLINE);
                            list_item_empty = false;
                        }
                        if list_ident > 0 && !list_item_empty {
                            self.insert(iter, "    ".repeat(list_ident).as_str());
                        }
                    }
                    CTag::Image(..) => pos_image = iter.offset(),
                    CTag::Link(..) => pos_link = iter.offset(),
                    CTag::List(number) => {
                        list_number.push(number);
                        // a sublist comes before the end tag
                        if !iter.starts_line() {
                            self.insert(iter, NEWLINE);
                        }
                        list_ident += 1;
                    }
                    CTag::Item => {
                        list_item_empty = true;
                        self.insert(
                            iter,
                            format!(
                                "{}{} ",
                                "    ".repeat(list_ident - 1),
                                if let Some(Some(i)) = list_number.last_mut() {
                                    let r = i.to_string() + ".";
                                    *i += 1;
                                    r
                                } else {
                                    String::from("*")
                                },
                            )
                            .as_str(),
                        )
                    }
                    CTag::Strong => pos_bold = iter.offset(),
                    CTag::Emphasis => pos_italic = iter.offset(),
                    CTag::CodeBlock(kind) => match kind {
                        CodeBlockKind::Indented => pos_mono = iter.offset(),
                        CodeBlockKind::Fenced(_) => pos_mono = iter.offset(),
                    },
                    CTag::Strikethrough => pos_strike = iter.offset(),
                    _ => {} //println!("\nStart tag: {:?}", &tag),
                },
                Event::End(tag) => match tag {
                    CTag::Heading(level) => {
                        let tag = match level {
                            1 => Tag::H1,
                            2 => Tag::H2,
                            3 => Tag::H3,
                            4 => Tag::H4,
                            5 => Tag::H5,
                            6 => Tag::H6,
                            _ => continue,
                        };
                        self.apply_tag_offset(iter, tag, pos_heading);
                        self.insert(iter, NEWLINE);
                    }
                    CTag::Paragraph => {
                        // paragraphs in lists already have a newline
                        if !iter.starts_line() {
                            self.insert(iter, NEWLINE);
                        }
                    }
                    CTag::Image(_, image, title) => {
                        self.apply_image_offset(iter, image.as_ref(), title.as_ref(), pos_image)
                    }
                    CTag::Link(_, link, title) => {
                        self.apply_link_offset(iter, link.as_ref(), title.as_ref(), pos_link)
                    }
                    CTag::List(_) => {
                        list_ident -= 1;
                        list_number.pop();
                    }
                    CTag::Item => {
                        // a sublist comes before the end tag
                        // also sublists close directly after one another
                        if !iter.starts_line() {
                            self.insert(iter, NEWLINE);
                        }
                    }
                    CTag::Strong => self.apply_tag_offset(iter, Tag::BOLD, pos_bold),
                    CTag::Emphasis => self.apply_tag_offset(iter, Tag::ITALIC, pos_italic),
                    CTag::CodeBlock(kind) => match kind {
                        CodeBlockKind::Indented => self.apply_tag_offset(iter, Tag::MONO, pos_mono),
                        CodeBlockKind::Fenced(_) => {
                            self.apply_tag_offset(iter, Tag::CODE, pos_mono)
                        }
                    },
                    CTag::Strikethrough => self.apply_tag_offset(iter, Tag::STRIKE, pos_strike),
                    _ => {} //println!("\nEnd tag: {:?}", &tag),
                },
                Event::Text(text) => {
                    self.insert(iter, text.as_ref());
                    list_item_empty = false;
                }
                Event::Code(text) => {
                    pos_mono = iter.offset();
                    self.insert(iter, text.as_ref());
                    self.apply_tag_offset(iter, Tag::MONO, pos_mono);
                }
                Event::Html(html) => {
                    // special newline handling
                    let str = html.as_ref();
                    if str.eq(BREAK_NEWLINE) {
                        self.insert(iter, NEWLINE);
                    } else {
                        self.insert(iter, str);
                    }
                }
                // Event::SoftBreak => self.insert(iter, NEWLINE),
                Event::HardBreak => self.insert(iter, NEWLINE),
                Event::Rule => {
                    let pos_rule = iter.offset();
                    self.insert(iter, format!("{}{}", Tag::MD_RULE, NEWLINE).as_str());
                    self.apply_tag_offset(iter, Tag::RULE, pos_rule);
                }
                // FootnoteReference(name) => {
                //     let len = self.numbers.len() + 1;
                //     self.write("<sup class=\"footnote-reference\"><a href=\"#")?;
                //     escape_html(&mut self.writer, &name)?;
                //     self.write("\">")?;
                //     let number = *self.numbers.entry(name).or_insert(len);
                //     write!(&mut self.writer, "{}", number)?;
                //     self.write("</a></sup>")?;
                // }
                // TaskListMarker(true) => {
                //     self.write("<input disabled=\"\" type=\"checkbox\" checked=\"\"/>\n")?;
                // }
                // TaskListMarker(false) => {
                //     self.write("<input disabled=\"\" type=\"checkbox\"/>\n")?;
                // }
                _ => {} //println!("\nEvent:{:?}", &event),
            }
        }

        self.convert_colors(Tag::GREEN, pos_start);
        self.convert_colors(Tag::RED, pos_start);
        self.convert_colors(Tag::BLUE, pos_start);
        self.convert_colors(Tag::YELLOW, pos_start);
    }

    fn assign_markup(&self, markup: &str) -> &gtk::TextBuffer {
        self.delete(&mut self.start_iter(), &mut self.end_iter());
        self.insert_markup(&mut self.start_iter(), markup);
        self
    }

    fn assign_markdown(&self, markdown: &str, buffer_is_modified: bool) -> &gtk::TextBuffer {
        self.delete(&mut self.start_iter(), &mut self.end_iter());
        self.insert_markdown(&mut self.start_iter(), markdown);
        self.set_modified(buffer_is_modified);
        self
    }

    fn apply_tag_offset(&self, iter: &mut gtk::TextIter, tag_name: &str, start_offset: i32) {
        let mut start = iter.clone();
        start.backward_chars(iter.offset() - start_offset);
        // formatting should not be applied to the last newline (if present)
        if iter.starts_line() {
            let mut end = iter.clone();
            end.backward_char();
            self.apply_tag(&self.tag_table().lookup(tag_name).unwrap(), &start, &end);
        } else {
            self.apply_tag(&self.tag_table().lookup(tag_name).unwrap(), &start, &iter);
        }
    }

    fn apply_image_offset(
        &self,
        iter: &gtk::TextIter,
        image: &str,
        title: &str,
        start_offset: i32,
    ) {
        let mut start = iter.clone();
        start.backward_chars(iter.offset() - start_offset);
        let tag = if title.is_empty() {
            self.create_image_tag(image)
        } else {
            self.create_image_tag(format!("{} \"{}\"", image, title).as_str())
        };
        self.apply_tag(&tag, &start, &iter);
    }

    // Convert markup for colors to the corresponding formatting and delete the markup
    fn convert_colors(&self, tag: &str, pos_start: i32) {
        let mut offset = pos_start;
        while let Some((start_tag_start, start_tag_end)) =
            self.get_iter_at_offset(offset).forward_search(
                TextTagTable::md_start_tag(tag).unwrap(),
                gtk::TextSearchFlags::VISIBLE_ONLY,
                None,
            )
        {
            let offset_sts = start_tag_start.offset();
            let offset_ste = start_tag_end.offset();

            if let Some((mut end_tag_start, mut end_tag_end)) = start_tag_end.forward_search(
                TextTagTable::md_end_tag(tag).unwrap(),
                gtk::TextSearchFlags::VISIBLE_ONLY,
                None,
            ) {
                self.apply_tag(
                    &self.tag_table().lookup(tag).unwrap(),
                    &start_tag_end,
                    &end_tag_start,
                );
                offset = end_tag_start.offset() - 6; // length of two tags, which get removed
                self.delete(&mut end_tag_start, &mut end_tag_end);
                end_tag_start.set_offset(offset_sts);
                end_tag_end.set_offset(offset_ste);
                self.delete(&mut end_tag_start, &mut end_tag_end);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gtk::TextTagExt;

    #[allow(dead_code)]
    fn cb(
        _buffer: &gtk::TextBuffer,
        tag: &gtk::TextTag,
        _start: &gtk::TextIter,
        _end: &gtk::TextIter,
    ) {
        println!("apply tag {:?} {}", tag, tag.get_property_name().unwrap().as_str());
    }

    fn buffer_new() -> gtk::TextBuffer {
        let _ = gtk::init();
        let table = TextTagTable::new();
        let buffer = gtk::TextBuffer::new(Some(table.tag_table()));
        //buffer.connect_apply_tag(cb);
        buffer
    }

    #[allow(dead_code)]
    #[test]
    fn test_simple_markup() {
        let buffer = buffer_new();

        let pairs = vec![
            ("<b>Hello bold</b> World", "**Hello bold** World\n"),
            ("<i>Hello italic </i>World", "*Hello italic *World\n"),
            ("<tt>Hello mono</tt>World", "``Hello mono``World\n"),
            ("<s>Hello strike World</s>", "~~Hello strike World~~\n"),
            ("<b>Hello <i>World</i></b>", "**Hello *World***\n"),
            ("<b>Hello <i>World</i></b><i>again</i>", "**Hello *World****again*\n"),
            ("<b>Hello <i>World</i></b> <i>again</i>", "**Hello *World*** *again*\n"),
        ];

        for (input, output) in pairs {
            buffer.assign_markup(input);
            assert_eq!(buffer.to_markdown().as_str(), output);
        }
    }

    #[allow(dead_code)]
    #[test]
    fn test_simple_markdown_turnaround() {
        let buffer = buffer_new();

        let s1 = format!("``Code``\n\n{}\n\n**Bold Text**\n", Tag::MD_RULE);

        let strings = vec![
            "Hello world!\n",
            "☺☹ ♠♣♥♦ äöüß\n",
            "**bold**\n",
            "**Hello *World****again*\n",
            "**Hello *World*** *again*\n",
            "``mono``\n",
            "``mono **bold**``\n",
            "Newlines\n\n<br/>\n\nOne above\n\n<br/>\n<br/>\n<br/>\n\n{--Three--} above\n\nNone\n\n<br/>\n\nOne *above*\n",
            "* first\n\n* ``second``\n\n    * inner first\n\n    * inner second\n\n* third\n",
            "* first\n\n* second\n\n    * {++inner first++}\n\n    * inner **second**\n\n* third\n",
            "* first\n\n* second\n\n    3. {++inner first++}\n\n    4. inner **second**\n\n* third\n",
            "1. first\n\n2. second\n\n    * {++inner first++}\n\n    * inner **second**\n\n3. third\n",
            "# Level 1\n\n# Level 1\n\nSome ``text``\n\n### Level 3\n\n##### Level 5\n\n<br/>\n<br/>\n<br/>\n\n##### Level 5b\n",
            "## Hallo Welt\n",
            "### Hallo Welt\n",
            "#### Hallo Welt\n",
            "##### Hallo Welt\n",
            "###### Hallo **Welt**\n\nNext paragraph!\n",
            s1.as_str(),
            "My text\n\n* first\n\n* second\n\n    * third\n\n1. foo\n\n2. bar\n\n    * faz\n\n    * wuz\n\n        3. red\n\n        4. green\n\n        5. blue\n\n3. baz\n",
            "* item\n\n    ``paragraph`` in item\n\n    **paragraph** two in item\n\n    * child item\n\n        paragraph in child item\n\n* item two\n\nnormal paragraph\n",
            "* [Marko Editor](http://www.marko-editor.com)\n\n* [PDF](file:///home/foo/doc.pdf)\n",
            "[Marko Editor](http://www.marko-editor.com)\n",
            "**[Marko Editor]**(http://www.marko-editor.com)\n",
            "{==**[Marko Editor]**==}(http://www.marko-editor.com)\n",
            "![Marko Editor screenshot](./doc/marko-editor-screenshot.png?raw=true)\n",
            "![Marko Editor screenshot](./doc/marko-editor-screenshot.png?raw=true \"Marko Editor\")\n",
            "```\nfor (int i=0; i<10; ++i) {\n    std::cout << i << std::endl;\n}\n```\n\n```\nOne\n\nTwo\n\n\nThree\n\n\n\nDone\n```\n",
            "**Bold**\n\n```\nfor (int i=0; i<10; ++i) {\n    std::cout << i << std::endl;\n}\n```\n\n* Item One\n\n* Item Two\n",
            "{++{==Hallo **Welt!**==}++}\n",
            "**{++text++}**\n",
            "{++**text**++}\n",
            "***text***\n",
            "{++{==***text***==}++}\n",
            "~~***text***~~\n",
            "*ABC**Hello*** **World** *again*\n",
            "``**text**``\n",
            "Hallo \\*Welt\\*\n\nHallo ``*Welt*``\n",
            "* **Hallo Welt**\n\n* **Hallo zwei**\n\n    * **Hallo drei**\n\n    * 5 \\* 4 = 20\n\n    * **5 \\* 4 = 20**\n",
            "```\n* foo\n    * **bar**\n* _baz_\n```\n",
            "```\n* first\n* second\n\n<br/>\n\n    * inner first\n\n    * inner **second**\n* third\n```\n",

        ];

        for s in strings {
            assert_eq!(buffer.assign_markdown(s, true).to_markdown().as_str(), s);
        }
    }

    #[allow(dead_code)]
    #[test]
    fn test_markdown_turnaround() {
        let buffer = buffer_new();

        let pairs = vec![(
                             "* first\n* second\n    * inner first\n    * inner second\n        * second inner first\n        * second inner second\n* third\n* fourth\n",
                             "* first\n\n* second\n\n    * inner first\n\n    * inner second\n\n        * second inner first\n\n        * second inner second\n\n* third\n\n* fourth\n"
                         ),
                         ("My text\n\nAnother paragraph...", "My text\n\nAnother paragraph...\n"),
                         ("[Marko Editor](http://www.marko-editor.com)","[Marko Editor](http://www.marko-editor.com)\n",),
                         ("**Hello _World_**_again_\n","**Hello *World****again*\n"),
                         ("**Hello _World_** _again_\n","**Hello *World*** *again*\n"),
                         ("**_text_**\n","***text***\n"),
                         ("_**text**_\n","***text***\n"),
                         ("*ABC**Hello***** World** *again*\n","*ABC**Hello***\\*\\* World\\*\\* *again*\n"),
                         // ToDo: these results are broken and need to be changed
                         // ToDo: mixed formatting in links
                         ("[**Marko** {++Editor++} *Website*](http://www.marko-editor.com)\n", "**[Marko** {++Editor++} *Website](http://www.marko-editor.com)*\n"),
                         // ToDo: critics markup in code blocks
                         ("```\nHallo {++Welt++}\n```\n", "```\nHallo Welt\n```\n"),


        ];

        for (input, output) in pairs {
            assert_eq!(buffer.assign_markdown(input, true).to_markdown().as_str(), output);
        }
    }

    #[allow(dead_code)]
    #[test]
    fn test_current_work() {
        let buffer = buffer_new();
        let s = r#"
"#;

        let r = s;
        // https://spec.commonmark.org/dingus/
        assert_eq!(buffer.assign_markdown(s, true).to_markdown().as_str(), r);
    }
}
