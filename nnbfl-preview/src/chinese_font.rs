// Adapted from https://github.com/szjerrywang/egui-chinese-font/tree/egui-0.34, for egui-0.35 support

// Copyright (c) 2025 egui-chinese-font contributors <259901434@qq.com>
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

//! # egui-chinese-font
//!
//! A simple crate to load Chinese fonts for egui applications.
//!
//! This crate provides cross-platform support for loading system Chinese fonts
//! and configuring them with egui applications.
//!
//! ## Usage
//!
//! ```rust,no_run
//! use egui_chinese_font::setup_chinese_fonts;
//!
//! let ctx = egui::Context::default();
//! setup_chinese_fonts(&ctx);
//! ```

use egui::{Context, FontData, FontDefinitions, FontFamily};

/// Error type for font loading operations
#[derive(Debug)]
pub enum FontError {
    /// Font file not found
    NotFound(String),
    /// Failed to read font file
    ReadError(std::io::Error),
    /// Platform not supported
    UnsupportedPlatform,
}

impl std::fmt::Display for FontError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FontError::NotFound(path) => write!(f, "Font file not found: {}", path),
            FontError::ReadError(err) => write!(f, "Failed to read font file: {}", err),
            FontError::UnsupportedPlatform => write!(f, "Platform not supported"),
        }
    }
}

impl std::error::Error for FontError {}

/// Setup Chinese fonts for egui context
///
/// This function will attempt to load system Chinese fonts and configure them
/// for use with the provided egui context.
///
/// # Arguments
/// * `ctx` - The egui context to configure
///
/// # Returns
/// * `Ok(())` if fonts were successfully loaded
/// * `Err(FontError)` if font loading failed
pub fn setup_chinese_fonts(ctx: &Context) -> Result<(), FontError> {
    let mut fonts = FontDefinitions::default();

    // Try to load Chinese fonts based on platform
    let chinese_font_data = load_chinese_font()?;

    // Insert the Chinese font
    fonts
        .font_data
        .insert("chinese".to_owned(), chinese_font_data.into());

    // Configure font families
    fonts
        .families
        .entry(FontFamily::Proportional)
        .or_default()
        .insert(0, "chinese".to_owned());
    fonts
        .families
        .entry(FontFamily::Monospace)
        .or_default()
        .insert(0, "chinese".to_owned());

    // Apply the font configuration
    ctx.set_fonts(fonts);

    Ok(())
}

/// Load Chinese font data from system
fn load_chinese_font() -> Result<FontData, FontError> {
    #[cfg(target_os = "windows")]
    {
        load_windows_chinese_font()
    }

    #[cfg(target_os = "macos")]
    {
        load_macos_chinese_font()
    }

    #[cfg(target_os = "linux")]
    {
        load_linux_chinese_font()
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    {
        Err(FontError::UnsupportedPlatform)
    }
}

#[cfg(target_os = "windows")]
fn load_windows_chinese_font() -> Result<FontData, FontError> {
    // List of common Chinese font paths on Windows
    let font_paths = [
        r"C:\Windows\Fonts\msyh.ttc",    // Microsoft YaHei
        r"C:\Windows\Fonts\msyhbd.ttc",  // Microsoft YaHei Bold
        r"C:\Windows\Fonts\simsun.ttc",  // SimSun
        r"C:\Windows\Fonts\simhei.ttf",  // SimHei
        r"C:\Windows\Fonts\simkai.ttf",  // KaiTi
        r"C:\Windows\Fonts\simfang.ttf", // FangSong
        r"C:\Windows\Fonts\msjh.ttc",    // Microsoft JhengHei (Traditional Chinese)
        r"C:\Windows\Fonts\msjhbd.ttc",  // Microsoft JhengHei Bold
        r"C:\Windows\Fonts\kaiu.ttf",    // DFKai-SB (Traditional Chinese)
        r"C:\Windows\Fonts\mingliu.ttc", // MingLiU (Traditional Chinese)
    ];

    for font_path in &font_paths {
        if let Ok(font_data) = std::fs::read(font_path) {
            return Ok(FontData::from_owned(font_data));
        }
    }

    Err(FontError::NotFound(
        "No Chinese font found on Windows".to_string(),
    ))
}

#[cfg(target_os = "macos")]
fn load_macos_chinese_font() -> Result<FontData, FontError> {
    let font_paths = [
        "/System/Library/Fonts/PingFang.ttc",      // PingFang SC
        "/System/Library/Fonts/STHeiti Light.ttc", // STHeiti
        "/System/Library/Fonts/STHeiti Medium.ttc",
        "/System/Library/Fonts/Hiragino Sans GB.ttc", // Hiragino Sans GB
        "/Library/Fonts/Arial Unicode.ttf",           // Arial Unicode MS
        "/System/Library/Fonts/Apple LiGothic Medium.ttf", // Apple LiGothic (Traditional)
    ];

    for font_path in &font_paths {
        if let Ok(font_data) = std::fs::read(font_path) {
            return Ok(FontData::from_owned(font_data));
        }
    }

    Err(FontError::NotFound(
        "No Chinese font found on macOS".to_string(),
    ))
}

#[cfg(target_os = "linux")]
fn load_linux_chinese_font() -> Result<FontData, FontError> {
    // Common Chinese font paths on Linux distributions
    let font_paths = [
        "/usr/share/fonts/truetype/droid/DroidSansFallbackFull.ttf",
        "/usr/share/fonts/truetype/arphic/uming.ttc",
        "/usr/share/fonts/truetype/arphic/ukai.ttc",
        "/usr/share/fonts/truetype/wqy/wqy-microhei.ttc",
        "/usr/share/fonts/truetype/wqy/wqy-zenhei.ttc",
        "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
        "/usr/share/fonts/truetype/liberation/LiberationSans-Regular.ttf",
        // Ubuntu/Debian paths
        "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
        // CentOS/RHEL paths
        "/usr/share/fonts/google-droid/DroidSansFallbackFull.ttf",
        // Arch Linux paths
        "/usr/share/fonts/noto-cjk/NotoSansCJK-Regular.ttc",
    ];

    for font_path in &font_paths {
        if let Ok(font_data) = std::fs::read(font_path) {
            return Ok(FontData::from_owned(font_data));
        }
    }

    Err(FontError::NotFound(
        "No Chinese font found on Linux".to_string(),
    ))
}

/// Setup Chinese fonts with custom font data
///
/// This function allows you to provide your own font data instead of
/// loading from system fonts.
///
/// # Arguments
/// * `ctx` - The egui context to configure
/// * `font_data` - The font data to use
/// * `font_name` - Name for the font (optional, defaults to "chinese")
pub fn setup_custom_chinese_font(ctx: &Context, font_data: Vec<u8>, font_name: Option<&str>) {
    let mut fonts = FontDefinitions::default();
    let name = font_name.unwrap_or("chinese");

    fonts
        .font_data
        .insert(name.to_owned(), FontData::from_owned(font_data).into());

    fonts
        .families
        .entry(FontFamily::Proportional)
        .or_default()
        .insert(0, name.to_owned());
    fonts
        .families
        .entry(FontFamily::Monospace)
        .or_default()
        .insert(0, name.to_owned());

    ctx.set_fonts(fonts);
}

/// Get available Chinese font paths on the current system
///
/// This function returns a list of paths where Chinese fonts might be located
/// on the current platform. Useful for debugging font loading issues.
pub fn get_chinese_font_paths() -> Vec<String> {
    #[cfg(target_os = "windows")]
    {
        vec![
            r"C:\Windows\Fonts\msyh.ttc".to_string(),
            r"C:\Windows\Fonts\msyhbd.ttc".to_string(),
            r"C:\Windows\Fonts\simsun.ttc".to_string(),
            r"C:\Windows\Fonts\simhei.ttf".to_string(),
            r"C:\Windows\Fonts\simkai.ttf".to_string(),
            r"C:\Windows\Fonts\simfang.ttf".to_string(),
            r"C:\Windows\Fonts\msjh.ttc".to_string(),
            r"C:\Windows\Fonts\msjhbd.ttc".to_string(),
        ]
    }

    #[cfg(target_os = "macos")]
    {
        vec![
            "/System/Library/Fonts/PingFang.ttc".to_string(),
            "/System/Library/Fonts/STHeiti Light.ttc".to_string(),
            "/System/Library/Fonts/STHeiti Medium.ttc".to_string(),
            "/System/Library/Fonts/Hiragino Sans GB.ttc".to_string(),
            "/Library/Fonts/Arial Unicode.ttf".to_string(),
        ]
    }

    #[cfg(target_os = "linux")]
    {
        vec![
            "/usr/share/fonts/truetype/droid/DroidSansFallbackFull.ttf".to_string(),
            "/usr/share/fonts/truetype/arphic/uming.ttc".to_string(),
            "/usr/share/fonts/truetype/wqy/wqy-microhei.ttc".to_string(),
            "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc".to_string(),
        ]
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    {
        vec![]
    }
}
