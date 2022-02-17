use anyhow::Result;
use clipboard_common::{Listener, Config, StorageContainer};
use lazy_static::lazy_static;
use log::LevelFilter;
use log4rs::{config::{Root, Appender}, encode::pattern::PatternEncoder, append::file::FileAppender};
use core::mem::MaybeUninit;
use std::{sync::{Mutex, Arc, RwLock}, process::{Command, self}, path::PathBuf, thread};
use trayicon::*;
use winapi::{um::{winuser, processthreadsapi::{TerminateProcess, OpenProcess}, winnt::{HANDLE, PROCESS_QUERY_INFORMATION, PROCESS_TERMINATE}, handleapi::CloseHandle}, shared::minwindef::DWORD};

lazy_static! {
	pub static ref APPLICATION: Mutex<Option<u32>> = Mutex::new(None);
}



fn main() {
	let init = || -> Result<()> {
		// Logging
		init_logging()?;

		// Start Clipboard Listener.
		init_listener()?;

		// Application
		init_tray()?;

		Ok(())
	};

	if let Err(e) = init() {
		log::error!("{}", e);
	}
}



fn init_logging() -> Result<()> {
	let logfile = FileAppender::builder()
		.encoder(Box::new(PatternEncoder::new("{d} {l} {t} - {m}{n}")))
		.build("output.log")?;

	let config = log4rs::Config::builder()
		.appender(Appender::builder().build("logfile", Box::new(logfile)))
		.build(Root::builder()
		.appender("logfile")
		.build(LevelFilter::Info))?;

	log4rs::init_config(config)?;

	Ok(())
}


fn init_listener() -> Result<()> {
	let config = Arc::new(RwLock::new(Config::load()?));
	let store = StorageContainer::open("userdata.db")?;

	thread::spawn(move || {
		log::info!("Starting Listener");

		if let Err(e) = clipboard_common::AppListener::default().run(store, config) {
			log::error!(target: "clipboard_listener", "{}", e);
		}
	});

	Ok(())
}


fn init_tray() -> Result<()> {
	#[derive(Copy, Clone, Eq, PartialEq, Debug)]
	enum Events {
		ClickTrayIcon,
		Exit
	}

	let (s, r) = crossbeam_channel::unbounded();

	let icon = include_bytes!("../../app.ico");

	// Needlessly complicated tray icon with all the whistles and bells
	let _tray = TrayIconBuilder::new()
		.sender_crossbeam(s)
		.icon_from_buffer(icon)
		.tooltip("Clipboard")
		.on_click(Events::ClickTrayIcon)
		.menu(MenuBuilder::new().item("Exit", Events::Exit))
		.build()?;

	thread::spawn(move || {
		let _ = _tray;

		r.iter()
		.for_each(|m| match m {
			Events::ClickTrayIcon => {
				if let Err(e) = toggle_application() {
					log::error!("{}", e);
				}
			}

			Events::Exit => {
				if let Some(app_id) = *APPLICATION.lock().unwrap() {
					log::info!("Closing Application");
					CommandProcess::open(app_id).unwrap().kill().unwrap();
				}

				log::info!("Exiting Tray App");

				process::exit(0);
			}
		})
	});

	loop {
		unsafe {
			let mut msg = MaybeUninit::uninit();

			let bret = winuser::GetMessageA(msg.as_mut_ptr(), 0 as _, 0, 0);

			if bret > 0 {
				winuser::TranslateMessage(msg.as_ptr());
				winuser::DispatchMessageA(msg.as_ptr());
			} else {
				break;
			}
		}
	}

	Ok(())
}


pub fn toggle_application() -> Result<()> {
	let mut app = APPLICATION.lock().unwrap();

	if let Some(app_id) = *app {
		log::info!("Attempting to close Application");
		CommandProcess::open(app_id)?.kill()?;
	} else {
		log::info!("Attempting to open Application");

		let mut spawn = Command::new(path_to_application()?).spawn()?;

		*app = Some(spawn.id());

		thread::spawn(move || {
			if let Err(e) = spawn.wait() {
				log::error!("{}", e);
			}

			log::info!("application closed");

			// Application was killed. Unset it.
			*APPLICATION.lock().unwrap() = None;
		});
	}

	Ok(())
}


/// Application should be in the same folder as the tray
fn path_to_application() -> Result<PathBuf> {
	let mut app_path = std::env::current_exe()?;
	app_path.set_file_name("clipboard-app.exe");
	Ok(app_path)
}



// https://stackoverflow.com/questions/55230450

struct CommandProcess(HANDLE);

impl CommandProcess {
    fn open(pid: DWORD) -> Result<Self> {
        // https://msdn.microsoft.com/en-us/library/windows/desktop/ms684320%28v=vs.85%29.aspx
        let pc = unsafe { OpenProcess(PROCESS_QUERY_INFORMATION | PROCESS_TERMINATE, 0, pid) };

        if pc.is_null() {
            return Err(anyhow::anyhow!("!OpenProcess"));
        }

        Ok(Self(pc))
    }

    fn kill(self) -> Result<()> {
        unsafe { TerminateProcess(self.0, 1) };
        Ok(())
    }
}

impl Drop for CommandProcess {
    fn drop(&mut self) {
        unsafe { CloseHandle(self.0) };
    }
}