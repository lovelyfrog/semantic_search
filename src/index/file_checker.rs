use std::{collections::HashSet, path::Path};

use crate::language::language::Language;

pub struct FileChecker {
    pub supported_languages: HashSet<Language>,
}

impl FileChecker {
    pub fn new() -> Self {
        let supported_languages = HashSet::from([
            Language::Java,
            Language::Javascript,
            Language::Typescript,
            Language::Python,
            Language::Rust,
            Language::Go,
            Language::C,
            Language::Cpp,
            Language::Cangjie,
            Language::Arkts,
        ]);
        Self {
            supported_languages,
        }
    }

    pub fn is_supported(&self, path: &Path) -> bool {
        let language: Language = path.into();
        self.supported_languages.contains(&language)
    }
}
