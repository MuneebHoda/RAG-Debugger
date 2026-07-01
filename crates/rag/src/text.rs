use std::collections::BTreeSet;

pub(crate) fn query_terms(query: &str) -> Vec<String> {
    normalized_tokens(query)
        .into_iter()
        .filter(|token| !is_stop_word(token))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

pub(crate) fn normalized_tokens(text: &str) -> Vec<String> {
    text.split(|character: char| !character.is_alphanumeric())
        .filter_map(|token| {
            let token = token.trim().to_ascii_lowercase();
            (!token.is_empty()).then_some(token)
        })
        .collect()
}

pub(crate) fn normalize_text(text: &str) -> String {
    normalized_tokens(text).join(" ")
}

pub(crate) fn split_sentences(text: &str) -> Vec<&str> {
    let mut sentences = Vec::new();
    let mut start = 0usize;

    for (index, character) in text.char_indices() {
        if matches!(character, '.' | '!' | '?' | '\n') {
            if start < index {
                sentences.push(&text[start..index]);
            }
            start = index + character.len_utf8();
        }
    }

    if start < text.len() {
        sentences.push(&text[start..]);
    }

    sentences
}

pub(crate) fn best_snippet(text: &str, terms: &[String]) -> String {
    let mut best_sentence = "";
    let mut best_score = 0usize;

    for sentence in split_sentences(text) {
        let tokens = normalized_tokens(sentence);
        let score = terms
            .iter()
            .filter(|term| tokens.iter().any(|token| token == *term))
            .count();
        if score > best_score {
            best_sentence = sentence;
            best_score = score;
        }
    }

    if best_sentence.trim().is_empty() {
        truncate_chars(text.trim(), 280)
    } else {
        truncate_chars(best_sentence.trim(), 280)
    }
}

pub(crate) fn truncate_chars(text: &str, max_chars: usize) -> String {
    let mut output = text.chars().take(max_chars).collect::<String>();
    if text.chars().count() > max_chars {
        output.push_str("...");
    }
    output
}

fn is_stop_word(token: &str) -> bool {
    matches!(
        token,
        "a" | "an"
            | "and"
            | "are"
            | "as"
            | "at"
            | "be"
            | "by"
            | "can"
            | "could"
            | "did"
            | "do"
            | "does"
            | "explain"
            | "for"
            | "from"
            | "has"
            | "have"
            | "how"
            | "i"
            | "in"
            | "is"
            | "it"
            | "of"
            | "on"
            | "or"
            | "please"
            | "should"
            | "that"
            | "the"
            | "to"
            | "was"
            | "were"
            | "what"
            | "when"
            | "where"
            | "which"
            | "who"
            | "with"
            | "would"
    )
}
