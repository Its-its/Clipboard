use std::time::{Instant, Duration};

use chrono::Utc;
use clipboard_common::config::Config;
use eframe::{egui, epi};


use crate::{Tab, ReturnedItem, StorageContainer, StorageQuery};




pub struct RecentTab {
	items: Vec<ReturnedItem>,
	// Scroll
	loading_more_items: bool,
	last_called_load_scroll: Instant,
	can_load_more_data: bool, // Can we continue scrolling.
	// Auto-load new data
	last_auto_recent_check: Instant,
}

impl Default for RecentTab {
	fn default() -> Self {
		Self {
			last_called_load_scroll: Instant::now(),
			last_auto_recent_check: Instant::now(),
			loading_more_items: false,
			can_load_more_data: true,
			items: Vec::new()
		}
	}
}


impl Tab for RecentTab {
	fn on_close(&mut self, _frame: &epi::Frame) {
		self.items.clear();
		self.loading_more_items = false;
	}

	fn update(&mut self, ctx: &egui::CtxRef, frame: &epi::Frame, store: &StorageContainer, config: &mut Config) {
		// Initial load of items.
		if self.items.is_empty() && !self.loading_more_items {
			self.loading_more_items = true;

			let new_items = store.query(StorageQuery::Recent {
				limit: config.app.query_return_limit,
				skip: 0
			}).unwrap();

			super::prepend_new_items_into_existing(&mut self.items, new_items, frame);

			self.loading_more_items = false;
		}

		// Auto Load new copies every 5 seconds.
		if !self.items.is_empty() && self.last_auto_recent_check.elapsed() >= Duration::from_secs(5) {
			let new_items_count = store.count_the_recents_newer_than(self.items[0].timestamp.timestamp_millis() as usize).unwrap();

			if new_items_count != 0 {
				let new_items = store.query(StorageQuery::Recent {
					limit: new_items_count,
					skip: 0
				}).unwrap();

				super::prepend_new_items_into_existing(&mut self.items, new_items, frame);

				self.last_auto_recent_check = Instant::now();
			}
		}


		egui::CentralPanel::default()
		.show(ctx, |ui| {
			egui::ScrollArea::vertical()
			.show_viewport(ui, |ui, rect| {
				let mut removed_data_index: Option<usize> = None;

				let desired_size = egui::vec2(ui.available_width(), 40.0);

				let total_height = self.items.len() as f32 * desired_size.y;

				let now = Utc::now();

				for item in &mut self.items {
					super::display_scroll_row(ui, desired_size, item, config, store, &mut removed_data_index, now);
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


				if self.can_load_more_data {
					if self.loading_more_items {
						ui.allocate_ui_with_layout(desired_size, egui::Layout::centered_and_justified(egui::Direction::LeftToRight), |ui| {
							ui.label(egui::RichText::new("Loading...").color(egui::Rgba::from_rgb(1.0, 1.0, 1.0)).strong().heading());
						});
					}

					if total_height - rect.bottom() < 40.0 * 3.0 && self.last_called_load_scroll.elapsed() > Duration::from_millis(500) {
						self.loading_more_items = true;

						let mut items = store.query(StorageQuery::Recent {
							limit: config.app.query_return_limit,
							skip: self.items.len()
						}).unwrap();

						if items.len() != config.app.query_return_limit {
							self.can_load_more_data = false;
						}

						self.items.append(&mut items);

						self.loading_more_items = false;
						self.last_called_load_scroll = Instant::now();
					}
				} else {
					ui.allocate_ui_with_layout(desired_size, egui::Layout::centered_and_justified(egui::Direction::LeftToRight), |ui| {
						ui.label(egui::RichText::new("No More Items...").color(egui::Rgba::from_rgb(1.0, 1.0, 1.0)).strong().heading());
					});
				}
			});
		});
	}
}