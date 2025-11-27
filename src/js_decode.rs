use std::error::Error;

/// Decode JavaScript string literal to plain text
/// Handles: \xNN hex escapes, \uNNNN unicode escapes, \\u double escapes, invalid escapes like \-
pub fn decode_js_string(input: &str) -> Result<String, Box<dyn Error>> {
    let mut result = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '\\' {
            match chars.peek() {
                Some('x') => {
                    // Hex escape: \xNN
                    chars.next(); // consume 'x'
                    let hex: String = chars.by_ref().take(2).collect();
                    if hex.len() == 2 {
                        if let Ok(byte) = u8::from_str_radix(&hex, 16) {
                            result.push(byte as char);
                        } else {
                            return Err(format!("Invalid hex escape: \\x{}", hex).into());
                        }
                    } else {
                        return Err(format!("Incomplete hex escape: \\x{}", hex).into());
                    }
                }
                Some('u') => {
                    // Unicode escape: \uNNNN
                    chars.next(); // consume 'u'
                    let hex: String = chars.by_ref().take(4).collect();
                    if hex.len() == 4 {
                        if let Ok(code_point) = u32::from_str_radix(&hex, 16) {
                            if let Some(unicode_char) = char::from_u32(code_point) {
                                result.push(unicode_char);
                            } else {
                                return Err(
                                    format!("Invalid unicode code point: \\u{}", hex).into()
                                );
                            }
                        } else {
                            return Err(format!("Invalid unicode escape: \\u{}", hex).into());
                        }
                    } else {
                        return Err(format!("Incomplete unicode escape: \\u{}", hex).into());
                    }
                }
                Some('\\') => {
                    // Check for double-escaped unicode: \\uNNNN
                    chars.next(); // consume first \
                    if chars.peek() == Some(&'u') {
                        chars.next(); // consume 'u'
                        let hex: String = chars.by_ref().take(4).collect();
                        if hex.len() == 4 {
                            if let Ok(code_point) = u32::from_str_radix(&hex, 16) {
                                if let Some(unicode_char) = char::from_u32(code_point) {
                                    result.push(unicode_char);
                                } else {
                                    return Err(format!(
                                        "Invalid unicode code point: \\\\u{}",
                                        hex
                                    )
                                    .into());
                                }
                            } else {
                                return Err(format!("Invalid unicode escape: \\\\u{}", hex).into());
                            }
                        } else {
                            return Err(format!("Incomplete unicode escape: \\\\u{}", hex).into());
                        }
                    } else {
                        // Just a regular backslash
                        result.push('\\');
                    }
                }
                // Standard JSON escapes
                Some('n') => {
                    chars.next();
                    result.push('\n');
                }
                Some('r') => {
                    chars.next();
                    result.push('\r');
                }
                Some('t') => {
                    chars.next();
                    result.push('\t');
                }
                Some('"') => {
                    chars.next();
                    result.push('"');
                }
                Some('\'') => {
                    chars.next();
                    result.push('\'');
                }
                Some('b') => {
                    chars.next();
                    result.push('\u{0008}');
                }
                Some('f') => {
                    chars.next();
                    result.push('\u{000C}');
                }
                Some('v') => {
                    chars.next();
                    result.push('\u{000B}');
                }
                Some('0') => {
                    chars.next();
                    result.push('\0');
                }
                // Invalid escapes that JavaScript allows but JSON doesn't
                Some('-') => {
                    chars.next(); // consume '-'
                    result.push('-');
                }
                Some('/') => {
                    chars.next(); // consume '/'
                    result.push('/');
                }
                Some(&next_ch) => {
                    // Unknown escape, just output the character literally
                    chars.next();
                    result.push(next_ch);
                }
                None => {
                    result.push('\\');
                }
            }
        } else {
            result.push(ch);
        }
    }

    Ok(result)
}

/// Attempt to fix UTF-8 mojibake (text encoded as UTF-8 but interpreted as Latin-1)
pub fn fix_mojibake(input: &str) -> String {
    // Try to re-encode as latin-1, then decode as UTF-8
    let bytes: Vec<u8> = input
        .chars()
        .filter_map(|c| {
            let code = c as u32;
            if code <= 255 { Some(code as u8) } else { None }
        })
        .collect();

    match String::from_utf8(bytes) {
        Ok(fixed) => fixed,
        Err(_) => input.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hex_escapes() {
        let input = r#"[\x22Salary\x22:\x2250000$\x22]"#;
        let expected = r#"["Salary":"50000$"]"#;
        assert_eq!(decode_js_string(input).unwrap(), expected);
    }

    #[test]
    fn test_unicode_escapes() {
        let input = r#"Title\u2013Profil"#;
        let expected = "Title–Profil";
        assert_eq!(decode_js_string(input).unwrap(), expected);
    }

    #[test]
    fn test_double_unicode_escapes() {
        let input = r#"Title\\u2013Profil"#;
        let expected = "Title–Profil";
        assert_eq!(decode_js_string(input).unwrap(), expected);
    }

    #[test]
    fn test_invalid_escapes() {
        let input = r#"50000$\-80000$"#;
        let expected = "50000$-80000$";
        assert_eq!(decode_js_string(input).unwrap(), expected);
    }

    #[test]
    fn test_combined() {
        let input =
            r#"[\x22Salary\x22:\x2250000$ \- 80000$\x22,\x22Langue\x22:[\x22Français\x22]]"#;
        let expected = r#"["Salary":"50000$ - 80000$","Langue":["Français"]]"#;
        assert_eq!(decode_js_string(input).unwrap(), expected);
    }
}
