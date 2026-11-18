pub const DEFAULT_SYSTEM_PROMPT: &str = r#"You are Yiyu, a precise translation engine.
Translate the user's content from {sourceLang} to {targetLang}.
Return only the translation. Do not explain, summarize, answer, add facts, or provide alternatives.
Preserve paragraphs, Markdown structure, lists, code blocks, indentation, URLs, placeholders, and inline markup.
Do not translate code identifiers or code inside fenced blocks unless they contain natural-language comments."#;

pub fn resolve_system_prompt(
    custom: Option<&str>,
    source_language: &str,
    target_language: &str,
) -> String {
    let template = custom
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(DEFAULT_SYSTEM_PROMPT);

    template
        .replace("{sourceLang}", source_language)
        .replace("{sourceLanguage}", source_language)
        .replace("{targetLang}", target_language)
        .replace("{targetLanguage}", target_language)
}

pub fn prompt_has_known_placeholder(value: &str) -> bool {
    value.contains("{sourceLang}")
        || value.contains("{sourceLanguage}")
        || value.contains("{targetLang}")
        || value.contains("{targetLanguage}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_custom_prompt_uses_the_builtin_prompt() {
        let prompt = resolve_system_prompt(Some("   "), "English", "Simplified Chinese");
        assert!(prompt.contains("Return only the translation")); // ensures the built-in prompt was resolved
        assert!(!prompt.contains("{targetLang}"));
        assert!(prompt.contains("Simplified Chinese"));
    }

    #[test]
    fn both_placeholder_spellings_are_replaced() {
        let prompt = resolve_system_prompt(
            Some("{sourceLang} -> {targetLanguage}"),
            "English",
            "Japanese",
        );
        assert_eq!(prompt, "English -> Japanese");
    }

    #[test]
    fn builtin_prompt_states_the_output_constraint() {
        assert!(DEFAULT_SYSTEM_PROMPT.contains("Return only the translation"));
        assert!(prompt_has_known_placeholder(DEFAULT_SYSTEM_PROMPT));
    }
}
