mod vars;
mod guess;

use gtk::{Inhibit, ComboBoxExt, ComboBoxTextExt, ComboBoxText, TreeModelExt, FileChooserExt, TextBufferExt};
use gtk::prelude::*;
use gtk::DialogFlags;
use relm_derive::{Msg, widget};
use relm::{Component, Widget, init, connect, Relm};
use serde::{Serialize, Deserialize};
use std::collections::BTreeMap;
use sha::sha256::ops::digest;
use reqwest::Url;
use regex::Regex;
use crate::vars::*;
use std::path::{PathBuf, Path};
use std::process::Command;
use std::string::ToString;

use self::HeaderMsg::*;
use self::WinMsg::*;



#[derive(Msg)]
pub enum HeaderMsg {
    BtnNew,
    Load,
    BtnFromUrl,
    NewSubtitle(String),
    SetSaved(bool)
}

pub struct HeaderModel {
    subtitle: String,
    is_saved: bool,
    pkg_name: String
}
#[widget]
impl Widget for Header {
    fn model() -> HeaderModel {
        HeaderModel {
            subtitle: "Untitled *".to_string(),
            is_saved: false,
            pkg_name: "Untitled".to_string()
        }
    }

    fn update(&mut self, event: HeaderMsg) {
        fn make_sub(model: &HeaderModel) -> String {
            let pkg_name = {
                if !model.pkg_name.is_empty() {
                    model.pkg_name.clone()
                }
                else {
                    "Untitled".to_string()
                }
            };

            if model.is_saved {
                pkg_name
            }
            else {
                pkg_name + "*"
            }
        }
        match event {
            NewSubtitle(subtitle) => {
                self.model.pkg_name = subtitle;
                self.model.subtitle = make_sub(&self.model);
            }
            SetSaved(is_saved) => {
                self.model.is_saved = is_saved;
                self.model.subtitle = make_sub(&self.model);
            }
            _ => {}
        }
    }

    view! {
        #[name="titlebar"]
        gtk::HeaderBar {
            title: Some("Package Maker"),
            subtitle: Some(&self.model.subtitle),
            show_close_button: true,

            gtk::Button {
                clicked => BtnNew,
                label: "New"
            },
            #[name="load_button"]
            gtk::Button {
                clicked => Load,
                label: "Load",
            },

            gtk::Button {
                clicked => BtnFromUrl,
                label: "From URL"
            }
        }
    }
}

pub struct Model {
    header: Component<Header>,
    pkg_data: PkgData,
    can_start: bool
}

#[derive(Debug, Clone)]
pub struct PkgData {
    name: String,
    version: String,
    release: u16,
    source: String,
    license: String,
    component: String,
    summary: String,
    description: String,
    build_sys: String,
    org_yaml: Option<YamlPkg>,
    file_path: Option<PathBuf>
}

fn ask_for_url(parent: &gtk::Window) -> Option<String> {
    let dialog = gtk::MessageDialog::new::<gtk::Window>(Some(parent), DialogFlags::MODAL | DialogFlags::USE_HEADER_BAR, gtk::MessageType::Question, gtk::ButtonsType::OkCancel, "Please enter the desired URL to analyze");
    let url_entry = gtk::Entry::new();
    dialog.get_content_area().pack_end(&url_entry, false, false, 0);
    dialog.show_all();

    let response = dialog.run();
    let text = url_entry.get_text().expect("get text failed").to_string();
    dialog.destroy();

    if response == gtk::ResponseType::Ok {
        Some(text)
    }
    else {
        None
    }

}

fn ask_for_file(parent: &gtk::Window) -> Option<std::path::PathBuf> {
    let chooser = gtk::FileChooserDialog::with_buttons::<gtk::Window>(Some("Select package.yml"), Some(parent), gtk::FileChooserAction::Open, &[("Open",gtk::ResponseType::Ok)]);
    chooser.show_all();
    let response = chooser.run();
    let opt_file = chooser.get_filename();
    chooser.destroy();

    if response == gtk::ResponseType::Ok {
        opt_file
    }
    else {
        None
    }

}

impl PkgData {
    fn new() -> Self {
        Self {
            name: "".to_string(),
            version: "".to_string(),
            release: 1,
            source: "".to_string(),
            license: "".to_string(),
            component: "".to_string(),
            summary: "".to_string(),
            description: "".to_string(),
            build_sys: "".to_string(),
            org_yaml: None,
            file_path: None

        }
    }

    fn is_filled(&self) -> bool {
        !self.name.is_empty() && !self.version.is_empty() && !self.license.is_empty() && !self.component.is_empty() && !self.summary.is_empty() && !self.description.is_empty() && !self.build_sys.is_empty() && !self.source.is_empty()
    }

    fn join_url_data(&mut self, url_data: &PkgDataUrl) {
        if self.name.is_empty() {
            self.name = url_data.name.clone();
        }

        if url_data.summary.is_some() && self.summary.is_empty() {
            self.summary = url_data.summary.clone().unwrap();
        }

        // For now just clone the summary
        if url_data.summary.is_some() && self.description.is_empty() {
            self.description = url_data.summary.clone().unwrap();
        }

        if url_data.license.is_some() && self.license == "Unknown" {
            self.license = url_data.license.as_ref().unwrap().to_string();
        }

        if url_data.build_sys.is_some() && self.build_sys == "None" {
            self.build_sys = url_data.build_sys.as_ref().unwrap().to_string();
        }

        self.source = url_data.source.clone();
        self.version = url_data.version.clone();
    }
}



fn to_u8s(x: u32) -> [u8;4] {
    let b1 : u8 = ((x >> 24) & 0xff) as u8;
    let b2 : u8 = ((x >> 16) & 0xff) as u8;
    let b3 : u8 = ((x >> 8) & 0xff) as u8;
    let b4 : u8 = (x & 0xff) as u8;
    return [b1, b2, b3, b4]
}

fn calc_sha(source: &str) -> String  {
    let mut buffer = Vec::new();
    reqwest::blocking::get(source).unwrap().copy_to(&mut buffer).unwrap();
    let sha_u32 = digest(&buffer);
    let sha_u8 = [to_u8s(sha_u32[0]), to_u8s(sha_u32[1]), to_u8s(sha_u32[2]), to_u8s(sha_u32[3]), to_u8s(sha_u32[4]), to_u8s(sha_u32[5]), to_u8s(sha_u32[6]), to_u8s(sha_u32[7])].concat();

    hex::encode(&sha_u8[..])
}

#[derive(Debug)]
struct PkgDataUrl {
    name: String,
    version: String,
    source: String,
    summary: Option<String>,
    license: Option<String>,
    build_sys: Option<String>
}

#[derive(Deserialize)]
struct RepoApiLicense {
    spdx_id: String
}

#[derive(Deserialize)]
struct RepoApiCall {
    description: String,
    license: RepoApiLicense
}

fn ask_gh_api_repo(author: &str, repo: &str) -> RepoApiCall{
    let gh_api = Url::parse("https://api.github.com/repos/").unwrap();
    let gh_api = gh_api.join(&(author.to_owned() + "/")).unwrap();
    let gh_api = gh_api.join(repo).unwrap();
    let client = reqwest::blocking::Client::new();

    let api_call_resp = client.get(gh_api).header("User-Agent", "curl/7.37.0").send().unwrap();
    api_call_resp.json().unwrap()
}

fn guess_summary (org_url: &Url) -> Option<String> {
    if let Some(host_str) = org_url.host_str() {
        match host_str {
            "github.com" => {
                let mut segments = org_url.path_segments().unwrap();
                if let Some(repo) = segments.clone().nth(1) {
                    let author_name = segments.nth(0).unwrap();
                    let resp = ask_gh_api_repo(author_name, repo);
                    Some(resp.description)

                }
                else {
                    None
                }
            }
            _ => None
        }
    }
    else {
        None
    }
}


fn guess_license_from_url(org_url: &Url) -> Option<String> {
    if let Some(host_str) = org_url.host_str() {
        match host_str {
            "github.com" => {
                let mut segments = org_url.path_segments().unwrap();
                if let Some(repo) = segments.clone().nth(1) {
                    let author_name = segments.nth(0).unwrap();
                    let resp = ask_gh_api_repo(author_name, repo);
                    Some(update_license_id(resp.license.spdx_id))

                }
                else {
                    None
                }
            }
            _ => None
        }
    }
    else {
        None
    }

}

fn update_license_id(id: String) -> String {
    match id.as_str() {
        "GPL-3.0" => "GPL-3.0-or-later".to_string(),
        _ => id
    }
}

fn from_url(url_str: &str) -> PkgDataUrl {
    let url = Url::parse(url_str).unwrap();
    let url_kind = url_kind_analyze(url_str);
    let url_parser = Regex::new(r"(?P<name>\D\w+)?-?\s*(?P<version>\d+\.?(?:\d+\.)?\d+?)?").unwrap();
    let url_segments = url.path_segments().ok_or_else(|| "cannot be base").unwrap();
    let whole_name = url_segments.clone().last().unwrap();

    let captures = url_parser.captures(whole_name).unwrap();
    let name = {
        let match_str = captures.name("name").map_or("", |reg_match| reg_match.as_str());
        // If there's no name in url then try to get from the third segment in URL
        // e.g: name/_something_/2.3.1.zip
        // Note: This works for Github releases
        if match_str.is_empty() {
            url_segments.clone().nth_back(2).unwrap_or("")
        }
        else {
            match_str
        }
    };

    let (version, summary, license, build_sys) = match url_kind {
        UrlKind::File(_) => {
            let version = captures.name("version").map_or("", |reg_match| reg_match.as_str());
            let summary = guess_summary(&url);
            let (license, build_sys) = crate::guess::try_guess_license_build_sys_from_url(&url);

            (version.to_string(), summary, license, build_sys)
        }
        UrlKind::GitRepo => {
            let version = chrono::Utc::now().format("%Y%m%d%H%M").to_string();
            let summary = guess_summary(&url);
            let license = guess_license_from_url(&url);
            (version, summary, license, None)
        }
    };
    

    println!("{:?}, -> {}, {}, {}", whole_name, name, version, license.clone().unwrap_or("No license found".to_string()));


    PkgDataUrl {
        name: name.to_string(),
        version: version,
        source: url.to_string(),
        summary,
        license,
        build_sys
    }
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
struct YamlPkg {
    name: String,
    version: String,
    release: u16,
    source: Vec<BTreeMap<String, String >>,
    license: String,
    component: String,
    summary: String,
    description: String,
    builddeps: Vec<String>, // This one is actually optional
    setup: String, // This one is actually optional
    build: String, // This one is actually optional
    install: String, // This one is actually optional

    // Optional keys
    #[serde(skip_serializing_if = "Option::is_none")]
    clang: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    extract: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    autodep: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    emul32: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    libsplit: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    optimize: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    rundeps: Option<BTreeMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    replaces: Option<BTreeMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    patterns: Option<BTreeMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    environment: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    networking: Option<bool>,


    // Build steps, optional
    #[serde(skip_serializing_if = "Option::is_none")]
    check: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    profile: Option<String>



}

#[derive(Clone, Copy)]
enum FileKind {
    Zip,
    Other
}

#[derive(Clone, Copy)]
enum UrlKind {
    GitRepo,
    File(FileKind)
}

fn url_kind_analyze(url: &str) -> UrlKind {
    let url = Url::parse(url).unwrap();
    let last_part = url.path_segments().unwrap().nth_back(0).unwrap();
    let opt_ext = Path::new(last_part).extension();

    if let Some(ext) = opt_ext {
        match ext.to_str().unwrap() {
            "zip" => UrlKind::File(FileKind::Zip),
            "git" => UrlKind::GitRepo,
            _ => UrlKind::File(FileKind::Other)
        }
    }
    else {
        // Might be other things, but right now only a Git repo is supported
        UrlKind::GitRepo
    }
}

fn url_format(url: &str, kind: UrlKind) -> String {
    match kind {
        UrlKind::GitRepo => "git|".to_owned() + url,
        UrlKind::File(_) => url.to_string()
    }
}

fn calc_sha_git(url: &str) -> String {
    let url_parsed = Url::parse(url).unwrap();
    let whole_name = url_parsed.path_segments().unwrap().nth_back(0).unwrap();
    let base_name =Path::new(&whole_name).file_name().unwrap();
    let succeed = Command::new("git").args(&["clone", url]).status().unwrap().success();
    if succeed {
        let stdout = Command::new("git").args(&["log", "--format=%H", "-n", "1"]).current_dir(base_name).output().unwrap().stdout;
        std::str::from_utf8(&stdout).unwrap().to_string()
    }
    else {
        "".to_string()
    }
}

fn calc_sha_for(url: &str, kind: UrlKind) -> String {
    match kind {
        UrlKind::File(_) => calc_sha(url),
        UrlKind::GitRepo => calc_sha_git(url)
    }
}

impl Into<YamlPkg> for PkgData {

    fn into(self) -> YamlPkg {
        let mut bmap = BTreeMap::new();
        let url_kind = url_kind_analyze(&self.source);
        let sha = calc_sha_for(&self.source, url_kind);
        let url_formatted = url_format(&self.source, url_kind);
        bmap.insert(url_formatted, sha);

        let empty = ("".to_string(), "".to_string(), "".to_string());
        let (setup_str, build_str, install_str) = match self.build_sys.as_str() {
            "Meson" => ("%meson_configure".to_string(), "%ninja_build".to_string(), "%ninja_install".to_string()),
            "Configure & Make" => ("%configure".to_string(), "%make".to_string(), "%make_install".to_string()),
            "CMake & Make" => ("%cmake".to_string(), "%make".to_string(), "%make_install".to_string()),
            "CMake & Ninja" => ("%cmake_ninja".to_string(), "%ninja_build".to_string(), "%ninja_install".to_string()),
            "Unknown" => {
                if let Some(org_yaml) = self.org_yaml.clone() {
                    (org_yaml.setup, org_yaml.build, org_yaml.install)
                }
                else {
                    empty
                }
            }
            _ => empty
        };



        YamlPkg {
            name: self.name,
            version: self.version,
            release: self.release,
            source: vec![bmap],
            license: self.license,
            component: self.component,
            summary: self.summary,
            description: self.description,
            builddeps: self.org_yaml.clone().map_or(Vec::new(), |yaml| yaml.builddeps),
            setup: setup_str.to_string(),
            build: build_str.to_string(),
            install: install_str.to_string(),

            //Optional Keys
            clang: self.org_yaml.clone().map_or(None, |yaml| yaml.clang),
            extract: self.org_yaml.clone().map_or(None, |yaml| yaml.extract),
            autodep: self.org_yaml.clone().map_or(None, |yaml| yaml.autodep),
            emul32: self.org_yaml.clone().map_or(None, |yaml| yaml.emul32),
            libsplit: self.org_yaml.clone().map_or(None, |yaml| yaml.libsplit),
            optimize: self.org_yaml.clone().map_or(None, |yaml| yaml.optimize),
            rundeps: self.org_yaml.clone().map_or(None, |yaml| yaml.rundeps),
            replaces: self.org_yaml.clone().map_or(None, |yaml| yaml.replaces),
            patterns: self.org_yaml.clone().map_or(None, |yaml| yaml.patterns),
            environment: self.org_yaml.clone().map_or(None, |yaml| yaml.environment),
            networking: self.org_yaml.clone().map_or(None, |yaml| yaml.networking),


            // Build steps, optional
            check: self.org_yaml.clone().map_or(None, |yaml| yaml.check),
            profile: self.org_yaml.map_or(None, |yaml| yaml.profile),

        }
    }
}
impl From<YamlPkg> for PkgData {
    fn from(yaml: YamlPkg) -> Self {
        let build_sys_setup = match yaml.setup.as_str() {
            "%meson_configure" => "Meson",
            "%configure" => "Configure & Make",
            "%cmake" => "CMake & Make",
            "%cmake_ninja" => "CMake & Ninja",
            _ => "Unknown" // Nothing else is supported right now xD
        };

        let build_sys = {
            match build_sys_setup {
                "Meson" | "CMake & Ninja" => {
                    if yaml.build.as_str() == "%ninja_build" && yaml.build.as_str() == "%ninja_install" {
                        build_sys_setup
                    }
                    else {
                        "Unknown"
                    }
                }

                "Configure & Make" | "CMake & Make" => {
                    if yaml.build.as_str() == "%make" && yaml.build.as_str() == "%make_install" {
                            build_sys_setup
                    }
                    else {
                        "Unknown"
                    }
                }

                _ => "Unknown"
            }
        };

        let yaml_copy = yaml.clone();

        let (url_str, _) = yaml.source.first().unwrap().iter().nth(0).unwrap();
        PkgData {
            name: yaml.name,
            version: yaml.version,
            release: yaml.release,
            source: url_str.to_string(),
            license: yaml.license,
            component: yaml.component,
            summary: yaml.summary,
            description: yaml.description,
            build_sys:  build_sys.to_string(),
            org_yaml: Some(yaml_copy),
            file_path: None
        }

    }
}

#[derive(Msg)]
pub enum WinMsg {
    Quit,
    NameChanged(String),
    VersionChanged(String),
    UrlChanged(String),
    LicenseChanged(String),
    ComponentChanged(String),
    BuildSysChanged(String),
    SummaryChanged(String),
    DescriptionChanged,
    New,
    LoadFile,
    FromUrl,
    StartMaking
}

impl Win {
    fn update_descr(&self) {
        let buffer = self.txt_descr.get_buffer().unwrap();
        buffer.set_text(&self.model.pkg_data.description);
    }
}

const RIGHT_COL_PROPORTION: i32 = 10;
#[widget]
impl Widget for Win {
    fn model() -> Model {
        let header = init::<Header>(()).expect("Header");

        Model {
            header,
            pkg_data: PkgData::new(),
            can_start: false
        }
    }

    fn init_view(&mut self) {
        fn fill_combo(cmb: &ComboBoxText, slice_data: &[&str]) {
            for str_data in slice_data.iter() {
                cmb.append(Some(str_data),str_data);
            }

            cmb.set_active_iter(cmb.get_model().unwrap().get_iter_first().as_ref());
        }

        fill_combo(&self.cmb_license, LICENSES);
        fill_combo(&self.cmb_component, COMPONENTS);
        fill_combo(&self.cmb_buildsys, BUILD_SYSS);
        
    }

    fn update(&mut self, event: WinMsg) {

        match event {
            Quit => gtk::main_quit(),
            NameChanged(name) => {
                self.model.header.emit(HeaderMsg::NewSubtitle(name.clone()));
                self.model.header.emit(HeaderMsg::SetSaved(false));
                self.model.pkg_data.name = name.clone();
            },
            VersionChanged(version) => {
                self.model.header.emit(HeaderMsg::SetSaved(false));
                self.model.pkg_data.version = version;
            },
            UrlChanged(url) => {
                self.model.header.emit(HeaderMsg::SetSaved(false));
                self.model.pkg_data.source = url;
            },
            LicenseChanged(license) => {
                self.model.header.emit(HeaderMsg::SetSaved(false));
                self.model.pkg_data.license = license;
            },
            ComponentChanged(comp) => {
                self.model.header.emit(HeaderMsg::SetSaved(false));
                self.model.pkg_data.component = comp;
            },
            BuildSysChanged(build_sys) => {
                self.model.header.emit(HeaderMsg::SetSaved(false));
                self.model.pkg_data.build_sys = build_sys;
            },
            SummaryChanged(summary) => {
                self.model.header.emit(HeaderMsg::SetSaved(false));
                self.model.pkg_data.summary = summary;
            },
            DescriptionChanged => {
                self.model.header.emit(HeaderMsg::SetSaved(false));
                let buffer = self.txt_descr.get_buffer().unwrap();
                self.model.pkg_data.description = buffer.get_text(&buffer.get_start_iter(), &buffer.get_end_iter(), false).unwrap().to_string();
            },
            New => {
                self.model.header.emit(HeaderMsg::SetSaved(false));
                self.model.pkg_data = PkgData::new();
                self.update_descr();
            }
            LoadFile => {
                if let Some(pkg_path) = ask_for_file(&self.window) {
                    let pkg_str = std::fs::read_to_string(&pkg_path).expect("Something went wrong reading package.yml");
                    let pkg_yaml: YamlPkg = serde_yaml::from_str(&pkg_str).expect("Something went wrong parsing package.yml");
                    let mut pkg_data: PkgData = pkg_yaml.into();
                    pkg_data.release += 1; // Update release
                    pkg_data.file_path = Some(pkg_path);
                    self.model.pkg_data = pkg_data;

                    self.update_descr();

                    self.cmb_license.set_active_id(Some(&self.model.pkg_data.license));
                    self.cmb_buildsys.set_active_id(Some(&self.model.pkg_data.build_sys));
                    self.cmb_component.set_active_id(Some(&self.model.pkg_data.component));

                    self.model.header.emit(HeaderMsg::SetSaved(true));
                }
            },
            FromUrl => {
                if let Some(url_str) = ask_for_url(&self.window) {
                    let url_data = from_url(&url_str);
                    self.model.pkg_data.join_url_data(&url_data);

                    // Update Gui
                    self.ent_name.set_text(&self.model.pkg_data.name);
                    self.ent_version.set_text(&self.model.pkg_data.version);
                    self.ent_source.set_text(&self.model.pkg_data.source);
                    self.ent_summary.set_text(&self.model.pkg_data.summary);
                    self.cmb_license.set_active_id(Some(&self.model.pkg_data.license));
                    self.cmb_buildsys.set_active_id(Some(&self.model.pkg_data.build_sys));
                    self.update_descr();
                }
            },
            StartMaking => {
                let yaml: YamlPkg = self.model.pkg_data.clone().into();
                let file_path = {
                    if let Some(file_path) = &self.model.pkg_data.file_path {
                        file_path.clone()
                    }
                    else {
                        let pkg_path = Path::new(&std::env::current_dir().unwrap()).join(self.model.pkg_data.name.clone()).to_path_buf();
                        if !pkg_path.is_dir() {
                            std::fs::create_dir_all(&pkg_path).unwrap();
                        }

                        pkg_path.join("package.yml")
                    }
                };
                serde_yaml::to_writer(std::fs::File::create(file_path).unwrap(),&yaml).unwrap();
                self.model.header.emit(HeaderMsg::SetSaved(true));
            }
        }
        self.model.can_start = self.model.pkg_data.is_filled();
    }

    fn subscriptions(&mut self, relm: &Relm<Self>) {
        let header = &self.model.header;
        connect!(header@BtnNew, relm, New);
        connect!(header@Load, relm, LoadFile);
        connect!(header@BtnFromUrl, relm, FromUrl);

        let buffer = &self.txt_descr.get_buffer().unwrap();
        connect!(relm, buffer, connect_changed(_), DescriptionChanged);
    }

    view! {
        #[name="window"]
        gtk::Window {
            titlebar: Some(self.model.header.widget()),

            #[name="app"]
            gtk::Grid {
                row_homogeneous: true,
                column_homogeneous: true,
                gtk::Label {
                    markup: "<b>Name</b>",
                },
                #[name="ent_name"]
                gtk::Entry {
                    text: &self.model.pkg_data.name,
                    changed(entry) => NameChanged(entry.get_text().expect("get_text failed").to_string()),
                    cell : {
                        width: RIGHT_COL_PROPORTION
                    }
                },
                gtk::Label {
                    markup: "<b>Version</b>",
                    cell: {
                        top_attach: 1,
                        left_attach: 0,
                        width: 1
                    }
                },
                #[name="ent_version"]
                gtk::Entry {
                    text: &self.model.pkg_data.version,
                    changed(entry) => VersionChanged(entry.get_text().expect("get_text failed").to_string()),
                    cell: {
                        top_attach: 1,
                        left_attach: 1,
                        width: RIGHT_COL_PROPORTION
                    }
                },
                gtk::Label {
                    markup: "<b>URL</b>",
                    cell: {
                        top_attach: 2,
                        left_attach: 0,
                    }
                },
                #[name="ent_source"]
                gtk::Entry {
                    text: &self.model.pkg_data.source,
                    changed(entry) => UrlChanged(entry.get_text().expect("get_text failed").to_string()),
                    cell: {
                        top_attach: 2,
                        left_attach: 1,
                        width: RIGHT_COL_PROPORTION
                    }
                },
                gtk::Label {
                    markup: "<b>License</b>",
                    cell: {
                        top_attach: 3,
                        left_attach: 0
                    }
                },
                #[name="cmb_license"]
                gtk::ComboBoxText {
                    changed(combo) => LicenseChanged(combo.get_active_text().expect("get_active_text failed").to_string()),
                    cell: {
                        top_attach: 3,
                        left_attach: 1,
                        width: RIGHT_COL_PROPORTION
                    },
                },
                gtk::Label {
                    markup: "<b>Component</b>",
                    cell: {
                        top_attach: 4,
                        left_attach: 0
                    }
                },
                #[name="cmb_component"]
                gtk::ComboBoxText {
                    changed(combo) => ComponentChanged(combo.get_active_text().expect("get_active_text failed").to_string()),
                    cell: {
                        top_attach: 4,
                        left_attach: 1,
                        width: RIGHT_COL_PROPORTION
                    },
                },
                gtk::Label {
                    markup: "<b>Build Sys</b>",
                    cell: {
                        top_attach: 5,
                        left_attach:0
                    }
                },
                #[name="cmb_buildsys"]
                gtk::ComboBoxText {
                    changed(combo) => BuildSysChanged(combo.get_active_text().expect("get_active_text failed").to_string()),
                    cell: {
                        top_attach: 5,
                        left_attach: 1,
                        width: RIGHT_COL_PROPORTION
                    }
                },
                gtk::Label {
                    markup: "<b>Summary</b>",
                    cell: {
                        top_attach: 6,
                        left_attach: 0
                    }
                },
                #[name="ent_summary"]
                gtk::Entry { // Summary
                    text: &self.model.pkg_data.summary,
                    changed(entry) => SummaryChanged(entry.get_text().expect("get_text failed").to_string()),
                    cell: {
                        top_attach: 6,
                        left_attach: 1,
                        width: RIGHT_COL_PROPORTION
                    }
                },
                gtk::Label {
                    markup: "<b>Description</b>",
                    cell: {
                        top_attach: 7,
                        left_attach: 0
                    }
                },
                #[name="txt_descr"]
                gtk::TextView {
                    wrap_mode: gtk::WrapMode::Word,
                    cell: {
                        top_attach: 7,
                        left_attach: 1,
                        width: RIGHT_COL_PROPORTION
                    }
                },
                gtk::Button {
                    label: "Start",
                    sensitive: self.model.can_start,
                    clicked => StartMaking,
                    cell: {
                        top_attach: 8,
                        left_attach: RIGHT_COL_PROPORTION
                    }
                }
            },
            delete_event(_, _) => (Quit, Inhibit(false)),
        }
    }
}

fn main() {
    Win::run(()).expect("Window::run");
}
