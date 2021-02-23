# from: https://stackoverflow.com/a/18092830
# docs: http://www.gnu.org/software/make/manual/make.html

# call: make DESTDIR=...

DESTDIR=./install

COPY_FILES = $(DESTDIR)/share/applications/marko-editor.desktop $(DESTDIR)/share/icons/hicolor/scalable/apps/marko-editor.svg $(DESTDIR)/share/menu/marko-editor $(DESTDIR)/share/pixmaps/marko-editor.xpm

all: $(COPY_FILES) cargo

$(DESTDIR)/share/applications/marko-editor.desktop: package/marko-editor.desktop
$(DESTDIR)/share/icons/hicolor/scalable/apps/marko-editor.svg: package/marko-editor.svg
$(DESTDIR)/share/menu/marko-editor: package/marko-editor.menu
$(DESTDIR)/share/pixmaps/marko-editor.xpm: package/marko-editor.xpm

$(DESTDIR)/%:
	cp -f $< $@

$(COPY_FILES) : | folders

folders:
	mkdir -p $(DESTDIR)/share/applications
	mkdir -p $(DESTDIR)/share/icons/hicolor/scalable/apps
	mkdir -p $(DESTDIR)/share/menu
	mkdir -p $(DESTDIR)/share/pixmaps

cargo:
	cargo install --path=. --root=$(DESTDIR)
