use std::sync::Arc;

use anyhow::Result;
use chrono::{Utc, TimeZone};
use rusqlite::{Connection, params, Row, OptionalExtension};
use serde::{Serialize, Deserialize};
use sha2::{Sha256, Digest};

use crate::config::Config;


#[derive(Clone)]
pub struct StorageContainer(Arc<Connection>);

// TODO: I KNOW I KNOW
#[allow(clippy::non_send_fields_in_send_ty)]
unsafe impl Send for StorageContainer {}


impl StorageContainer {
	pub fn open(path: &str) -> Result<Self> {
		let conn = Connection::open(path)?;

		init_database(&conn)?;

		Ok(Self(Arc::new(conn)))
	}


	pub fn query(&self, value: StorageQuery) -> Result<Vec<ReturnedItem>> {
		match value {
			StorageQuery::Favorites => {
				let sql = r#"
					SELECT
						recent.id,
						recent.date,
						data.is_starred,
						data.type_of,
						data.text_data,
						data.image_thumb_data,
						data.id
					FROM recent
					INNER JOIN data ON
						data.id = recent.row_id
					WHERE data.is_starred = 1
					ORDER BY recent.id DESC
				"#;

				let mut stmt = self.0.prepare(sql)?;

				let iter = stmt.query_map(
					[],
					|r| Ok(ReturnedItem {
						recent_id: r.get(0)?,
						timestamp: Utc.timestamp_millis(r.get(1)?),
						is_favorite: r.get(2)?,
						value: ReturnedItemType::from_sql(r.get(3)?, r.get(4)?, r.get(5)?),
						data_id: r.get(6)?,
				}))?;

				Ok(iter.collect::<std::result::Result<Vec<_>, _>>()?)
			}

			StorageQuery::Recent { limit, skip } => {
				let sql = format!(r#"
					SELECT
						recent.id,
						recent.date,
						data.is_starred,
						data.type_of,
						data.text_data,
						data.image_thumb_data,
						data.id
					FROM recent
					INNER JOIN data ON
						data.id = recent.row_id
					WHERE 1
					ORDER BY recent.id DESC
					LIMIT {}
					OFFSET {}
				"#, limit, skip);

				let mut stmt = self.0.prepare(&sql)?;

				let iter = stmt.query_map(
					[],
					|r| Ok(ReturnedItem {
						recent_id: r.get(0)?,
						timestamp: Utc.timestamp_millis(r.get(1)?),
						is_favorite: r.get(2)?,
						value: ReturnedItemType::from_sql(r.get(3)?, r.get(4)?, r.get(5)?),
						data_id: r.get(6)?,
				}))?;

				Ok(iter.collect::<std::result::Result<Vec<_>, _>>()?)
			}

			StorageQuery::Search { value } => {
				// TODO: Query works but I don't like it. Currently will remove newest instead of oldest duplicates from results.
				let sql = if value.contains('%') || value.contains('_') {
					let mut escape_char = '\\';

					if value.contains(escape_char) {
						for car in [ '!', '@', '#', '$', '^', '&', '*', '-', '=', '+', '|', '~', '`', '/', '?', '>', '<', ',' ] {
							if !value.contains(car) {
								escape_char = car;
								break;
							}
						}
					}

					format!(
						r#"
							SELECT
								recent.id,
								recent.date,
								data.is_starred,
								data.type_of,
								data.text_data,
								data.image_thumb_data,
								data.id
							FROM data
							INNER JOIN recent
								ON recent.row_id = data.id
							WHERE
								text_data LIKE '%{}%' ESCAPE '{}'
							GROUP BY recent.row_id
							ORDER BY recent.id DESC
						"#,
						value.replace("%", &format!("{}%", escape_char)).replace("_", &format!("{}_", escape_char)),
						escape_char
					)
				} else {
					format!(r#"
						SELECT
							recent.id,
							recent.date,
							data.is_starred,
							data.type_of,
							data.text_data,
							data.image_thumb_data,
							data.id
						FROM data
						INNER JOIN recent
							ON recent.row_id = data.id
						WHERE
							text_data LIKE '%{}%'
						GROUP BY recent.row_id
						ORDER BY recent.id DESC
					"#, value)
				};

				let mut stmt = self.0.prepare(&sql)?;

				let iter = stmt.query_map(
					[],
					|r| Ok(ReturnedItem {
						recent_id: r.get(0)?,
						timestamp: Utc.timestamp_millis(r.get(1)?),
						is_favorite: r.get(2)?,
						value: ReturnedItemType::from_sql(r.get(3)?, r.get(4)?, r.get(5)?),
						data_id: r.get(6)?,
				}))?;

				Ok(iter.collect::<std::result::Result<Vec<_>, _>>()?)
			}
		}
	}

	pub fn add_text(&self, text_data: String, html_data: Option<String>, config: &Config) -> Result<()> {
		if text_data.len() > config.stores.text.max_size * 1000 * 1000 { // B -> KB -> MB
			log::info!(target: "clipboard_listener", "[add_text]: Text Length {}MB > Max Length {}MB", text_data.len() / 1000 / 1000, config.stores.text.max_size);
			return Ok(());
		}

		let hash = Sha256::digest(&text_data)
			.iter()
			.map(|v| format!("{:02x}", v))
			.collect::<String>();

		if let Some(v) = self.get_data_from_hash(&hash)? {
			// Already exists?
			let recent = self.get_most_recent_data(v.id)?;
			let recent_items_after_previous = self.count_the_recents_newer_than(recent.date)?;

			let current_date = Utc::now().timestamp_millis() as usize;
			let minutes_ago = (current_date - recent.date) / 1000 / 60;

			// If it's been 1 hour AND we've surpassed 30 new recents.
			if minutes_ago > 60 && recent_items_after_previous > 30 {
				self.insert_recent(&LastCopied {
					id: 0,
					row_id: v.id,
					date: current_date
				})?;
			}
		} else {
			self.0.execute(
				r#"INSERT INTO data (hash, type_of, text_size, text_data, html_size, html_data) VALUES (?1, ?2, ?3, ?4, ?5, ?6)"#,
				params![ hash, 0, text_data.len(), text_data, html_data.as_deref().map(|v| v.len()), html_data ]
			)?;

			let data = self.get_data_from_hash(&hash)?.unwrap();

			self.insert_recent(&LastCopied {
				id: 0,
				row_id: data.id,
				date: Utc::now().timestamp_millis() as usize
			})?;
		}

		Ok(())
	}

	pub fn add_image(&self, image_data: Vec<u8>, image_thumb_data: Option<Vec<u8>>, config: &Config) -> Result<()> {
		if image_data.len() > config.stores.image.max_size * 1000 * 1000 { // B -> KB -> MB
			log::info!(target: "clipboard_listener", "[add_image]: Image Length {}MB > Max Length {}MB", image_data.len() / 1000 / 1000, config.stores.image.max_size);
			return Ok(());
		}

		let hash = Sha256::digest(&image_data)
			.iter()
			.map(|v| format!("{:02x}", v))
			.collect::<String>();

		if let Some(stored_data) = self.get_data_from_hash(&hash)? {
			// Already exists?
			let recent = self.get_most_recent_data(stored_data.id)?;
			let recent_items_after_previous = self.count_the_recents_newer_than(recent.date)?;

			let current_date = Utc::now().timestamp_millis() as usize;
			let minutes_ago = (current_date - recent.date) / 1000 / 60;

			// If it's been 1 hour AND we've surpassed 30 new recents.
			if minutes_ago > 60 && recent_items_after_previous > 30 {
				self.insert_recent(&LastCopied {
					id: 0,
					row_id: stored_data.id,
					date: current_date
				})?;
			}
		} else {
			self.0.execute(
				r#"INSERT INTO data (hash, type_of, image_size, image_data, image_thumb_size, image_thumb_data) VALUES (?1, ?2, ?3, ?4, ?5, ?6)"#,
				params![ hash, 1, image_data.len(), image_data, image_thumb_data.as_deref().map(|v| v.len()), image_thumb_data ]
			)?;

			let stored_data = self.get_data_from_hash(&hash)?.unwrap();

			self.insert_recent(&LastCopied {
				id: 0,
				row_id: stored_data.id,
				date: Utc::now().timestamp_millis() as usize
			})?;
		}

		Ok(())
	}

	pub fn set_favorite(&self, index: usize, value: bool) -> Result<usize> {
		Ok(self.0.execute(
			r#"UPDATE data SET is_starred = ?1 WHERE id = ?2"#,
			params![value, index]
		)?)
	}

	pub fn delete(&self, index: usize) -> Result<usize> {
		let deleted = self.0.execute(
			r#"DELETE FROM data WHERE id = ?1"#,
			params![index]
		)?;

		if deleted != 0 {
			Ok(self.0.execute(
				r#"DELETE FROM recent WHERE row_id = ?1"#,
				params![index]
			)?)
		} else {
			Ok(0)
		}
	}

	pub fn clear_database(&self) -> Result<usize> {
		let data_deleted = self.0.execute(
			r#"DELETE FROM data WHERE 1"#,
			[]
		)?;

		let recent_deleted = self.0.execute(
			r#"DELETE FROM recent WHERE 1"#,
			[]
		)?;

		Ok(data_deleted + recent_deleted)
	}

	pub fn compute_total_size(&self) -> Result<usize> {
		let mut stmt = self.0.prepare("SELECT text_size FROM data WHERE 1")?;

		let iter = stmt.query_map([], |v| v.get::<_, usize>(0))?;

		Ok(iter.sum::<rusqlite::Result<usize>>()?)
	}

	pub fn get_image(&self, data_id: usize) -> Result<Vec<u8>> {
		Ok(self.0.query_row(
			r#"SELECT image_data FROM data WHERE id = ?1 LIMIT 1"#,
			params![data_id],
			|v| v.get(0)
		)?)
	}


	fn insert_recent(&self, value: &LastCopied) -> Result<usize> {
		Ok(self.0.execute(
			r#"INSERT INTO recent (row_id, date) VALUES (?1, ?2)"#,
			params![ value.row_id, value.date ]
		)?)
	}


	fn get_data_from_hash(&self, hash: &str) -> Result<Option<CopiedData>> {
		Ok(self.0.query_row(
			r#"SELECT * FROM data WHERE hash = ?1 LIMIT 1"#,
			params![hash],
			CopiedData::from_row
		).optional()?)
	}

	fn get_most_recent_data(&self, data_id: usize) -> Result<LastCopied> {
		Ok(self.0.query_row(
			r#"SELECT * FROM recent WHERE row_id = ?1 ORDER BY id DESC LIMIT 1"#,
			params![data_id],
			LastCopied::from_row
		)?)
	}

	fn get_most_recent_data_list(&self, data_id: usize) -> Result<Vec<LastCopied>> {
		let mut stmt = self.0.prepare(&format!("SELECT * FROM recent WHERE row_id = '{}' ORDER BY id DESC LIMIT 1", data_id))?;

		let iter = stmt.query_map([], LastCopied::from_row)?;

		Ok(iter.collect::<std::result::Result<Vec<_>, _>>()?)
	}

	pub fn count_the_recents_newer_than(&self, time: usize) -> Result<usize> {
		Ok(self.0.query_row(
			r#"SELECT COUNT(*) FROM recent WHERE date > ?1"#,
			params![time],
			|v| v.get(0)
		)?)
	}
}

fn init_database(conn: &Connection) -> Result<()> {
	conn.execute(r#"
		CREATE TABLE IF NOT EXISTS data (
			id					INTEGER NOT NULL,
			hash				TEXT NOT NULL UNIQUE,
			is_starred			BOOLEAN NOT NULL DEFAULT 0,

			type_of				TINYINT NOT NULL,

			text_size			INTEGER,
			text_data			TEXT,

			html_size			INTEGER,
			html_data			TEXT,

			image_size			INTEGER,
			image_data			TEXT,
			image_thumb_size	INTEGER,
			image_thumb_data	TEXT,

			PRIMARY KEY("id")
		)
		"#,
		[]
	)?;

	conn.execute(r#"
		CREATE TABLE IF NOT EXISTS recent (
			id		INTEGER NOT NULL,
			row_id	INTEGER NOT NULL,
			date	INTEGER NOT NULL,

			PRIMARY KEY("id")
		)
		"#,
		[]
	)?;

	Ok(())
}


pub enum StorageQuery {
	Recent {
		limit: usize,
		skip: usize,
	},

	Search {
		value: String,
	},

	Favorites
}




// Storage Tables

#[derive(Serialize, Deserialize)]
pub struct CopiedData {
	pub id: usize,
	pub hash: String,
	pub is_starred: bool,

	pub type_of: u8,

	// Text
	pub text_size: Option<usize>,
	pub text_data: Option<String>,

	// Html
	pub html_size: Option<usize>,
	pub html_data: Option<String>,

	// Image
	pub image_size: Option<usize>,
	pub image_data: Option<Vec<u8>>, // TODO: Blobify
	pub image_thumb_size: Option<usize>,
	pub image_thumb_data: Option<Vec<u8>>,
}

impl CopiedData {
	pub fn from_row(row: &Row) -> rusqlite::Result<Self> {
		Ok(Self {
			id: row.get(0)?,
			hash: row.get(1)?,
			is_starred: row.get(2)?,

			type_of: row.get(3)?,

			text_size: row.get(4)?,
			text_data: row.get(5)?,

			html_size: row.get(6)?,
			html_data: row.get(7)?,

			image_size: row.get(8)?,
			image_data: row.get(9)?,
			image_thumb_size: row.get(10)?,
			image_thumb_data: row.get(11)?,
		})
	}
}



#[derive(Serialize, Deserialize)]
pub struct LastCopied {
	pub id: usize,
	pub row_id: usize,
	pub date: usize,
}

impl LastCopied {
	pub fn from_row(row: &Row) -> rusqlite::Result<Self> {
		Ok(Self {
			id: row.get(0)?,
			row_id: row.get(1)?,
			date: row.get(2)?
		})
	}
}


//

// type_of
//   0 - Text
//   1 - Image


pub struct ReturnedItem {
	pub data_id: usize,
	pub value: ReturnedItemType,
	pub is_favorite: bool,

	pub recent_id: usize,
	pub timestamp: chrono::DateTime<chrono::Utc>,
}

pub enum ReturnedItemType {
	Text(String),
	Thumb(Vec<u8>),
	ThumbTextureId(u64)
}

impl ReturnedItemType {
	pub fn from_sql(type_of: usize, text_value: Option<String>, thumb_value: Option<Vec<u8>>) -> Self {
		match type_of {
			0 => Self::Text(text_value.unwrap()),
			1 => Self::Thumb(thumb_value.unwrap()),
			_ => panic!("Invalid Type Of Value Found! Value {}", type_of)
		}
	}
}