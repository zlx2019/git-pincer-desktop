//! 用户设置: app-data 目录下的 `settings.json`。
//!
//! 跨版本兼容策略: 结构体整体挂 `#[serde(default)]`——旧版本文件缺字段自动落默认值,
//! 新版本删掉的字段被忽略, 升级永远不会重置或读挂用户设置; 读取失败(缺失/损坏)
//! 一律回全默认, 不打断启动。

use std::path::Path;

use serde::{Deserialize, Serialize};

/// 编辑器字号钳制范围(px)
const FONT_SIZE_RANGE: std::ops::RangeInclusive<u32> = 8..=32;

/// 窗口逻辑尺寸(DPI 无关; 形态尺寸记忆用)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct WinSize {
    /// 逻辑宽(px)
    pub width: u32,
    /// 逻辑高(px)
    pub height: u32,
}

impl WinSize {
    /// 双维下限钳制(记忆值不得小于形态最小尺寸)
    pub fn clamp_min(self, min: WinSize) -> WinSize {
        WinSize {
            width: self.width.max(min.width),
            height: self.height.max(min.height),
        }
    }
}

/// 小窗(打开页/菜单)最小尺寸
pub const COMPACT_MIN: WinSize = WinSize {
    width: 380,
    height: 520,
};
/// 小窗出厂尺寸
pub const COMPACT_DEFAULT: WinSize = WinSize {
    width: 420,
    height: 640,
};
/// 大窗(冲突列表/三栏)最小尺寸
pub const LARGE_MIN: WinSize = WinSize {
    width: 960,
    height: 640,
};
/// 大窗出厂尺寸
pub const LARGE_DEFAULT: WinSize = WinSize {
    width: 1280,
    height: 800,
};

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

/// 界面主题(IDEA New UI 双色系)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum AppTheme {
    /// 暗色(默认)
    #[default]
    Dark,
    /// 亮色
    Light,
}

/// 界面语言
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    /// 中文(默认): 保持"大窗 IDEA 英文原文 + 小窗中文辅助"的分层设计
    #[default]
    Zh,
    /// 全英文: 小窗辅助文案也切英文
    En,
}

impl Language {
    /// 托盘菜单文案 (显示窗口, 退出)
    pub fn tray_labels(self) -> (&'static str, &'static str) {
        match self {
            Language::Zh => ("显示窗口", "退出"),
            Language::En => ("Show Window", "Quit"),
        }
    }

    /// 应用菜单"设置"项文案(macOS 菜单栏, 省略号是 macOS 惯例)
    pub fn settings_label(self) -> &'static str {
        match self {
            Language::Zh => "设置…",
            Language::En => "Settings…",
        }
    }
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
    /// 界面主题
    pub theme: AppTheme,
    /// 界面语言
    pub language: Language,
    /// 小窗上次尺寸(None = 没手动调过, 用出厂默认)。
    /// 两个尺寸字段由 Rust 壳层独占写入(离开形态/隐藏/退出时快照),
    /// `set_settings` 会忽略前端带来的值——前端副本可能是陈旧的
    pub compact_size: Option<WinSize>,
    /// 大窗上次尺寸(None = 没手动调过, 用出厂默认)
    pub large_size: Option<WinSize>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            editor_font_size: 12,
            editor_font_family: String::new(),
            close_behavior: CloseBehavior::Tray,
            highlight_words: true,
            theme: AppTheme::Dark,
            language: Language::Zh,
            compact_size: None,
            large_size: None,
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
        self.compact_size = self.compact_size.map(|s| s.clamp_min(COMPACT_MIN));
        self.large_size = self.large_size.map(|s| s.clamp_min(LARGE_MIN));
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
        assert_eq!(s.theme, AppTheme::Dark);
        assert_eq!(s.language, Language::Zh);
        assert_eq!(s.compact_size, None);
        assert_eq!(s.large_size, None);
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
            theme: AppTheme::Light,
            language: Language::En,
            compact_size: Some(WinSize {
                width: 500,
                height: 700,
            }),
            large_size: None,
        };
        s.save(&file).unwrap();
        assert_eq!(Settings::load(&file), s);
        // 文件键名为 camelCase(与前端镜像一致), 枚举值 lowercase
        let raw = std::fs::read_to_string(&file).unwrap();
        assert!(raw.contains("editorFontSize"));
        assert!(raw.contains("closeBehavior"));
        assert!(raw.contains("compactSize"));
        assert!(raw.contains("\"light\""));
        assert!(raw.contains("\"en\""));
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

    #[test]
    fn normalized_clamps_window_sizes_to_form_minimums() {
        let s = Settings {
            compact_size: Some(WinSize {
                width: 100,
                height: 9000,
            }),
            large_size: Some(WinSize {
                width: 1000,
                height: 100,
            }),
            ..Settings::default()
        }
        .normalized();
        // 单维不足只抬该维; 高于最小值的维度原样保留
        assert_eq!(
            s.compact_size,
            Some(WinSize {
                width: 380,
                height: 9000,
            })
        );
        assert_eq!(
            s.large_size,
            Some(WinSize {
                width: 1000,
                height: 640,
            })
        );
    }
}
