//! Normalize static select arms before the indentation pass. Token spans keep
//! comments and literal bytes intact; dynamic `select from` is untouched.
use crate::FormatError;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy)]
struct Token<'a> {
    text: &'a str,
    start: usize,
    end: usize,
}

fn tokens(source: &str) -> Vec<Token<'_>> {
    let mut result = Vec::new();
    let mut chars = source.char_indices().peekable();
    while let Some((start, c)) = chars.next() {
        if c.is_whitespace() {
            continue;
        }
        if c == '/' && chars.peek().is_some_and(|(_, c)| *c == '/') {
            for (_, c) in chars.by_ref() {
                if c == '\n' {
                    break;
                }
            }
            continue;
        }
        if c == '/' && chars.peek().is_some_and(|(_, c)| *c == '*') {
            chars.next();
            while let Some((_, c)) = chars.next() {
                if c == '*' && chars.peek().is_some_and(|(_, c)| *c == '/') {
                    chars.next();
                    break;
                }
            }
            continue;
        }
        let mut end = start + c.len_utf8();
        if matches!(c, '"' | '\'' | '`') {
            let mut escaped = false;
            for (pos, next) in chars.by_ref() {
                end = pos + next.len_utf8();
                if escaped {
                    escaped = false;
                } else if next == '\\' {
                    escaped = true;
                } else if next == c {
                    break;
                }
            }
        } else if c.is_alphanumeric() || c == '_' {
            while let Some(&(pos, next)) = chars.peek() {
                if !next.is_alphanumeric() && next != '_' {
                    break;
                }
                chars.next();
                end = pos + next.len_utf8();
            }
        }
        result.push(Token {
            text: &source[start..end],
            start,
            end,
        });
    }
    result
}

fn line_start(source: &str, pos: usize) -> usize {
    source[..pos].rfind('\n').map_or(0, |i| i + 1)
}

fn break_before(source: &str, pos: usize, edits: &mut BTreeMap<usize, String>) {
    if !source[line_start(source, pos)..pos].trim().is_empty() {
        if edits
            .range(..=pos)
            .next_back()
            .is_some_and(|(&previous, edit)| {
                edit.ends_with('\n') && source[previous..pos].trim().is_empty()
            })
        {
            return;
        }
        edits.entry(pos).or_default().push('\n');
    }
}

fn break_after(source: &str, pos: usize, edits: &mut BTreeMap<usize, String>) {
    let rest = source[pos..].split('\n').next().unwrap_or("").trim();
    // A trailing comment belongs to this header/closer.
    if !rest.is_empty() && !rest.starts_with("//") && !rest.starts_with("/*") {
        edits.entry(pos).or_default().push('\n');
    }
}

fn matching_brace(tokens: &[Token<'_>], open: usize) -> Option<usize> {
    let mut depth = 0usize;
    for (i, token) in tokens.iter().enumerate().skip(open) {
        match token.text {
            "{" => depth += 1,
            "}" => {
                depth = depth.checked_sub(1)?;
                if depth == 0 {
                    return Some(i);
                }
            }
            _ => {}
        }
    }
    None
}

pub(crate) fn normalize(source: &str) -> Result<String, FormatError> {
    let tokens = tokens(source);
    let mut edits = BTreeMap::<usize, String>::new();
    let mut removed = BTreeSet::new();
    for (i, token) in tokens.iter().enumerate() {
        if token.text != "select" || (i > 0 && tokens[i - 1].text == ".") {
            continue;
        }
        let mut open = i + 1;
        if tokens
            .get(open)
            .is_some_and(|t| matches!(t.text, "first" | "fair" | "random"))
        {
            open += 1;
        }
        if tokens.get(open).is_none_or(|t| t.text != "{") {
            continue;
        }
        let error = |pos: usize, message: &str| {
            FormatError::new(
                source[..pos].bytes().filter(|b| *b == b'\n').count() + 1,
                message,
            )
        };
        let close = matching_brace(&tokens, open)
            .ok_or_else(|| error(token.start, "unclosed static select"))?;
        break_after(source, tokens[open].end, &mut edits);
        break_before(source, tokens[close].start, &mut edits);
        let mut arm = open + 1;
        while arm < close {
            if !matches!(tokens[arm].text, "case" | "default") {
                return Err(error(
                    tokens[arm].start,
                    "expected case/default inside select",
                ));
            }
            break_before(source, tokens[arm].start, &mut edits);
            let read = tokens.get(arm + 1).is_some_and(|t| t.text == "readch");
            if tokens[arm].text == "case"
                && !read
                && tokens.get(arm + 1).is_none_or(|t| t.text != "writech")
            {
                return Err(error(
                    tokens[arm].start,
                    "select case must start with readch or writech",
                ));
            }
            // Find the header separator, skipping nested expressions.
            let mut header = arm + 1;
            let mut nesting = Vec::new();
            let mut ternaries = 0usize;
            while header < close {
                match tokens[header].text {
                    "?" if nesting.is_empty() && tokens[header - 1].text != "as" => {
                        ternaries += 1;
                    }
                    ":" if nesting.is_empty() && ternaries > 0 => ternaries -= 1,
                    ":" if nesting.is_empty() => break,
                    "(" | "[" | "{" => nesting.push(tokens[header].text),
                    ")" | "]" | "}" => {
                        nesting.pop();
                    }
                    "case" | "default" if nesting.is_empty() => break,
                    _ => {}
                }
                header += 1;
            }
            if header == close || tokens[header].text != ":" {
                return Err(error(tokens[arm].start, "select arm requires ':'"));
            }
            let mut body = header + 1;
            if read
                && tokens
                    .get(body)
                    .is_some_and(|t| matches!(t.text, "val" | "let" | "const"))
            {
                if !tokens.get(body + 1).is_some_and(|t| {
                    t.text
                        .chars()
                        .next()
                        .is_some_and(|c| c.is_alphabetic() || c == '_')
                        && t.text.chars().all(|c| c.is_alphanumeric() || c == '_')
                }) {
                    return Err(error(
                        tokens[arm].start,
                        "readch case binding requires a name",
                    ));
                }
                body += 2; // binding kind and name
                if tokens.get(body).is_some_and(|t| t.text == ";") {
                    removed.insert(tokens[body].start);
                    body += 1;
                }
            }
            let header_end = if body > 0 && tokens[body - 1].text == ";" {
                tokens[body - 2].end
            } else {
                tokens[body - 1].end
            };
            if body < close && tokens[body].text == "{" {
                let end = matching_brace(&tokens, body)
                    .filter(|end| *end < close)
                    .ok_or_else(|| error(tokens[body].start, "unclosed select arm body"))?;
                break_after(source, tokens[body].end, &mut edits);
                if body + 1 < end {
                    break_before(source, tokens[body + 1].start, &mut edits);
                }
                break_before(source, tokens[end].start, &mut edits);
                break_after(source, tokens[end].end, &mut edits);
                arm = end + 1;
            } else {
                edits.entry(header_end).or_default().push_str(" {");
                break_after(source, header_end, &mut edits);
                if body < close && !matches!(tokens[body].text, "case" | "default") {
                    break_before(source, tokens[body].start, &mut edits);
                }
                let mut end = body;
                let mut depth = 0usize;
                while end < close {
                    match tokens[end].text {
                        "case" | "default" if depth == 0 => break,
                        "{" => depth += 1,
                        "}" => depth = depth.saturating_sub(1),
                        _ => {}
                    }
                    end += 1;
                }
                let pos = tokens[end].start;
                let start = line_start(source, pos);
                let insertion = if source[start..pos].trim().is_empty() {
                    start
                } else {
                    pos
                };
                edits.entry(insertion).or_default().push_str("\n}\n");
                arm = end;
            }
        }
    }
    let mut output = String::new();
    for (pos, c) in source.char_indices() {
        if let Some(edit) = edits.get(&pos) {
            output.push_str(edit);
        }
        if !removed.contains(&pos) {
            output.push(c);
        }
    }
    if let Some(edit) = edits.get(&source.len()) {
        output.push_str(edit);
    }
    Ok(output)
}
