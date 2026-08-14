use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Language {
    #[default]
    /// US English
    USen,

    /// EU English
    EUen,

    /// Japanese
    JPja,

    /// Simplified Chinese
    CNzh,

    /// Traditional Chinese
    TWzh,

    /// Korean
    KRko,

    /// EU French
    EUfr,

    /// US French
    USfr,

    /// German
    EUde,

    /// EU Spanish
    EUes,

    /// US Spanish
    USes,

    /// Italian
    EUit,

    /// Dutch
    EUnl,

    /// Russian
    EUru,
}

impl Language {
    pub const ALL: &'static [Language] = &[
        Language::USen,
        Language::EUen,
        Language::JPja,
        Language::CNzh,
        Language::TWzh,
        Language::KRko,
        Language::EUfr,
        Language::USfr,
        Language::EUde,
        Language::EUes,
        Language::USes,
        Language::EUit,
        Language::EUnl,
        Language::EUru,
    ];

    pub fn label(&self) -> &'static str {
        match self {
            Self::USen => "English (US)",
            Self::EUen => "English (EU)",
            Self::JPja => "Japanese (日本語)",
            Self::CNzh => "Simplified Chinese (简体中文)",
            Self::TWzh => "Traditional Chinese (繁體中文)",
            Self::KRko => "Korean (한국어)",
            Self::EUfr => "French (EU - Français)",
            Self::USfr => "French (US - Français)",
            Self::EUde => "German (Deutsch)",
            Self::EUes => "Spanish (EU - Español)",
            Self::USes => "Spanish (US - Español)",
            Self::EUit => "Italian (Italiano)",
            Self::EUnl => "Dutch (Nederlands)",
            Self::EUru => "Russian (Русский)",
        }
    }

    pub fn code_str(&self) -> &'static str {
        match self {
            Self::USen => "USen",
            Self::EUen => "EUen",
            Self::JPja => "JPja",
            Self::CNzh => "CNzh",
            Self::TWzh => "TWzh",
            Self::KRko => "KRko",
            Self::EUfr => "EUfr",
            Self::USfr => "USfr",
            Self::EUde => "EUde",
            Self::EUes => "EUes",
            Self::USes => "USes",
            Self::EUit => "EUit",
            Self::EUnl => "EUnl",
            Self::EUru => "EUru",
        }
    }

    pub fn region_short(&self) -> &'static str {
        match self {
            Self::USen | Self::USfr | Self::USes => "US",
            Self::EUen
            | Self::EUfr
            | Self::EUde
            | Self::EUes
            | Self::EUit
            | Self::EUnl
            | Self::EUru => "EU",
            Self::JPja => "JP",
            Self::CNzh => "CN",
            Self::TWzh => "TW",
            Self::KRko => "KR",
        }
    }
}

impl std::fmt::Display for Language {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.label())
    }
}

pub trait GameConfig {
    fn romfs_path(&self) -> &Path;

    fn layout_folder_path(&self) -> PathBuf {
        self.romfs_path().join("Layout")
    }

    fn general_font_folder_path(&self) -> PathBuf {
        self.romfs_path().join("Font")
    }

    /// Directory containing `.bfcpx` files for a given language.
    fn fcpx_folder_path(&self, lang: Language) -> PathBuf;

    /// Directory containing scalable vector fonts (`.bfttf`, `.bfotf`) for a given language.
    fn scalable_font_folder_path(&self, lang: Language) -> PathBuf;

    /// Directory containing bitmap fonts (`.bffnt`) if applicable
    fn bmp_font_folder_path(&self, lang: Language) -> Option<PathBuf> {
        let _ = lang;
        None
    }

    /// Resolves the exact `.bfcpx` file path given a base name and language.
    fn fcpx_file_path(&self, base_name: &str, lang: Language) -> PathBuf;

    fn list_scalable_font_files(&self, lang: Language) -> Vec<PathBuf> {
        let folder = self.scalable_font_folder_path(lang);
        let mut fonts = Vec::new();

        let Ok(entries) = std::fs::read_dir(&folder) else {
            return fonts;
        };

        for entry in entries.flatten() {
            let path = entry.path();

            if path.is_file()
                && let Some(ext) = path.extension().and_then(|s| s.to_str())
            {
                let ext_lower = ext.to_lowercase();
                if matches!(ext_lower.as_str(), "ttf" | "otf" | "bfttf" | "bfotf") {
                    fonts.push(path);
                }
            }
        }

        fonts
    }
}

pub struct ACNH {
    pub romfs_path: PathBuf,
}

impl GameConfig for ACNH {
    fn romfs_path(&self) -> &Path {
        &self.romfs_path
    }

    fn fcpx_folder_path(&self, _lang: Language) -> PathBuf {
        self.general_font_folder_path().join("Fcpx")
    }

    fn scalable_font_folder_path(&self, _lang: Language) -> PathBuf {
        self.general_font_folder_path().join("ScalableFont")
    }

    fn bmp_font_folder_path(&self, lang: Language) -> Option<PathBuf> {
        let font_base = self.general_font_folder_path();
        let folder_name = format!("BmpFont_{}", lang.region_short());
        let path = font_base.join(&folder_name);

        if path.exists() {
            Some(path)
        } else {
            Some(font_base.join("BmpFont_US"))
        }
    }

    fn fcpx_file_path(&self, base_name: &str, lang: Language) -> PathBuf {
        let fcpx_dir = self.fcpx_folder_path(lang);
        let name_clean = base_name.strip_suffix(".bfcpx").unwrap_or(base_name);

        let lang_file_name = match lang {
            Language::CNzh | Language::TWzh | Language::KRko => {
                format!("{name_clean}_{}.bfcpx", lang.code_str())
            }
            _ => format!("{name_clean}.bfcpx"),
        };

        let target_path = fcpx_dir.join(&lang_file_name);

        if target_path.exists() {
            target_path
        } else {
            fcpx_dir.join(format!("{name_clean}.bfcpx"))
        }
    }
}

pub struct TomodachiLife {
    pub romfs_path: PathBuf,
}

impl TomodachiLife {
    fn localized_font_root(&self, lang: Language) -> PathBuf {
        let folder_name = match lang {
            Language::USen | Language::EUen => "Font.Nin_NX_NVN".to_string(),
            other => format!("Font_{}.Nin_NX_NVN", other.code_str()),
        };

        let path = self.general_font_folder_path().join(&folder_name);

        if path.exists() {
            path
        } else {
            self.general_font_folder_path().join("Font.Nin_NX_NVN")
        }
    }
}

impl GameConfig for TomodachiLife {
    fn romfs_path(&self) -> &Path {
        &self.romfs_path
    }

    fn fcpx_folder_path(&self, lang: Language) -> PathBuf {
        self.localized_font_root(lang).join("fcpx")
    }

    fn scalable_font_folder_path(&self, lang: Language) -> PathBuf {
        self.localized_font_root(lang).join("scft")
    }

    fn fcpx_file_path(&self, base_name: &str, lang: Language) -> PathBuf {
        let fcpx_dir = self.fcpx_folder_path(lang);
        let name_clean = base_name.strip_suffix(".bfcpx").unwrap_or(base_name);

        let target_path = fcpx_dir.join(format!("{name_clean}.bfcpx"));

        if target_path.exists() {
            target_path
        } else {
            self.localized_font_root(Language::USen)
                .join("fcpx")
                .join(format!("{name_clean}.bfcpx"))
        }
    }
}
