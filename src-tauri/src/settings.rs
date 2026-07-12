//! 用户设置: app-data 目录下的 `settings.json`。
//!
//! 跨版本兼容策略: 结构体整体挂 `#[serde(default)]`——旧版本文件缺字段自动落默认值,
//! 新版本删掉的字段被忽略, 升级永远不会重置或读挂用户设置; 读取失败(缺失/损坏)
//! 一律回全默认, 不打断启动。

use std::path::Path;

use serde::{Deserialize, Serialize};

/// 编辑器字号钳制范围(px)
const FONT_SIZE_RANGE: std::ops::RangeInclusive<u32> = 8..=32;

/// 关闭窗口时的行为
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum CloseBehavior {
    /// 收进系统托盘驻留(默认)
    #[default]
    Tray,
    /// 直接退出应用
    Quit,
}

/// 用户设置(与前端 `api.ts` 的 `Settings` 严格镜像, camelCase)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    /// 编辑器字号(px), 钳制在 8–32
    pub editor_font_size: u32,
    /// 编辑器等宽字体名; 空串 = 内嵌 JetBrains Mono
    pub editor_font_family: String,
    /// 关闭窗口时的行为
    pub close_behavior: CloseBehavior,
    /// 三栏词级强调的默认开关
    pub highlight_words: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            editor_font_size: 12,
            editor_font_family: String::new(),
            close_behavior: CloseBehavior::Tray,
            highlight_words: true,
        }
    }
}

impl Settings {
    /// 从文件读取; 文件缺失或损坏时回全默认(尽力而为, 不打断启动)
    pub fn load(path: &Path) -> Self {
        std::fs::read_to_string(path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    /// 落盘为 pretty JSON(父目录不存在则创建)
    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let json = serde_json::to_string_pretty(self).map_err(std::io::Error::other)?;
        std::fs::write(path, json)
    }

    /// 归一化非法值(字号钳制、字体名去掉会破坏 CSS 值的字符并截断),
    /// 写入前统一过一遍, 前端也以返回值回同步
    pub fn normalized(mut self) -> Self {
        self.editor_font_size = self
            .editor_font_size
            .clamp(*FONT_SIZE_RANGE.start(), *FONT_SIZE_RANGE.end());
        self.editor_font_family = self
            .editor_font_family
            .chars()
            .filter(|c| !matches!(c, ';' | '"' | '\'' | '{' | '}' | '\\'))
            .take(64)
            .collect::<String>()
            .trim()
            .to_string();
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_file_falls_back_to_defaults() {
        let s = Settings::load(Path::new("/nonexistent/settings.json"));
        assert_eq!(s, Settings::default());
    }

    #[test]
    fn partial_json_fills_defaults_and_ignores_unknown_fields() {
        // 模拟旧版本文件(缺新字段)与更新版本文件(带未知字段)
        let s: Settings =
            serde_json::from_str(r#"{"editorFontSize": 16, "futureField": true}"#).unwrap();
        assert_eq!(s.editor_font_size, 16);
        assert_eq!(s.editor_font_family, "");
        assert_eq!(s.close_behavior, CloseBehavior::Tray);
        assert!(s.highlight_words);
    }

    #[test]
    fn corrupt_json_falls_back_to_defaults() {
        let dir = std::env::temp_dir().join(format!("pincer-settings-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("corrupt.json");
        std::fs::write(&file, "{ not json").unwrap();
        assert_eq!(Settings::load(&file), Settings::default());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn save_load_roundtrip() {
        let dir = std::env::temp_dir().join(format!("pincer-settings-rt-{}", std::process::id()));
        let file = dir.join("settings.json");
        let s = Settings {
            editor_font_size: 14,
            editor_font_family: "Fira Code".into(),
            close_behavior: CloseBehavior::Quit,
            highlight_words: false,
        };
        s.save(&file).unwrap();
        assert_eq!(Settings::load(&file), s);
        // 文件键名为 camelCase(与前端镜像一致)
        let raw = std::fs::read_to_string(&file).unwrap();
        assert!(raw.contains("editorFontSize"));
        assert!(raw.contains("closeBehavior"));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn normalized_clamps_font_size_and_sanitizes_family() {
        let low = Settings {
            editor_font_size: 4,
            ..Settings::default()
        };
        assert_eq!(low.normalized().editor_font_size, 8);
        let high = Settings {
            editor_font_size: 99,
            ..Settings::default()
        };
        assert_eq!(high.normalized().editor_font_size, 32);
        let dirty = Settings {
            editor_font_family: "  \"Fira; Code\"{}  ".into(),
            ..Settings::default()
        };
        assert_eq!(dirty.normalized().editor_font_family, "Fira Code");
    }
}
