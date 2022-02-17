use anyhow::Result;
use eframe::{egui, epi};


use crate::{Tab, StorageContainer, Config};


#[derive(Default)]
pub struct SettingsTab {
	database_size: Option<Result<u64>> // read File size.
}

impl Tab for SettingsTab {
	fn on_open(&mut self, _frame: &epi::Frame, _store: &StorageContainer, _config: &mut Config) {
		self.database_size = Some(std::fs::metadata("userdata.db").map(|v| v.len()).map_err(|v| v.into()));
	}

	fn update(&mut self, ctx: &egui::CtxRef, _frame: &epi::Frame, _store: &StorageContainer, config: &mut Config) {
		egui::CentralPanel::default()
		.show(ctx, |ui| {
			ui.heading("Settings");

			ui.add_space(4.0);

			if ui.button("Save Settings").clicked() {
				let _ = config.save();
			}

			ui.add_space(20.0);

			// ui.checkbox(&mut config.app.hide_when_deleted, "Hide when deleted");
			ui.checkbox(&mut config.app.always_on_top, "Always On Top (Requires App Restart)");
			ui.add(egui::Slider::new(&mut config.app.query_return_limit, 5..=100).text("Query Batch Size"));


			ui.add_space(20.0);
			ui.heading("Store Types");

			// Text
			ui.label("Text");
			ui.indent(123, |ui| {
				ui.checkbox(&mut config.stores.text.enabled, "Save text?");
				ui.add(egui::Slider::new(&mut config.stores.text.max_size, 1..=1000).text("Max Size (MB)"));
			});

			ui.label("Images");
			ui.indent(456, |ui| {
				ui.checkbox(&mut config.stores.image.enabled, "Save images?");
				ui.add(egui::Slider::new(&mut config.stores.image.max_size, 1..=10240).text("Max Size (MB)"));
			});

			// ui.label("Files");
			// ui.indent(789, |ui| {
			// 	ui.checkbox(&mut false, "Save files?");
			// 	ui.add(egui::Slider::new(&mut 5120i64, 1..=10240).text("Max Size"));
			// });

			// Authentication (Button, popup)

			ui.add_space(20.0);

			// Database size
			if let Some(size) = self.database_size.as_ref() {
				match size {
					&Ok(v) => {
						ui.label(format!("Database Size {:?}", display_size(v)));
					}

					Err(v) => {
						ui.label(format!("Database Size Error {}", v));
					}
				}
			} else {
				ui.label("Unknown Database Size");
			}
		});
	}
}


fn display_size(mut bytes: u64) -> String {
	if bytes < 1000 {
		return format!("{} Bytes", bytes);
	}

	bytes /= 1000;
	if bytes < 1000 {
		return format!("{} Kilobytes", bytes);
	}

	bytes /= 1000;
	if bytes < 1000 {
		return format!("{} Megabytes", bytes);
	}

	bytes /= 1000;

	format!("{} Gigabytes", bytes)
}