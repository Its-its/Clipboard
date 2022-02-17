#![allow(clippy::new_without_default)]

use std::sync::{Arc, RwLock};

use chrono::{DateTime, Utc};
use log::LevelFilter;
use log4rs::{append::file::FileAppender, encode::pattern::PatternEncoder, config::{Appender, Root}};

mod gui;


pub use clipboard_common::*;
pub use gui::*;

pub fn main() -> anyhow::Result<()> {
	let config = Config::load()?;

	if config.app.logging {
		let logfile = FileAppender::builder()
			.encoder(Box::new(PatternEncoder::new("{d} {l} {t} - {m}{n}")))
			.build("output.log")?;

		let config = log4rs::Config::builder()
			.appender(Appender::builder().build("logfile", Box::new(logfile)))
			.build(Root::builder()
			.appender("logfile")
			.build(LevelFilter::Info))?;

		log4rs::init_config(config)?;
	}

	log::info!("Starting Application");
	log::info!("Opening Database");

	// Initiations
	let config = Arc::new(RwLock::new(config));
	let store = StorageContainer::open("userdata.db")?;

	// Open the APP
	open_app(config, store);

	Ok(())
}

pub fn open_app(config: Arc<RwLock<Config>>, store: StorageContainer) {
	log::info!("Launching GUI");

	let native_options = eframe::NativeOptions {
		always_on_top: { config.read().unwrap().app.always_on_top },
		.. Default::default()
	};

	let app = App::new(store, config);
	eframe::run_native(Box::new(app), native_options);
}

pub fn item_time_ago(value: DateTime<Utc>, now: DateTime<Utc>) -> String {
	let mut time_ago = now.signed_duration_since(value).num_seconds();

	// Seconds
	if time_ago < 60 {
		return format!("{} seconds ago", time_ago);
	}

	// Minutes
	time_ago /= 60;
	if time_ago < 60 {
		return format!("{} minutes ago", time_ago);
	}

	// Hours
	time_ago /= 60;
	if time_ago < 24 {
		return format!("{} hours ago", time_ago);
	}

	// Days
	time_ago /= 24;
	if time_ago < 30 {
		return format!("{} days ago", time_ago);
	}

	// Months
	time_ago /= 30;
	if time_ago < 12 {
		return format!("{} months ago", time_ago);
	}

	// Years
	time_ago /= 12;
	format!("{} years ago", time_ago)
}