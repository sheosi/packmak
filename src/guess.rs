use zip::read::ZipFile;
use std::io::Read;
use std::io::Seek;

use edit_distance::edit_distance;
use zip::read::{ZipArchive};
use reqwest::Url;



const MAX_DIST: usize = 20;
const GPL_V_3_SNIPPET: &str  = "GNU GENERAL PUBLIC LICENSE
                       Version 3, 29 June 2007

 Copyright (C) 2007 Free Software Foundation, Inc. <http://fsf.org/>
 Everyone is permitted to copy and distribute verbatim copies
 of this license document, but changing it is not allowed.

                            Preamble

  The GNU General Public License is a free, copyleft license for
software and other kinds of works.";

fn guess_license_str(input: &str) -> String {
	let input = input.trim();
	let gpl3_dist = edit_distance(&input[..GPL_V_3_SNIPPET.len()], GPL_V_3_SNIPPET);
	if gpl3_dist < MAX_DIST {
		"GPL-3.0".to_string()
	}
	else {
		"Unknown".to_string()
	}
}

fn guess_license_from_zipfile(input: &mut ZipFile) -> String {

	let mut license_str = String::new();
	input.read_to_string(&mut license_str).unwrap();

	guess_license_str(&license_str)
}

// Search in zip_arc for a file whose name is like one of names, case insensitve
// NOTE: names must be in lowercase
fn search_for_something_like<R: Read + Seek>(zip_arc: &mut ZipArchive<R>, names: &[&str]) -> Option<usize>{
	let max = zip_arc.len();

	for i in 0..max {
		let zip_file = zip_arc.by_index(i).unwrap();
		let file_name = zip_file.sanitized_name().file_name().unwrap().to_str().unwrap().to_lowercase();
		for name in names.iter() {
			if name == &file_name {
				return Some(i)
			}
		}
	}

	None
}

fn guess_license_from_zip<R: Read + Seek>(pkg_zip: &mut ZipArchive<R>) -> Option<String> {
	if let Some(license_index) = search_for_something_like(pkg_zip, &["license", "copying"]) {
		Some(guess_license_from_zipfile(&mut pkg_zip.by_index(license_index).unwrap()))
	}
	else {
		None
	}

}

fn guess_build_sys_from_zip<R: Read + Seek>(pkg_zip: &mut ZipArchive<R>) -> Option<String> {
	if let Some(_) = search_for_something_like(pkg_zip, &["meson"]) {
		Some("Meson".to_string())
	}
	else {
				println!("IDK");
		None
	}
}

pub fn try_guess_license_build_sys_from_url(url: &Url) -> (Option<String>, Option<String>) {
	let mut buffer = Vec::new();


	let filename = url.path_segments().unwrap().last().unwrap();
	reqwest::blocking::get(url.as_str()).unwrap().copy_to(&mut buffer).unwrap();
	let ext = std::path::Path::new(filename).extension().unwrap().to_str().unwrap();

	let mut cursor = std::io::Cursor::new(buffer);
	let mut pkg_zip = zip::read::ZipArchive::new(&mut cursor).unwrap();

	match ext {
		"zip" => (guess_license_from_zip(&mut pkg_zip), guess_build_sys_from_zip(&mut pkg_zip)),
		_ => (None, None)
	}
}