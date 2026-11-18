use serde::{Deserialize, Serialize};
use whatlang::{Lang, detect};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Language {
    pub code: &'static str,
    pub name_zh: &'static str,
    pub name_en: &'static str,
}

pub const LANGUAGES: &[Language] = &[
    Language {
        code: "zh-Hans",
        name_zh: "简体中文",
        name_en: "Simplified Chinese",
    },
    Language {
        code: "zh-Hant",
        name_zh: "繁體中文",
        name_en: "Traditional Chinese",
    },
    Language {
        code: "en",
        name_zh: "英语",
        name_en: "English",
    },
    Language {
        code: "ja",
        name_zh: "日语",
        name_en: "Japanese",
    },
    Language {
        code: "ko",
        name_zh: "韩语",
        name_en: "Korean",
    },
    Language {
        code: "fr",
        name_zh: "法语",
        name_en: "French",
    },
    Language {
        code: "de",
        name_zh: "德语",
        name_en: "German",
    },
    Language {
        code: "es",
        name_zh: "西班牙语",
        name_en: "Spanish",
    },
    Language {
        code: "pt",
        name_zh: "葡萄牙语",
        name_en: "Portuguese",
    },
    Language {
        code: "it",
        name_zh: "意大利语",
        name_en: "Italian",
    },
    Language {
        code: "ru",
        name_zh: "俄语",
        name_en: "Russian",
    },
    Language {
        code: "ar",
        name_zh: "阿拉伯语",
        name_en: "Arabic",
    },
    Language {
        code: "tr",
        name_zh: "土耳其语",
        name_en: "Turkish",
    },
    Language {
        code: "nl",
        name_zh: "荷兰语",
        name_en: "Dutch",
    },
    Language {
        code: "pl",
        name_zh: "波兰语",
        name_en: "Polish",
    },
    Language {
        code: "uk",
        name_zh: "乌克兰语",
        name_en: "Ukrainian",
    },
    Language {
        code: "id",
        name_zh: "印度尼西亚语",
        name_en: "Indonesian",
    },
    Language {
        code: "vi",
        name_zh: "越南语",
        name_en: "Vietnamese",
    },
    Language {
        code: "th",
        name_zh: "泰语",
        name_en: "Thai",
    },
    Language {
        code: "ms",
        name_zh: "马来语",
        name_en: "Malay",
    },
    Language {
        code: "hi",
        name_zh: "印地语",
        name_en: "Hindi",
    },
    Language {
        code: "fa",
        name_zh: "波斯语",
        name_en: "Persian",
    },
    Language {
        code: "he",
        name_zh: "希伯来语",
        name_en: "Hebrew",
    },
    Language {
        code: "sv",
        name_zh: "瑞典语",
        name_en: "Swedish",
    },
    Language {
        code: "da",
        name_zh: "丹麦语",
        name_en: "Danish",
    },
    Language {
        code: "no",
        name_zh: "挪威语",
        name_en: "Norwegian",
    },
    Language {
        code: "fi",
        name_zh: "芬兰语",
        name_en: "Finnish",
    },
    Language {
        code: "cs",
        name_zh: "捷克语",
        name_en: "Czech",
    },
    Language {
        code: "ro",
        name_zh: "罗马尼亚语",
        name_en: "Romanian",
    },
    Language {
        code: "hu",
        name_zh: "匈牙利语",
        name_en: "Hungarian",
    },
    Language {
        code: "el",
        name_zh: "希腊语",
        name_en: "Greek",
    },
];

pub fn resolve_language(value: &str) -> Option<&'static Language> {
    let normalized = value.trim().replace('_', "-").to_ascii_lowercase();
    LANGUAGES.iter().find(|language| {
        language.code.to_ascii_lowercase() == normalized
            || language.name_en.to_ascii_lowercase() == normalized
            || language.name_zh == value.trim()
    })
}

pub fn language_label(value: &str, ui_language: &str) -> String {
    resolve_language(value)
        .map(|language| {
            if ui_language.starts_with("en") {
                language.name_en.to_string()
            } else {
                language.name_zh.to_string()
            }
        })
        .unwrap_or_else(|| value.to_string())
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DetectedLanguage {
    pub code: Option<String>,
    pub label: String,
    pub confidence: f64,
    pub uncertain: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LanguageDirection {
    pub source_code: Option<String>,
    pub source_label: String,
    pub target_value: String,
    pub target_label: String,
    pub confidence: f64,
    pub uncertain: bool,
}

pub fn detect_language(text: &str) -> DetectedLanguage {
    let char_count = text
        .chars()
        .filter(|character| !character.is_whitespace())
        .count();
    let info = detect(text);
    match info {
        Some(info) if char_count >= 6 => {
            let code = lang_code(info.lang()).map(str::to_string);
            let confidence = info.confidence();
            let label = code
                .as_deref()
                .map(|value| language_label(value, "zh"))
                .unwrap_or_else(|| "自动识别".to_string());
            DetectedLanguage {
                code,
                label,
                confidence,
                uncertain: confidence < 0.45,
            }
        }
        Some(info) => {
            let code = lang_code(info.lang()).map(str::to_string);
            DetectedLanguage {
                code: code.clone(),
                label: code
                    .as_deref()
                    .map(|value| language_label(value, "zh"))
                    .unwrap_or_else(|| "自动识别".to_string()),
                confidence: info.confidence(),
                uncertain: true,
            }
        }
        None => DetectedLanguage {
            code: None,
            label: "自动识别".to_string(),
            confidence: 0.0,
            uncertain: true,
        },
    }
}

pub fn resolve_direction(
    text: &str,
    manual_source: Option<&str>,
    target: &str,
    fallback_target: &str,
    ui_language: &str,
) -> LanguageDirection {
    let target = target.trim();
    let fallback_target = fallback_target.trim();
    let manual_source = manual_source
        .map(str::trim)
        .filter(|value| !value.is_empty() && !value.eq_ignore_ascii_case("auto"));

    if let Some(source) = manual_source {
        return LanguageDirection {
            source_code: Some(source.to_string()),
            source_label: language_label(source, ui_language),
            target_value: target.to_string(),
            target_label: language_label(target, ui_language),
            confidence: 1.0,
            uncertain: false,
        };
    }

    let detected = detect_language(text);
    let use_fallback = detected
        .code
        .as_deref()
        .is_some_and(|source| !detected.uncertain && same_language_family(source, target));
    let resolved_target = if use_fallback && fallback_target.is_empty() {
        target
    } else if use_fallback {
        fallback_target
    } else {
        target
    };

    LanguageDirection {
        source_code: detected.code,
        source_label: detected.label,
        target_value: resolved_target.to_string(),
        target_label: language_label(resolved_target, ui_language),
        confidence: detected.confidence,
        uncertain: detected.uncertain,
    }
}

fn same_language_family(source: &str, target: &str) -> bool {
    let source = source.to_ascii_lowercase();
    let target = target.to_ascii_lowercase();
    if source == target {
        return true;
    }
    let source_script = source.split('-').next().unwrap_or(&source);
    let target_script = target.split('-').next().unwrap_or(&target);
    source_script == target_script
}

fn lang_code(lang: Lang) -> Option<&'static str> {
    match lang {
        Lang::Cmn => Some("zh-Hans"),
        Lang::Eng => Some("en"),
        Lang::Jpn => Some("ja"),
        Lang::Kor => Some("ko"),
        Lang::Fra => Some("fr"),
        Lang::Deu => Some("de"),
        Lang::Spa => Some("es"),
        Lang::Por => Some("pt"),
        Lang::Ita => Some("it"),
        Lang::Rus => Some("ru"),
        Lang::Ara => Some("ar"),
        Lang::Tur => Some("tr"),
        Lang::Nld => Some("nl"),
        Lang::Pol => Some("pl"),
        Lang::Ukr => Some("uk"),
        Lang::Ind => Some("id"),
        Lang::Vie => Some("vi"),
        Lang::Tha => Some("th"),
        Lang::Hin => Some("hi"),
        Lang::Pes => Some("fa"),
        Lang::Heb => Some("he"),
        Lang::Swe => Some("sv"),
        Lang::Dan => Some("da"),
        Lang::Nob => Some("no"),
        Lang::Fin => Some("fi"),
        Lang::Ces => Some("cs"),
        Lang::Ron => Some("ro"),
        Lang::Hun => Some("hu"),
        Lang::Ell => Some("el"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_contains_distinct_language_codes() {
        let mut codes = std::collections::HashSet::new();
        for language in LANGUAGES {
            assert!(codes.insert(language.code));
            assert!(!language.name_zh.is_empty());
            assert!(!language.name_en.is_empty());
        }
        assert!(LANGUAGES.len() >= 30);
    }

    #[test]
    fn detects_english_and_flips_when_target_matches() {
        let direction = resolve_direction(
            "The quick brown fox jumps over the lazy dog and reads a book.",
            None,
            "en",
            "zh-Hans",
            "zh",
        );
        assert_eq!(direction.source_code.as_deref(), Some("en"));
        assert_eq!(direction.target_value, "zh-Hans");
    }

    #[test]
    fn manual_source_wins_over_detection() {
        let direction =
            resolve_direction("Bonjour tout le monde", Some("ja"), "zh-Hans", "en", "zh");
        assert_eq!(direction.source_code.as_deref(), Some("ja"));
        assert_eq!(direction.target_label, "简体中文");
        assert_eq!(direction.confidence, 1.0);
    }

    #[test]
    fn short_input_is_marked_uncertain() {
        let detected = detect_language("Hi");
        assert!(detected.uncertain);
    }
}
