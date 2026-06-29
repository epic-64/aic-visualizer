//! Parse a free-form turn description into token counts.
//!
//! Examples it understands:
//!   "300 tokens prompt, 12000 tokens tool inputs, 4000 tokens response"
//!   "300 prompt, 12k tool input, 4k response"
//!   "input 500, output 2000, cached 10000"

/// Token counts extracted from a turn description.
#[derive(Default, Debug, Clone)]
pub struct ParsedTurn {
    /// Fresh (non-cached) input tokens this turn: prompts + tool inputs.
    pub input: u64,
    /// Output / response tokens this turn.
    pub output: u64,
    /// Thinking / reasoning tokens this turn. Billed at the output rate, but
    /// they don't carry into the next turn's context (unlike `output`).
    pub thinking: u64,
    /// Explicit cached-token override. When `None`, the conversation's
    /// carried-over cache is used instead.
    pub cached_override: Option<u64>,
    /// How many times to apply this turn (from a `repeat N` segment).
    pub repeat: u64,
}

/// Parse a number that may use a `k`/`m` suffix, e.g. "12k", "1.5k", "300".
fn parse_number(token: &str) -> Option<u64> {
    let t = token.trim().to_lowercase();
    let t = t.trim_end_matches("tokens").trim_end_matches("token").trim();
    let (digits, mult) = if let Some(s) = t.strip_suffix('k') {
        (s, 1_000.0)
    } else if let Some(s) = t.strip_suffix('m') {
        (s, 1_000_000.0)
    } else {
        (t, 1.0)
    };
    let digits: String = digits.chars().filter(|c| c.is_ascii_digit() || *c == '.').collect();
    if digits.is_empty() {
        return None;
    }
    digits.parse::<f64>().ok().map(|v| (v * mult).round() as u64)
}

#[derive(PartialEq)]
enum Kind {
    Input,
    Output,
    Thinking,
    Cached,
}

/// Classify a segment by the keywords it contains.
///
/// Thinking: think / thinking / reason / reasoning
/// Output: out / output / response / completion / answer
/// Cached: anything containing "cach"
/// Input (the default): in / input / prompt / tools / instructions / ...
fn classify(segment: &str) -> Kind {
    let s = segment.to_lowercase();
    if s.contains("cach") {
        Kind::Cached
    } else if s.contains("think") || s.contains("reason") {
        Kind::Thinking
    } else if s.contains("out") || s.contains("response") || s.contains("completion") || s.contains("answer") {
        Kind::Output
    } else {
        Kind::Input
    }
}

/// Find the first number-like word inside a segment.
fn first_number(segment: &str) -> Option<u64> {
    for word in segment.split_whitespace() {
        if word.chars().next().map(|c| c.is_ascii_digit()).unwrap_or(false) {
            if let Some(n) = parse_number(word) {
                return Some(n);
            }
        }
    }
    None
}

/// Parse the whole line. Returns `None` if no numbers were found at all.
pub fn parse(line: &str) -> Option<ParsedTurn> {
    let mut out = ParsedTurn { repeat: 1, ..Default::default() };
    let mut found = false;

    for segment in line.split([',', ';']) {
        let lower = segment.to_lowercase();
        // A "repeat N" / "N times" segment sets the repeat count rather than
        // contributing any tokens.
        if lower.contains("repeat") || lower.contains("times") {
            if let Some(n) = first_number(segment) {
                out.repeat = n.max(1);
            }
            continue;
        }

        let Some(n) = first_number(segment) else {
            continue;
        };
        found = true;
        match classify(segment) {
            Kind::Input => out.input += n,
            Kind::Output => out.output += n,
            Kind::Thinking => out.thinking += n,
            Kind::Cached => out.cached_override = Some(out.cached_override.unwrap_or(0) + n),
        }
    }

    if found {
        Some(out)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn example_line() {
        let p = parse("300 tokens prompt, 12000 tokens tool inputs, 4000 tokens response").unwrap();
        assert_eq!(p.input, 12_300); // prompt + tool inputs
        assert_eq!(p.output, 4_000);
        assert_eq!(p.cached_override, None);
    }

    #[test]
    fn short_synonyms() {
        let p = parse("300 prompt, 5000 tools, 400 out").unwrap();
        assert_eq!(p.input, 5_300); // prompt + tools
        assert_eq!(p.output, 400);

        let p = parse("200 in, 500 instructions, 300 response").unwrap();
        assert_eq!(p.input, 700); // in + instructions
        assert_eq!(p.output, 300);
    }

    #[test]
    fn k_suffix_and_cached() {
        let p = parse("input 1.5k, output 4k, cached 10k").unwrap();
        assert_eq!(p.input, 1_500);
        assert_eq!(p.output, 4_000);
        assert_eq!(p.cached_override, Some(10_000));
    }

    #[test]
    fn repeat_count() {
        let p = parse("300 prompt, 5000 tools, 400 out, repeat 10").unwrap();
        assert_eq!(p.input, 5_300);
        assert_eq!(p.output, 400);
        assert_eq!(p.repeat, 10);
    }

    #[test]
    fn default_repeat_is_one() {
        assert_eq!(parse("100 in, 200 out").unwrap().repeat, 1);
    }

    #[test]
    fn thinking_tokens() {
        let p = parse("300 prompt, 2000 thinking, 400 out").unwrap();
        assert_eq!(p.input, 300);
        assert_eq!(p.thinking, 2_000);
        assert_eq!(p.output, 400);

        // "reasoning" is a synonym and shouldn't fall through to output.
        let p = parse("500 reasoning, 1000 response").unwrap();
        assert_eq!(p.thinking, 500);
        assert_eq!(p.output, 1_000);
    }

    #[test]
    fn no_numbers() {
        assert!(parse("hello there").is_none());
    }
}
