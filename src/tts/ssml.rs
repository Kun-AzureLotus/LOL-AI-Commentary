pub fn commentary_to_ssml(text: &str, comma_pause_ms: u32, sentence_pause_ms: u32) -> String {
    let mut body = String::new();
    let chars: Vec<char> = text.chars().collect();

    for (index, ch) in chars.iter().copied().enumerate() {
        body.push_str(&escape_xml(ch));
        if is_comma(ch) {
            body.push_str(&break_tag(comma_pause_ms));
        } else if is_sentence_end(ch) && !next_is_sentence_end(&chars, index) {
            body.push_str(&break_tag(sentence_pause_ms));
        }
    }

    format!(
        r#"<speak version="1.0" xml:lang="zh-CN">{body}</speak>"#
    )
}

fn escape_xml(ch: char) -> String {
    match ch {
        '&' => "&amp;".to_string(),
        '<' => "&lt;".to_string(),
        '>' => "&gt;".to_string(),
        '"' => "&quot;".to_string(),
        '\'' => "&apos;".to_string(),
        _ => ch.to_string(),
    }
}

fn break_tag(ms: u32) -> String {
    format!(r#"<break time="{ms}ms"/>"#)
}

fn is_comma(ch: char) -> bool {
    matches!(ch, ',' | '，' | '、')
}

fn is_sentence_end(ch: char) -> bool {
    matches!(ch, '。' | '！' | '？' | '.' | '!' | '?')
}

fn next_is_sentence_end(chars: &[char], index: usize) -> bool {
    chars
        .get(index + 1)
        .copied()
        .is_some_and(is_sentence_end)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_original_words_and_adds_short_and_long_pauses() {
        let ssml = commentary_to_ssml("蓝方集中，准备开龙。", 140, 260);

        assert!(ssml.contains("蓝方集中"));
        assert!(ssml.contains("准备开龙"));
        assert!(ssml.contains(r#"<break time="140ms"/>"#));
        assert!(ssml.contains(r#"<break time="260ms"/>"#));
        assert!(!ssml.contains("NarrativeIntent"));
    }

    #[test]
    fn does_not_stack_sentence_breaks_on_repeated_punctuation() {
        let ssml = commentary_to_ssml("拿下了！！", 140, 260);
        let count = ssml.matches(r#"<break time="260ms"/>"#).count();

        assert_eq!(count, 1);
    }
}
