//! Канал `files`: настройки файлового менеджера conchy-files (`~/.config/conchy/files.kdl`).

use anyhow::Result;
use kdl::{KdlDocument, KdlValue};

/// По какой колонке сортировать список.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortBy {
    Name,
    Size,
    Type,
    Modified,
}

impl SortBy {
    pub fn as_str(self) -> &'static str {
        match self {
            SortBy::Name => "name",
            SortBy::Size => "size",
            SortBy::Type => "type",
            SortBy::Modified => "modified",
        }
    }

    fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "name" => SortBy::Name,
            "size" => SortBy::Size,
            "type" => SortBy::Type,
            "modified" => SortBy::Modified,
            _ => return None,
        })
    }
}

/// Что показывать в колонке «Размер» у папок.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FolderSize {
    /// Число элементов внутри («12 элементов»), как в Thunar.
    Items,
    /// Ничего («—»): папка не читается, список открывается быстрее.
    None,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FilesConfig {
    pub show_hidden: bool,
    pub folders_first: bool,
    pub sort_by: SortBy,
    pub sort_ascending: bool,
    pub folder_size: FolderSize,
    pub width: u32,
    pub height: u32,
    pub sidebar_width: u32,
    /// Программа для «Открыть терминал здесь». `None` — `$TERMINAL` или первая найденная.
    pub terminal: Option<String>,
    /// Тема иконок. `None` — как у GTK-программ (`gtk-icon-theme-name`).
    pub icon_theme: Option<String>,
    pub font_size: f32,
}

impl Default for FilesConfig {
    fn default() -> Self {
        Self {
            show_hidden: false,
            folders_first: true,
            sort_by: SortBy::Name,
            sort_ascending: true,
            folder_size: FolderSize::Items,
            width: 1100,
            height: 720,
            sidebar_width: 210,
            terminal: None,
            icon_theme: None,
            font_size: 13.0,
        }
    }
}

pub fn default_files_kdl() -> &'static str {
    r#"// ==============================================================================
// Conchy OS - File Manager Configuration (conchy-files)
// Location: ~/.config/conchy/files.kdl or /etc/conchy/files.kdl
// ==============================================================================

view {
    show-hidden false        // Ctrl+H; conchy-files saves it here
    folders-first true
    sort-by "name"           // name, size, type, modified
    sort-order "ascending"   // ascending, descending
    folder-size "items"      // items: "12 items"; none: faster on huge folders
}

window {
    width 1100
    height 720
    sidebar-width 210
}

terminal {
    // "Open Terminal Here". Empty: $TERMINAL or the first one found
    command ""
}

theme {
    icon-theme ""            // empty: same as GTK apps (gtk-icon-theme-name)
    font-size 13
}
"#
}

/// Разбирает `files.kdl`. Незнакомые и неверные значения заменяются значениями по умолчанию.
pub fn parse_files_config(doc: &KdlDocument) -> Result<FilesConfig> {
    let mut c = FilesConfig::default();
    let block = |name: &str| doc.get(name).and_then(|n| n.children());

    if let Some(v) = block("view") {
        if let Some(b) = arg_bool(v, "show-hidden") {
            c.show_hidden = b;
        }
        if let Some(b) = arg_bool(v, "folders-first") {
            c.folders_first = b;
        }
        if let Some(s) = arg_str(v, "sort-by").and_then(|s| SortBy::parse(&s)) {
            c.sort_by = s;
        }
        match arg_str(v, "sort-order").as_deref() {
            Some("ascending") => c.sort_ascending = true,
            Some("descending") => c.sort_ascending = false,
            _ => {}
        }
        match arg_str(v, "folder-size").as_deref() {
            Some("items") => c.folder_size = FolderSize::Items,
            Some("none") => c.folder_size = FolderSize::None,
            _ => {}
        }
    }
    if let Some(w) = block("window") {
        if let Some(n) = arg_num(w, "width").filter(|n| *n >= 300.0) {
            c.width = n as u32;
        }
        if let Some(n) = arg_num(w, "height").filter(|n| *n >= 200.0) {
            c.height = n as u32;
        }
        if let Some(n) = arg_num(w, "sidebar-width").filter(|n| (100.0..=600.0).contains(n)) {
            c.sidebar_width = n as u32;
        }
    }
    if let Some(t) = block("terminal") {
        c.terminal = arg_str(t, "command").filter(|s| !s.trim().is_empty());
    }
    if let Some(t) = block("theme") {
        c.icon_theme = arg_str(t, "icon-theme").filter(|s| !s.trim().is_empty());
        if let Some(n) = arg_num(t, "font-size").filter(|n| (8.0..=32.0).contains(n)) {
            c.font_size = n as f32;
        }
    }
    Ok(c)
}

fn arg_bool(doc: &KdlDocument, key: &str) -> Option<bool> {
    match doc.get_arg(key)? {
        KdlValue::Bool(b) => Some(*b),
        _ => None,
    }
}

fn arg_str(doc: &KdlDocument, key: &str) -> Option<String> {
    match doc.get_arg(key)? {
        KdlValue::String(s) | KdlValue::RawString(s) => Some(s.clone()),
        _ => None,
    }
}

fn arg_num(doc: &KdlDocument, key: &str) -> Option<f64> {
    match doc.get_arg(key)? {
        KdlValue::Base10(i) => Some(*i as f64),
        KdlValue::Base10Float(f) => Some(*f),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_document_matches_default_struct() {
        let doc: KdlDocument = default_files_kdl().parse().unwrap();
        assert_eq!(parse_files_config(&doc).unwrap(), FilesConfig::default());
    }

    #[test]
    fn parses_values_and_ignores_bad_ones() {
        let doc: KdlDocument = r#"
            view {
                show-hidden true
                sort-by "size"
                sort-order "descending"
                folder-size "none"
                folders-first "yes"
            }
            window {
                width 50
                sidebar-width 260
            }
            terminal {
                command "foot"
            }
            theme {
                icon-theme "Papirus"
                font-size 14.5
            }
        "#
        .parse()
        .unwrap();
        let c = parse_files_config(&doc).unwrap();
        assert!(c.show_hidden);
        assert!(c.folders_first, "строка вместо bool — значение по умолчанию");
        assert_eq!(c.sort_by, SortBy::Size);
        assert!(!c.sort_ascending);
        assert_eq!(c.folder_size, FolderSize::None);
        assert_eq!(c.width, 1100, "слишком маленькое окно — значение по умолчанию");
        assert_eq!(c.sidebar_width, 260);
        assert_eq!(c.terminal.as_deref(), Some("foot"));
        assert_eq!(c.icon_theme.as_deref(), Some("Papirus"));
        assert_eq!(c.font_size, 14.5);
    }
}
