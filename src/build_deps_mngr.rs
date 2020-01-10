use std::collections::HashMap;
use std::process::Command;

use gtk::{TextBufferExt, TextViewExt, DialogFlags, DialogExt, BoxExt, WidgetExt, ToggleButtonExt};
use itertools::Itertools;
use regex::Regex;
use edit_distance::edit_distance;



const BLACKLIST: &[&str] = &["meson"];
const REPLACES: &[(&str, &str)] = &[("valac", "vala")];
fn make_replaces_dict() -> HashMap<&'static str, &'static str> {
	let mut map = HashMap::new();
	for (text, replace) in REPLACES.iter() {
		map.insert(*text, *replace);
	}
	map
}

fn strip_dep(dep: &str) -> String {
	let reg_strip = Regex::new(r"^\s*([^\s]*)").unwrap();
	reg_strip.captures(dep).unwrap().get(1).unwrap().as_str().to_string()
}

fn search_on_eopkg(dep: &str) -> Option<String> {
	println!("Looking for: {:?}", dep);
    let output = Command::new("eopkg")
        .args(&["search", dep])
        .output()
        .expect("failed to execute process");

    let out_regex = Regex::new(r"(?m)^(\S+)\s+-").unwrap();
    let whole_text = console::strip_ansi_codes(std::str::from_utf8(&output.stdout).unwrap()).to_string();
    let res = whole_text.lines()
    	.map(|line|out_regex.captures(line).unwrap()
    		.get(1).unwrap().as_str().to_string())
    	.map(|pkg_name| (pkg_name.clone(), edit_distance(&pkg_name, dep)))
    	.sorted_by(|(_, dist_a),(_, dist_b)| std::cmp::Ord::cmp(dist_a, dist_b))
    	.map(|(pkg_name, _)| pkg_name)
    	.next();

    res
}

fn try_search_dep(dep: String) -> String {
	if let Some(search_res) = search_on_eopkg(&dep) {
		search_res
	}
	else {
		dep + " (not found in repos)"
	}
}

fn filter_and_trans(dep: &str) -> Option<(String,String)> {
	let dep = strip_dep(dep);
	let repl_map = make_replaces_dict();
	if !BLACKLIST.contains(&dep.as_str()) {
		if !dep.is_empty() {
			let reg = Regex::new(r"^(?:lib)?([\w\d-]+?)(?:-?\d(?:\.\d)?)?(?:-dev)?$").unwrap();
			let reg_dev = Regex::new(r"-dev$").unwrap();
			let captures = reg.captures(&dep);
			let res = {
				if let Some(capture) = captures {
					let main_str = capture.get(1).unwrap().as_str();
					if reg_dev.find(&dep).is_some() {
						main_str.to_owned() + "-devel"
					}
					else {
						main_str.to_owned()
					}
				}
				else {
					println!("Regex failed for: {}", dep);
					dep.to_owned()
				}
			};

			let res_as_str = res.as_str();
			let repl_res = repl_map.get(res.as_str()).unwrap_or(&res_as_str);

			Some((dep, repl_res.to_string()))
		}
		else {// If it's empty let's skip all the analisys
			Some((dep.clone(), dep))
		}

	}
	else {
		None
	}
}


// True if modification has been made
pub fn show_build_deps(deps: &mut Vec<String>, parent: &gtk::Window) -> bool {
	let dialog = gtk::MessageDialog::new::<gtk::Window>(Some(parent), DialogFlags::MODAL | DialogFlags::USE_HEADER_BAR, gtk::MessageType::Question, gtk::ButtonsType::OkCancel, "Build dependencies");
	let txt_deps = gtk::TextView::new();
	txt_deps.set_vexpand(true);
	txt_deps.set_hexpand(true);
    let should_trans = gtk::CheckButton::new_with_label("Translate deps");
    should_trans.set_active(true);
    dialog.get_content_area().pack_end(&should_trans, false, false, 0);
    dialog.get_content_area().pack_end(&txt_deps, false, false, 0);
    let buffer = txt_deps.get_buffer().unwrap();
    let org_text = deps.join("\n");
    buffer.set_text(&org_text);
    dialog.show_all();
    let resp = dialog.run();
    let new_text = buffer.get_text(&buffer.get_start_iter(), &buffer.get_end_iter(), false).unwrap().to_string();

    if resp == gtk::ResponseType::Ok {
    	*deps = {
    		println!("{:?}", new_text);
    		if should_trans.get_active() {
    			new_text.lines()
    			.filter_map(filter_and_trans)
    			.map(|(_, new_dep)| new_dep)
    			.map(|dep|if !dep.is_empty(){try_search_dep(dep)} else{dep})
    			.collect()
    		}
    		else {
    			new_text.lines().map(|dep|dep.to_string()).collect()
    		}
    	};
    }

    dialog.destroy();

    org_text != new_text
}