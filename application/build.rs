#[cfg(windows)]
fn main() {
	// let mut res = winres::WindowsResource::new();

	// res.set_icon_with_id("../app.ico", "32512");

	// res.set_language(winapi::um::winnt::MAKELANGID(
	// 	winapi::um::winnt::LANG_ENGLISH,
	// 	winapi::um::winnt::SUBLANG_ENGLISH_US,
	// ));

	// res.compile().unwrap();
}


#[cfg(not(windows))]
fn main() {
	panic!("Non Windows not implemented.")
}