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

const GPL_V_2_OR_LATER_SNIPPET: &str = "This program is free software; you can redistribute it and/or
modify it under the terms of the GNU General Public License as
published by the Free Software Foundation; either version 2 of the
License, or (at your option) any later version.";

fn guess_license_str(input: &str) -> String {
	let input = input.trim();

	let licenses = [
		("GPL-3.0-only", GPL_V_3_SNIPPET),
		("GPL-2.0-or-later", GPL_V_2_OR_LATER_SNIPPET)
	];

	for (l_name, snippet) in licenses.iter() {
		if edit_distance(&input[..snippet.len()], snippet) < MAX_DIST {
			return l_name.to_string()
		}
	}
	
	
	"Unknown".to_string()
	
}

// This trait is a way to circumvent how Rust treats generic associated lifetimes
trait Gat<'a> {
	type FileRead: Read + 'a;

	fn get_file(&'a mut self, id: Self::ArchiveRef) -> Option<Self::FileRead> where Self: Archive;
}

type ArchRef<T> = <T as Archive>::ArchiveRef;

impl<'a, R: Read + Seek> Gat<'a> for ZipArchive<R> {
	type FileRead = ZipFile<'a>;


	fn get_file(&'a mut self, id: ArchRef<Self>) -> Option<Self::FileRead> {
		if let Ok(file) = self.by_index(id) {
			Some(file)
		}
		else {
			None
		}
	}
}

trait Archive {
	type ArchiveRef;

	fn search_like(& mut self, names: &[&str]) -> Option<Self::ArchiveRef>;
}

impl<R: Read + Seek> Archive for ZipArchive<R> {
	type ArchiveRef = usize;

	// Search in zip_arc for a file whose name is like one of names, case insensitve
	// NOTE: names must be in lowercase
	fn search_like(&mut self, names: &[&str]) -> Option<Self::ArchiveRef> {
		let max = self.len();
		let mut index_opt: Option<usize> = None;
		for i in 0..max {
			let zip_file = self.by_index(i).unwrap();
			let file_name = zip_file.sanitized_name().file_name().unwrap().to_str().unwrap().to_lowercase();
			for name in names.iter() {
				if name == &file_name {
					index_opt = Some(i)
				}
			}
		}

		if let Some(ind_val) = index_opt {
			Some(ind_val)
		}

		else {
			None
		}
	}
}

fn guess_license_from_archive_file<R: Read>(input: &mut R) -> String {

	let mut license_str = String::new();
	input.read_to_string(&mut license_str).unwrap();

	guess_license_str(&license_str)
}

fn guess_license_from_archive<'a, A: Archive + Gat<'a>>(mut pkg_zip: &'a mut A) -> Option<String> {
	let a = {
		let b1 = &mut pkg_zip;
		b1.search_like(&["license", "copying"])
	};
	if let Some(license_file) = a {
		Some(guess_license_from_archive_file::<A::FileRead>(&mut pkg_zip.get_file(license_file).unwrap()))
	}
	else {
		None
	}

}

fn guess_build_sys_from_zip<'a, A: Archive>(pkg_zip: &'a mut A) -> Option<String> {
	if let Some(_) = pkg_zip.search_like(&["meson"]) {
		Some("Meson".to_string())
	}
	else if let Some(_) = pkg_zip.search_like(&["configure"]) {
		Some("Configure & Make".to_string())
	}
	else if let Some(_) = pkg_zip.search_like(&["cmakelists"]) {
		Some("CMake & Make".to_string())
	}
	else {
		None
	}
}

pub fn try_guess_license_build_sys_from_url(url: &Url) -> (Option<String>, Option<String>) {
	let mut buffer = Vec::new();


	let filename = url.path_segments().unwrap().last().unwrap();
	reqwest::blocking::get(url.as_str()).unwrap().copy_to(&mut buffer).unwrap();
	let ext = std::path::Path::new(filename).extension().unwrap().to_str().unwrap();
	let mut cursor = std::io::Cursor::new(buffer);
	

	match ext {
		"zip" => {
			let mut pkg_zip = zip::read::ZipArchive::new(&mut cursor).unwrap();
			(guess_license_from_archive(&mut pkg_zip), guess_build_sys_from_zip(&mut pkg_zip))
		}
		_ => (None, None)
	}
}