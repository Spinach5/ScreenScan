//! 界面字体：系统字体扫描、自定义字体加载，以及 CJK 字形回退。

use std::collections::HashSet;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::OnceLock;
use std::sync::RwLock;

use egui::FontData;
use egui::FontDefinitions;
use egui::FontFamily;

use crate::util::path;

/// 用户选择的界面字体。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FontChoice {
    pub family: String,
    pub path: PathBuf,
    pub index: u32,
}

/// 扫描系统字体库得到的候选项。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FontEntry {
    pub family: String,
    pub path: PathBuf,
    pub index: u32,
}

impl FontEntry {
    pub fn choice(&self) -> FontChoice {
        FontChoice {
            family: self.family.clone(),
            path: self.path.clone(),
            index: self.index,
        }
    }
}

static SELECTED: OnceLock<RwLock<Option<FontChoice>>> = OnceLock::new();

fn selected() -> &'static RwLock<Option<FontChoice>> {
    SELECTED.get_or_init(|| RwLock::new(None))
}

pub fn set_choice(choice: Option<FontChoice>) {
    if let Ok(mut guard) = selected().write() {
        *guard = choice;
    }
}

pub fn choice() -> Option<FontChoice> {
    selected().read().ok().and_then(|guard| guard.clone())
}

/// 按优先级尝试的系统中文字体（覆盖 Linux / macOS / Windows）。
/// 单字体文件优先，避免 TTC 集合索引差异。
const CJK_CANDIDATES: &[&str] = &[
    // Linux
    "/usr/share/fonts/truetype/droid/DroidSansFallbackFull.ttf",
    "/usr/share/fonts/truetype/winfonts/simhei.ttf",
    "/usr/share/fonts/truetype/winfonts/NotoSansSC-VF.ttf",
    "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
    "/usr/share/fonts/truetype/wqy/wqy-microhei.ttc",
    "/usr/share/fonts/truetype/wqy/wqy-zenhei.ttc",
    // macOS
    "/System/Library/Fonts/PingFang.ttc",
    "/System/Library/Fonts/STHeiti Light.ttc",
    "/System/Library/Fonts/Hiragino Sans GB.ttc",
    // Windows
    "C:\\Windows\\Fonts\\msyh.ttc",
    "C:\\Windows\\Fonts\\msyh.ttf",
    "C:\\Windows\\Fonts\\simhei.ttf",
    "C:\\Windows\\Fonts\\simsun.ttc",
];

fn cjk_font() -> Option<&'static Arc<FontData>> {
    static FONT: OnceLock<Option<Arc<FontData>>> = OnceLock::new();
    FONT.get_or_init(|| {
        for path in CJK_CANDIDATES {
            match std::fs::read(path) {
                Ok(bytes) => {
                    log::info!("加载中文字体: {path}");
                    return Some(Arc::new(FontData::from_owned(bytes)));
                }
                Err(_) => continue,
            }
        }
        log::warn!("未找到系统中文字体，中文界面文字可能无法显示");
        None
    })
    .as_ref()
}

/// 在 egui Context 上安装界面字体：自定义字体优先，CJK 字体作为回退。
pub fn install(ctx: &egui::Context) {
    let mut defs = FontDefinitions::default();

    if let Some(choice) = choice() {
        match std::fs::read(&choice.path) {
            Ok(bytes) => {
                let mut data = FontData::from_owned(bytes);
                data.index = choice.index;
                defs.font_data
                    .insert("ui_selected".to_owned(), Arc::new(data));
                defs.families
                    .entry(FontFamily::Proportional)
                    .or_default()
                    .insert(0, "ui_selected".to_owned());
            }
            Err(e) => log::warn!("加载界面字体失败 {}: {e}", choice.path.display()),
        }
    }

    if let Some(font) = cjk_font() {
        defs.font_data.insert("cjk".to_owned(), font.clone());
        for family in [FontFamily::Proportional, FontFamily::Monospace] {
            defs.families
                .entry(family)
                .or_default()
                .push("cjk".to_owned());
        }
    }

    ctx.set_fonts(defs);
}

/// 扫描系统字体库，返回按族名排序的字体列表（同族只保留一个条目）。
pub fn scan_system_fonts() -> Vec<FontEntry> {
    let mut entries = Vec::new();
    let mut seen = HashSet::new();
    for dir in font_dirs() {
        collect_fonts(&dir, 0, &mut entries, &mut seen);
    }
    entries.sort_by_key(|entry| entry.family.to_lowercase());
    log::info!("扫描到 {} 个系统字体族", entries.len());
    entries
}

fn font_dirs() -> Vec<PathBuf> {
    let home = path::home_dir();
    let mut dirs = Vec::new();

    #[cfg(target_os = "linux")]
    {
        dirs.extend([
            PathBuf::from("/usr/share/fonts"),
            PathBuf::from("/usr/local/share/fonts"),
            home.join(".fonts"),
            home.join(".local/share/fonts"),
        ]);
    }

    #[cfg(target_os = "macos")]
    {
        dirs.extend([
            PathBuf::from("/System/Library/Fonts"),
            PathBuf::from("/Library/Fonts"),
            home.join("Library/Fonts"),
        ]);
    }

    #[cfg(target_os = "windows")]
    {
        if let Some(windir) = std::env::var_os("WINDIR") {
            dirs.push(PathBuf::from(windir).join("Fonts"));
        } else {
            dirs.push(PathBuf::from("C:\\Windows\\Fonts"));
        }
        if let Some(local) = std::env::var_os("LOCALAPPDATA") {
            dirs.push(PathBuf::from(local).join("Microsoft/Windows/Fonts"));
        }
    }

    dirs
}

fn collect_fonts(
    dir: &Path,
    depth: usize,
    entries: &mut Vec<FontEntry>,
    seen: &mut HashSet<String>,
) {
    if depth > 8 {
        return;
    }
    let Ok(read_dir) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in read_dir.flatten() {
        let path = entry.path();
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        if file_type.is_dir() {
            collect_fonts(&path, depth + 1, entries, seen);
            continue;
        }
        let Some(ext) = path.extension().and_then(|ext| ext.to_str()) else {
            continue;
        };
        if !matches!(
            ext.to_ascii_lowercase().as_str(),
            "ttf" | "otf" | "ttc" | "otc"
        ) {
            continue;
        }
        add_font_file(&path, entries, seen);
    }
}

fn add_font_file(path: &Path, entries: &mut Vec<FontEntry>, seen: &mut HashSet<String>) {
    let Ok(data) = std::fs::read(path) else {
        return;
    };
    let faces = ttf_parser::fonts_in_collection(&data).unwrap_or(1);
    for index in 0..faces {
        let Ok(face) = ttf_parser::Face::parse(&data, index) else {
            continue;
        };
        let Some(family) = family_name(&face) else {
            continue;
        };
        if !seen.insert(family.to_lowercase()) {
            continue;
        }
        entries.push(FontEntry {
            family,
            path: path.to_path_buf(),
            index,
        });
    }
}

fn family_name(face: &ttf_parser::Face<'_>) -> Option<String> {
    let mut fallback = None;
    // name_id 16 = Typographic Family，1 = Font Family
    for name_id in [16u16, 1u16] {
        for name in face.names() {
            if name.name_id != name_id {
                continue;
            }
            let Some(text) = name.to_string() else {
                continue;
            };
            let text = text.trim();
            if text.is_empty() {
                continue;
            }
            if name.language_id == 0x0409 {
                return Some(text.to_owned());
            }
            fallback.get_or_insert_with(|| text.to_owned());
        }
        if fallback.is_some() {
            break;
        }
    }
    fallback
}
