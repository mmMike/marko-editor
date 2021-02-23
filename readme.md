# Marko Editor

Marko Editor is a simple WYSIWYG editor for note taking written in Rust and GTK 4. It uses Markdown as storage format (with critics extension) and can read simple Markdown files. However, the main focus of Marko Editor is WYSIWYG note taking and not being a 100% compliant Markdown editor.

![Marko Editor screenshot](./doc/marko-editor-screenshot.png?raw=true "Marko Editor")

## Background

Marko Editor is a learning project driven by my personal note taking requirements. Coming from a C++ and Qt background this is my first deeper venture into Rust and GTK. So you should expect some short cummings in the source code:

* Not (yet) idiomatic in several places.

* Sometimes feature driven with technical dept.

* Incomplete error handling with many ``unwrap``.

### Interesting Rust and GTK Parts

While the source code is not perfect parts of it might serve as examples for GTK 4 development:

* Clean state management for UI callbacks with macros. Only one ``connect!`` call per callback similar to Qt.

* Restoring the window position on X11.

* Dynamic menu content depending on runtime data.

* Communication with UI thread from worker thread via channels - see also [MPSC Channel API for painless usage of threads with GTK in Rust](https://coaxion.net/blog/2019/02/mpsc-channel-api-for-painless-usage-of-threads-with-gtk-in-rust/)

* Structuring of the application for re-use and modularity - see also [GTK3 Patterns in Rust: Structure](https://blog.samwhited.com/2019/02/gtk3-patterns-in-rust-structure/).

## Development Status

Alpha stage - incomplete, not ready for production.

--- ---- ----- ------- ----- ---- ---

## Building

### Linux

* Make sure rust (the latest stable version) and the libs for gtk4 and x11 are installed.

* ``cargo run`` to compile and run the program.

* ``make DESTDIR=package/usr`` creates the contents for a standard installation on Linux.

* ``PKGBUILD`` for Arch Linux is supplied.

--- ---- ----- ------- ----- ---- ---

## License

Marko Editor is distributed under the terms of the GPL version 3. See [LICENSE](LICENSE).
