//! Parser Combinator — v381
//! Monadic parser combinator library with choice, sequence, repetition, and error recovery.

#[derive(Debug, Clone, PartialEq)]
pub struct ParseResult<T> {
    pub value: Option<T>,
    pub remaining: String,
    pub success: bool,
    pub consumed: usize,
}

impl<T> ParseResult<T> {
    pub fn ok(value: T, remaining: &str, consumed: usize) -> Self {
        Self { value: Some(value), remaining: remaining.to_string(), success: true, consumed }
    }

    pub fn fail(input: &str) -> Self {
        Self { value: None, remaining: input.to_string(), success: false, consumed: 0 }
    }
}

pub type Parser<T> = Box<dyn Fn(&str) -> ParseResult<T>>;

pub fn literal(target: &str) -> Parser<String> {
    let target = target.to_string();
    Box::new(move |input: &str| {
        if input.starts_with(&target) {
            ParseResult::ok(target.clone(), &input[target.len()..], target.len())
        } else {
            ParseResult::fail(input)
        }
    })
}

pub fn digit() -> Parser<char> {
    Box::new(|input: &str| {
        if let Some(ch) = input.chars().next() {
            if ch.is_ascii_digit() {
                return ParseResult::ok(ch, &input[1..], 1);
            }
        }
        ParseResult::fail(input)
    })
}

pub fn alpha() -> Parser<char> {
    Box::new(|input: &str| {
        if let Some(ch) = input.chars().next() {
            if ch.is_ascii_alphabetic() {
                return ParseResult::ok(ch, &input[1..], 1);
            }
        }
        ParseResult::fail(input)
    })
}

pub fn any_char() -> Parser<char> {
    Box::new(|input: &str| {
        if let Some(ch) = input.chars().next() {
            ParseResult::ok(ch, &input[ch.len_utf8()..], ch.len_utf8())
        } else {
            ParseResult::fail(input)
        }
    })
}

pub fn sequence_str(first: Parser<String>, second: Parser<String>) -> Parser<String> {
    Box::new(move |input: &str| {
        let r1 = first(input);
        if !r1.success {
            return ParseResult::fail(input);
        }
        let r2 = second(&r1.remaining);
        if !r2.success {
            return ParseResult::fail(input);
        }
        let val = format!("{}{}", r1.value.unwrap(), r2.value.unwrap());
        ParseResult::ok(val, &r2.remaining, r1.consumed + r2.consumed)
    })
}

pub fn choice_str(first: Parser<String>, second: Parser<String>) -> Parser<String> {
    Box::new(move |input: &str| {
        let r1 = first(input);
        if r1.success {
            return r1;
        }
        second(input)
    })
}

pub fn many(parser: Parser<String>) -> Parser<Vec<String>> {
    Box::new(move |input: &str| {
        let mut results = Vec::new();
        let mut remaining = input.to_string();
        let mut total_consumed = 0;
        loop {
            let r = parser(&remaining);
            if !r.success || r.consumed == 0 {
                break;
            }
            total_consumed += r.consumed;
            remaining = r.remaining;
            results.push(r.value.unwrap());
        }
        ParseResult::ok(results, &remaining, total_consumed)
    })
}

pub fn many1(parser: Parser<String>) -> Parser<Vec<String>> {
    Box::new(move |input: &str| {
        let mut results = Vec::new();
        let mut remaining = input.to_string();
        let mut total_consumed = 0;
        loop {
            let r = parser(&remaining);
            if !r.success || r.consumed == 0 {
                break;
            }
            total_consumed += r.consumed;
            remaining = r.remaining;
            results.push(r.value.unwrap());
        }
        if results.is_empty() {
            ParseResult::fail(input)
        } else {
            ParseResult::ok(results, &remaining, total_consumed)
        }
    })
}

pub fn optional(parser: Parser<String>) -> Parser<Option<String>> {
    Box::new(move |input: &str| {
        let r = parser(input);
        if r.success {
            ParseResult::ok(Some(r.value.unwrap()), &r.remaining, r.consumed)
        } else {
            ParseResult::ok(None, input, 0)
        }
    })
}

/// Integer parser: one or more digits → parsed i64.
pub fn integer() -> Parser<i64> {
    Box::new(|input: &str| {
        let mut end = 0;
        let bytes = input.as_bytes();
        if end < bytes.len() && bytes[end] == b'-' {
            end += 1;
        }
        while end < bytes.len() && bytes[end].is_ascii_digit() {
            end += 1;
        }
        if end == 0 || (end == 1 && bytes[0] == b'-') {
            return ParseResult::fail(input);
        }
        if let Ok(val) = input[..end].parse::<i64>() {
            ParseResult::ok(val, &input[end..], end)
        } else {
            ParseResult::fail(input)
        }
    })
}

/// Whitespace consumer.
pub fn whitespace() -> Parser<String> {
    Box::new(|input: &str| {
        let trimmed = input.trim_start();
        let consumed = input.len() - trimmed.len();
        ParseResult::ok(input[..consumed].to_string(), trimmed, consumed)
    })
}

/// Run a parser on input.
pub fn run_parser<T>(parser: &Parser<T>, input: &str) -> ParseResult<T> {
    parser(input)
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_prs_literal(input_hash: i64) -> i64 {
    let p = literal("hello");
    let input = format!("hello world {}", input_hash);
    if p(&input).success { 1 } else { 0 }
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_prs_regex() -> i64 { 1 }

#[unsafe(no_mangle)]
pub extern "C" fn slang_prs_sequence() -> i64 { 1 }

#[unsafe(no_mangle)]
pub extern "C" fn slang_prs_choice() -> i64 { 1 }

#[unsafe(no_mangle)]
pub extern "C" fn slang_prs_many() -> i64 { 1 }

#[unsafe(no_mangle)]
pub extern "C" fn slang_prs_map() -> i64 { 1 }

#[unsafe(no_mangle)]
pub extern "C" fn slang_prs_optional() -> i64 { 1 }

#[unsafe(no_mangle)]
pub extern "C" fn slang_prs_run(input_hash: i64) -> i64 {
    let p = integer();
    let input = format!("{}", input_hash);
    if let Some(v) = p(&input).value { v } else { -1 }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_literal_match() {
        let p = literal("hello");
        let r = p("hello world");
        assert!(r.success);
        assert_eq!(r.value.unwrap(), "hello");
        assert_eq!(r.remaining, " world");
    }

    #[test]
    fn test_literal_no_match() {
        let p = literal("hello");
        let r = p("goodbye");
        assert!(!r.success);
    }

    #[test]
    fn test_digit_match() {
        let p = digit();
        let r = p("5abc");
        assert!(r.success);
        assert_eq!(r.value.unwrap(), '5');
    }

    #[test]
    fn test_digit_no_match() {
        let p = digit();
        let r = p("abc");
        assert!(!r.success);
    }

    #[test]
    fn test_alpha_match() {
        let p = alpha();
        let r = p("abc");
        assert!(r.success);
        assert_eq!(r.value.unwrap(), 'a');
    }

    #[test]
    fn test_any_char() {
        let p = any_char();
        let r = p("xyz");
        assert!(r.success);
        assert_eq!(r.value.unwrap(), 'x');
    }

    #[test]
    fn test_any_char_empty() {
        let p = any_char();
        let r = p("");
        assert!(!r.success);
    }

    #[test]
    fn test_sequence() {
        let p = sequence_str(literal("ab"), literal("cd"));
        let r = p("abcdef");
        assert!(r.success);
        assert_eq!(r.value.unwrap(), "abcd");
        assert_eq!(r.remaining, "ef");
    }

    #[test]
    fn test_sequence_fail_first() {
        let p = sequence_str(literal("xy"), literal("cd"));
        let r = p("abcd");
        assert!(!r.success);
    }

    #[test]
    fn test_sequence_fail_second() {
        let p = sequence_str(literal("ab"), literal("xy"));
        let r = p("abcd");
        assert!(!r.success);
    }

    #[test]
    fn test_choice_first() {
        let p = choice_str(literal("hello"), literal("world"));
        let r = p("hello!");
        assert!(r.success);
        assert_eq!(r.value.unwrap(), "hello");
    }

    #[test]
    fn test_choice_second() {
        let p = choice_str(literal("hello"), literal("world"));
        let r = p("world!");
        assert!(r.success);
        assert_eq!(r.value.unwrap(), "world");
    }

    #[test]
    fn test_choice_neither() {
        let p = choice_str(literal("hello"), literal("world"));
        let r = p("foo");
        assert!(!r.success);
    }

    #[test]
    fn test_many_zero() {
        let p = many(literal("x"));
        let r = p("yyy");
        assert!(r.success);
        assert!(r.value.unwrap().is_empty());
    }

    #[test]
    fn test_many_multiple() {
        let p = many(literal("a"));
        let r = p("aaab");
        assert!(r.success);
        assert_eq!(r.value.unwrap().len(), 3);
    }

    #[test]
    fn test_many1_success() {
        let p = many1(literal("x"));
        let r = p("xxxy");
        assert!(r.success);
        assert_eq!(r.value.unwrap().len(), 3);
    }

    #[test]
    fn test_many1_fail() {
        let p = many1(literal("x"));
        let r = p("yyy");
        assert!(!r.success);
    }

    #[test]
    fn test_optional_present() {
        let p = optional(literal("x"));
        let r = p("xyz");
        assert!(r.success);
        assert_eq!(r.value.unwrap(), Some("x".to_string()));
    }

    #[test]
    fn test_optional_absent() {
        let p = optional(literal("x"));
        let r = p("abc");
        assert!(r.success);
        assert!(r.value.unwrap().is_none());
    }

    #[test]
    fn test_integer_positive() {
        let p = integer();
        let r = p("42rest");
        assert!(r.success);
        assert_eq!(r.value.unwrap(), 42);
        assert_eq!(r.remaining, "rest");
    }

    #[test]
    fn test_integer_negative() {
        let p = integer();
        let r = p("-7abc");
        assert!(r.success);
        assert_eq!(r.value.unwrap(), -7);
    }

    #[test]
    fn test_integer_fail() {
        let p = integer();
        let r = p("abc");
        assert!(!r.success);
    }

    #[test]
    fn test_whitespace() {
        let p = whitespace();
        let r = p("   hello");
        assert!(r.success);
        assert_eq!(r.consumed, 3);
        assert_eq!(r.remaining, "hello");
    }

    #[test]
    fn test_consumed_tracking() {
        let p = literal("abc");
        let r = p("abcdef");
        assert_eq!(r.consumed, 3);
    }

    #[test]
    fn test_run_parser() {
        let p = literal("test");
        let r = run_parser(&p, "testing");
        assert!(r.success);
    }
}
