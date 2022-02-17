use chrono::Utc;
use eframe::{egui, epi};
use log::error;


use crate::{Tab, ReturnedItem, StorageContainer, StorageQuery, Config};


#[derive(Default)]
pub struct StarredTab {
	items: Vec<ReturnedItem>,
	fetching_items: bool
}

impl StarredTab {
	pub fn fetch(&mut self, frame: &epi::Frame, store: &StorageContainer) {
		if self.fetching_items {
			return;
		}

		self.items.clear();

		self.fetching_items = true;

		match store.query(StorageQuery::Favorites) {
			Ok(new_items) => super::prepend_new_items_into_existing(&mut self.items, new_items, frame),
			Err(e) => error!(target: "clipboard_gui", "{:?}", e),
		}

		self.fetching_items = false;
	}
}

impl Tab for StarredTab {
	fn on_open(&mut self, frame: &epi::Frame, store: &StorageContainer, _config: &mut Config) {
		self.fetch(frame, store);
	}

	fn update(&mut self, ctx: &egui::CtxRef, _frame: &epi::Frame, store: &StorageContainer, config: &mut Config) {
		egui::CentralPanel::default()
		.show(ctx, |ui| {
			egui::ScrollArea::vertical()
			.show_rows(ui, 40.0, self.items.len(), |ui, viewing| {
				let mut removed_data_index: Option<usize> = None;

				let desired_size = egui::vec2(ui.available_width(), 40.0);

				let now = Utc::now();

				for index in viewing {
					super::display_scroll_row(ui, desired_size, &mut self.items[index], config, store, &mut removed_data_index, now);
					ui.separator();
				}

				// If you clicked the Remove Button
				if let Some(data_id) = removed_data_index {
					// Remove all items in view list.
					for index in self.items.len() - 1..=0 {
						if self.items[index].data_id == data_id {
							self.items.remove(index);
						}
					}

					store.delete(data_id).unwrap();
				}

				if self.fetching_items {
					ui.allocate_ui_with_layout(desired_size, egui::Layout::centered_and_justified(egui::Direction::LeftToRight), |ui| {
						ui.label(egui::RichText::new("Loading...").color(egui::Rgba::from_rgb(1.0, 1.0, 1.0)).strong().heading());
					});
				}

				if self.items.is_empty() {
					ui.allocate_ui_with_layout(desired_size, egui::Layout::centered_and_justified(egui::Direction::LeftToRight), |ui| {
						ui.label(egui::RichText::new("⭐ something :)").color(egui::Rgba::from_rgb(1.0, 1.0, 1.0)).strong().heading());
					});
				}
			});
		});
	}
}