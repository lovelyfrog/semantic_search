use std::path::Path;

use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};

pub type LanguageId = String;

pub const CPP_SUF: [&str; 12] = [
    "cpp", "hpp", "cxx", "cc", "c++", "cp", "hxx", "inc", "inl", "ipp", "hh", "h",
];

/// Definition of languages
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Language {
    Plaintext,
    C,
    Cangjie,
    Cpp,
    Css,
    Go,
    Html,
    Java,
    Javascript,
    Json,
    Json5,
    Php,
    Python,
    Ruby,
    Rust,
    Typescript,
    Markdown,
    Arkts,
    Cmake,
    Customized(LanguageId),
}

impl Language {
    pub fn all() -> Vec<Language> {
        vec![
            Language::Plaintext,
            Language::C,
            Language::Cangjie,
            Language::Cpp,
            Language::Css,
            Language::Go,
            Language::Html,
            Language::Java,
            Language::Javascript,
            Language::Json,
            Language::Json5,
            Language::Php,
            Language::Python,
            Language::Ruby,
            Language::Rust,
            Language::Typescript,
            Language::Markdown,
            Language::Arkts,
            Language::Cmake,
        ]
    }
    pub fn is_compatible_with_extension(&self, extension: impl Into<String>) -> bool {
        let extension = extension.into();
        LanguageSetting::from(self)
            .extensions()
            .contains(&extension.to_lowercase())
    }
}

#[derive(Clone, Debug)]
pub struct LanguageSetting {
    extensions: Vec<String>,
}

impl LanguageSetting {
    pub fn new(extensions: Vec<String>) -> Self {
        Self { extensions }
    }

    pub fn extensions(&self) -> &Vec<String> {
        &self.extensions
    }
}

impl From<&Language> for LanguageSetting {
    fn from(language: &Language) -> Self {
        let extensions = match language {
            Language::Plaintext => vec!["txt"],
            Language::C => vec!["c", "i"],
            Language::Cangjie => vec!["char", "cj", "macrocall"],
            Language::Cpp => CPP_SUF.to_vec(),
            Language::Css => vec!["css"],
            Language::Go => vec!["go"],
            Language::Html => vec!["html", "htm", "ng", "sht", "shtm", "shtml"],
            Language::Java => vec!["java", "class"],
            Language::Javascript => vec!["js", "cjs"],
            Language::Json => vec!["json"],
            Language::Json5 => vec!["json5"],
            Language::Php => vec!["php", "php3", "phtml"],
            Language::Python => vec!["py", "py3", "pyc", "pyo", "pyd", "pyw", "pyx", "pyz"],
            Language::Ruby => vec!["rb"],
            Language::Rust => vec!["rs"],
            Language::Typescript => vec!["ts", "tsx"],
            Language::Markdown => vec!["md"],
            Language::Arkts => vec!["ets"],
            Language::Cmake => vec!["cmake"],
            Language::Customized(id) => vec![id.as_str()],
        };
        Self {
            extensions: extensions.into_iter().map(|s| s.to_string()).collect(),
        }
    }
}

lazy_static! {
    pub static ref ALL_LANGUAGES: Vec<Language> = Language::all();
}

impl Language {
    pub fn id(&self) -> LanguageId {
        match self {
            Language::Plaintext => "plaintext".to_string(),
            Language::C => "c".to_string(),
            Language::Cangjie => "Cangjie".to_string(),
            Language::Cpp => "cpp".to_string(),
            Language::Css => "css".to_string(),
            Language::Go => "go".to_string(),
            Language::Html => "html".to_string(),
            Language::Java => "java".to_string(),
            Language::Javascript => "javascript".to_string(),
            Language::Json => "json".to_string(),
            Language::Json5 => "json5".to_string(),
            Language::Php => "php".to_string(),
            Language::Python => "python".to_string(),
            Language::Ruby => "ruby".to_string(),
            Language::Rust => "rust".to_string(),
            Language::Typescript => "typescript".to_string(),
            Language::Markdown => "markdown".to_string(),
            Language::Arkts => "ets".to_string(),
            Language::Cmake => "cmake".to_string(),
            Language::Customized(id) => id.clone(),
        }
    }
}

impl From<LanguageId> for Language {
    fn from(id: LanguageId) -> Self {
        match id.to_lowercase().as_str() {
            "plaintext" => Language::Plaintext,
            "c" => Language::C,
            "cangjie" => Language::Cangjie,
            "cpp" => Language::Cpp,
            "css" => Language::Css,
            "go" => Language::Go,
            "html" => Language::Html,
            "java" => Language::Java,
            "javascript" => Language::Javascript,
            "json" => Language::Json,
            "json5" => Language::Json5,
            "php" => Language::Php,
            "python" => Language::Python,
            "ruby" => Language::Ruby,
            "rust" => Language::Rust,
            "typescript" => Language::Typescript,
            "markdown" => Language::Markdown,
            "ets" | "arkts" => Language::Arkts,
            "cmake" => Language::Cmake,
            _ => Language::Customized(id),
        }
    }
}

impl From<&Path> for Language {
    fn from(path: &Path) -> Self {
        let extension = path
            .extension()
            .map(|ext| ext.to_string_lossy().to_string())
            .unwrap_or_default();
        for language in ALL_LANGUAGES.iter() {
            if language.is_compatible_with_extension(extension.as_str()) {
                return language.clone();
            }
        }
        Language::Customized(extension)
    }
}
