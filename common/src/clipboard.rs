use std::sync::{Arc, RwLock};


use anyhow::Result;
#[cfg(windows)]
pub use windows::*;
#[cfg(not(windows))]
pub use nonwindows::*;

use crate::{store::StorageContainer, config::Config};


pub trait Listener: Default {
	fn run(&mut self, conn: StorageContainer, config: Arc<RwLock<Config>>) -> Result<()>;
}


#[cfg(windows)]
mod windows {
    use std::io::{self, Cursor};
	use std::process;
	use std::sync::{Arc, RwLock};

	use anyhow::{Result, anyhow};
	use clipboard_win::SystemError;
    use image::ImageFormat;
	use log::error;
	use windows_win::{Messages, Window, raw};
	use windows_win::winapi;
	use windows_win::winapi::um::winuser::{AddClipboardFormatListener, RemoveClipboardFormatListener};

	use crate::config::Config;
	use crate::store::StorageContainer;

	pub fn set_clipboard_image(data_id: usize, store: &StorageContainer) -> Result<()> {
		let buffer = store.get_image(data_id)?;
		let image = image::load_from_memory(&buffer)?;

		let mut data = Cursor::new(Vec::new());
		image.write_to(&mut data, ImageFormat::Bmp)?;

		clipboard_win::set_clipboard(clipboard_win::formats::Bitmap, data.into_inner()).map_err(|v| anyhow!(v))?;

		Ok(())
	}

	// Creating a Clipboard Format Listener

	// A clipboard format listener is a window which has registered to be notified when the contents of the clipboard has changed.
	// This method is recommended over creating a clipboard viewer window because it is simpler to implement and
	//      avoids problems if programs fail to maintain the clipboard viewer chain properly or if a window in
	//      the clipboard viewer chain stops responding to messages.

	// A window registers as a clipboard format listener by calling the [AddClipboardFormatListener] function.
	// When the contents of the clipboard change, the window is posted a [WM_CLIPBOARDUPDATE] message.
	// The registration remains valid until the window unregister itself by calling the [RemoveClipboardFormatListener] function.

	fn attempt_to_register_format() -> u32 {
		for _ in 0..10 {
			let format = clipboard_win::register_format("HTML Format");

			if let Some(format) = format {
				// TODO: Doesn't always return true for some reason.
				// if clipboard_win::raw::is_format_avail(format.get()) {
				return format.get();
			} else {
				error!(target: "clipboard_listener", "HTML Format Creation Error: {}", SystemError::last());
			}
		}

		error!(target: "clipboard_listener", "Unable to create HTML Format for Clipboard");

		process::exit(1);
	}

	pub struct AppListener {
		html_format: u32
	}

	impl Default for AppListener {
		fn default() -> Self {
			let html_format = attempt_to_register_format();

			Self {
				html_format
			}
		}
	}

	impl super::Listener for AppListener {
		fn run(&mut self, conn: StorageContainer, config: Arc<RwLock<Config>>) -> Result<()> {
			let window = Window::from_builder(raw::window::Builder::new().class_name("STATIC").parent_message())?;

			let _dog = ListenerGuard::new(&window)?;

			// 0x031D (797) is Clipboard update.
			for msg in Messages::new().window(Some(window.inner())).low(Some(0x031D)).high(Some(0x031D)) {
				match msg {
					Ok(_) => {
						{
							// Reload config each update.
							// TODO FIX: added since we update the config in the GUI but don't update it for the tray where the events are listened to.
							config.write().unwrap().reload()?;
						}

						if let Err(e) = self.new_clipboard_update(&conn, &*config.read().unwrap()) {
							error!(target: "clipboard_listener", "{:?}", e);
						}
					}

					Err(error) => {
						error!(target: "clipboard_listener", "{:?}", error);
					}
				}
			}

			Ok(())
		}
	}

	impl AppListener {
		fn new_clipboard_update(&self, conn: &StorageContainer, config: &Config) -> Result<()> {
			let _clippy = clipboard_win::Clipboard::new_attempts(10).map_err(|v| anyhow::anyhow!(v))?;

			// Text Clipboard
			if config.stores.text.enabled && clipboard_win::is_format_avail(clipboard_win::formats::CF_UNICODETEXT) {
				let mut data = Vec::new();

				if let Err(e) = clipboard_win::raw::get_vec(self.html_format, &mut data) {
					// Error Codes: https://docs.microsoft.com/en-us/windows/win32/debug/system-error-codes--1000-1299-
					error!(target: "clipboard_listener", "Clipboard Update Error {:?}", e);
				}

				// Parse clipboard html and wrap in Option to check if it's empty or not.
				let html_data = Some(parse_html_clipboard(String::from_utf8(data)?)).filter(|v| !v.is_empty());

				let text_data = clipboard_win::get::<String, _>(clipboard_win::Unicode).map_err(|v| anyhow::anyhow!(v))?;

				if !text_data.is_empty() {
					if let Err(e) = conn.add_text(text_data, html_data, config) {
						error!(target: "clipboard_listener", "[add_text] Clipboard Text Error: {:?}", e);
					}
				}
			}

			// Image Clipboard
			if config.stores.image.enabled && clipboard_win::is_format_avail(clipboard_win::formats::CF_BITMAP) {
				let image_data = clipboard_win::get::<Vec<u8>, _>(clipboard_win::formats::Bitmap).map_err(|v| anyhow::anyhow!(v))?;

				match image::load_from_memory(&image_data) {
					Ok(img) => {
						let thumb = img.thumbnail(64, 64);
						let mut buffer = Cursor::new(Vec::new());
						let _ = thumb.write_to(&mut buffer, image::ImageFormat::Jpeg);

						let image_thumb_data = Some(buffer.into_inner()).filter(|v| !v.is_empty());

						if let Err(e) = conn.add_image(image_data, image_thumb_data, config) {
							error!(target: "clipboard_listener", "[add_img] Clipboard Image Error: {:?}", e);
						}
					}

					Err(e) => error!(target: "clipboard_listener", "Image Load Error: {:?}", e)
				}
			}

			// File Clipboard

			Ok(())
		}
	}




	pub struct ListenerGuard(winapi::shared::windef::HWND);

	impl ListenerGuard {
		#[inline]
		pub fn new(window: &Window) -> io::Result<Self> {
			let window = window.inner();

			unsafe {
				if AddClipboardFormatListener(window) != 1 {
					Err(io::Error::last_os_error())
				} else {
					Ok(ListenerGuard(window))
				}
			}
		}
	}

	impl Drop for ListenerGuard {
		fn drop(&mut self) {
			unsafe {
				RemoveClipboardFormatListener(self.0);
			}
		}
	}




	static FRAG_START: &str = "<!--StartFragment-->";
	static FRAG_END: &str = "<!--EndFragment-->";

	fn parse_html_clipboard(value: String) -> String {
		if let (Some(start), Some(end)) = (value.find(FRAG_START), value.find(FRAG_END)) {
			value[start + FRAG_START.len()..end].to_string()
		} else {
			value
		}
	}
}



#[cfg(not(windows))]
mod nonwindows {
    use std::{io, sync::{RwLock, Arc}};

    use anyhow::Result;
    use cli_clipboard::linux_clipboard::LinuxClipboardContext;

    use crate::{StorageContainer, Config};

	pub fn set_clipboard_image(_data_id: usize, _store: &StorageContainer) -> Result<()> {
		panic!("Unable to set clipboard image. Unsupported OS");
	}


	pub struct AppListener {
		_ctx: LinuxClipboardContext
	}

	impl Default for AppListener {
		fn default() -> Self {
			use cli_clipboard::ClipboardProvider;

			Self {
				_ctx: cli_clipboard::ClipboardContext::new().unwrap()
			}
		}
	}

	impl super::Listener for AppListener {
		fn run(&mut self, _conn: StorageContainer, _config: Arc<RwLock<Config>>) -> io::Result<()> {
			panic!("Unable to listen for clipboard changes. Unsupported OS");
		}
	}
}