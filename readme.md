A quickly made Clipboard Manager for Windows. Mainly made to test out the [EGUI](https://github.com/emilk/egui) crate.

TODO:
 - MacOS/Linux Compatability
 - Store Files
 - Advanced Search
 - Title Starred
 - Fine tune max save size

To build and run use:

`cargo build --bin clipboard-app --release`

and then:

`cargo run --bin clipboard-tray --release`

The first command builds the GUI Application. The second command builds AND runs the Tray Application and Clipboard Listener.