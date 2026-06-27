use memchr::{memmem::Finder, memmem::FinderRev};
use oxc_span::VALID_EXTENSIONS;

use crate::loader::JavaScriptSource;

mod astro;
mod svelte;
mod vue;
pub use astro::AstroPartialLoader;
pub use svelte::SveltePartialLoader;
pub use vue::VuePartialLoader;

const SCRIPT_START: &str = "<script";
const SCRIPT_END: &str = "</script>";
const COMMENT_START: &str = "<!--";
const COMMENT_END: &str = "-->";

/// File extensions that can contain JS/TS code in certain parts, such as in `<script>` tags, and can
/// be loaded using the [`PartialLoader`].
pub const LINT_PARTIAL_LOADER_EXTENSIONS: &[&str] = &["vue", "astro", "svelte"];

/// All valid JavaScript/TypeScript extensions, plus additional framework files that
/// contain JavaScript/TypeScript code in them (e.g., Vue, Astro, Svelte, etc.).
pub const LINTABLE_EXTENSIONS: &[&str] =
    constcat::concat_slices!([&str]: VALID_EXTENSIONS, LINT_PARTIAL_LOADER_EXTENSIONS);

pub struct PartialLoader;

impl PartialLoader {
    /// Extract js section of special files.
    /// Returns `None` if the special file does not have a js section.
    pub fn parse<'a>(ext: &str, source_text: &'a str) -> Option<Vec<JavaScriptSource<'a>>> {
        match ext {
            "vue" => Some(VuePartialLoader::new(source_text).parse()),
            "astro" => Some(AstroPartialLoader::new(source_text).parse()),
            "svelte" => Some(SveltePartialLoader::new(source_text).parse()),
            _ => None,
        }
    }
}

/// Find closing angle for situations where there is another `>` in between.
/// e.g. `<script generic="T extends Record<string, string>">`
/// or `<script attribute="text with > inside">`
/// or `<script onload={() => {}}>`
fn find_script_closing_angle(source_text: &str, pointer: usize) -> Option<usize> {
    let mut open_angle = 0;
    let mut open_brace = 0;
    let mut in_quote: Option<char> = None;

    for (offset, c) in source_text[pointer..].char_indices() {
        match c {
            '"' | '\'' => {
                if let Some(q) = in_quote {
                    if q == c {
                        in_quote = None;
                    }
                } else if open_brace == 0 {
                    in_quote = Some(c);
                }
            }
            '{' if in_quote.is_none() => {
                open_brace += 1;
            }
            '}' if in_quote.is_none() && open_brace > 0 => {
                open_brace -= 1;
            }
            '<' if in_quote.is_none() && open_brace == 0 => {
                open_angle += 1;
            }
            '>' if in_quote.is_none() && open_brace == 0 => {
                if open_angle == 0 {
                    return Some(offset);
                }
                open_angle -= 1;
            }
            _ => {}
        }
    }

    None
}

/// Find the next `<script` after `pointer` that is not inside an HTML comment.
///
/// Returns the offset, relative to `pointer`, of the position right after the matched
/// `<script`, or `None` if there is none.
///
/// `pointer` must not be inside an HTML comment that opened before it. All callers guarantee
/// this: they pass `0`, the end of the Astro frontmatter, or a position past a `<script>`
/// element whose start this function already classified as not commented out.
fn find_script_start(
    source_text: &str,
    pointer: usize,
    script_start_finder: &Finder<'_>,
    comment_start_finder: &FinderRev<'_>,
    comment_end_finder: &Finder<'_>,
) -> Option<usize> {
    let bytes = source_text.as_bytes();
    let mut new_pointer = pointer;
    // Lower bound for the backward `<!--` search.
    //
    // It starts at `pointer` (see the precondition above), and whenever a candidate is rejected
    // because the last `<!--` before it (at `offset`) is unclosed, it advances to `offset`:
    // the last `<!--` before any later candidate is at `offset` or later.
    //
    // Without this bound the backward search rescanned the whole `..new_pointer` prefix for
    // every candidate, which made parsing files with many `<script-*>` custom element tags
    // quadratic in the file size.
    let mut floor = pointer;

    loop {
        new_pointer += script_start_finder.find(&bytes[new_pointer..])? + SCRIPT_START.len();

        if let Some(offset) = comment_start_finder.rfind(&bytes[floor..new_pointer]) {
            let offset = floor + offset;
            if comment_end_finder.find(&bytes[offset + COMMENT_START.len()..new_pointer]).is_some()
            {
                // The last `<!--` before the candidate is closed, so the candidate is real.
                break;
            }
            // The candidate is inside the unclosed comment opened at `offset`.
            floor = offset;
        } else {
            break;
        }
    }

    Some(new_pointer - pointer)
}

enum AttributeValue<'a> {
    Empty,
    Value(&'a str),
}

fn find_attribute<'a>(content: &'a str, target: &str) -> Option<AttributeValue<'a>> {
    let mut rest = content.trim();
    if let Some(stripped) = rest.strip_prefix("<script") {
        rest = stripped;
    }

    loop {
        rest = rest.trim_start_matches(|c: char| c.is_whitespace() || c == '/');
        if rest.is_empty() || rest.starts_with('>') {
            return None;
        }

        let name_end = rest
            .find(|c: char| c.is_whitespace() || matches!(c, '=' | '>' | '/'))
            .unwrap_or(rest.len());
        if name_end == 0 {
            return None;
        }

        let name = &rest[..name_end];
        rest = &rest[name_end..];
        rest = rest.trim_start();

        let value = if let Some(stripped) = rest.strip_prefix('=') {
            rest = stripped.trim_start();

            match rest.chars().next() {
                Some('"' | '\'') => {
                    let quote = rest.chars().next().unwrap();
                    rest = &rest[quote.len_utf8()..];
                    let end = rest.find(quote)?;
                    let value = &rest[..end];
                    rest = &rest[end + quote.len_utf8()..];
                    AttributeValue::Value(value)
                }
                Some(_) => {
                    let end = rest
                        .find(|c: char| c.is_whitespace() || matches!(c, '>' | '/'))
                        .unwrap_or(rest.len());
                    let value = &rest[..end];
                    rest = &rest[end..];
                    AttributeValue::Value(value)
                }
                None => return None,
            }
        } else {
            AttributeValue::Empty
        };

        if name.eq_ignore_ascii_case(target) {
            return Some(value);
        }
    }
}

#[cfg(test)]
mod test {
    use memchr::memmem::{Finder, FinderRev};

    use super::{COMMENT_END, COMMENT_START, SCRIPT_START, find_script_start};

    /// Returns the absolute position right after the matched `<script`, if any.
    fn find(source_text: &str, pointer: usize) -> Option<usize> {
        let script_start_finder = Finder::new(SCRIPT_START);
        let comment_start_finder = FinderRev::new(COMMENT_START);
        let comment_end_finder = Finder::new(COMMENT_END);
        find_script_start(
            source_text,
            pointer,
            &script_start_finder,
            &comment_start_finder,
            &comment_end_finder,
        )
        .map(|offset| pointer + offset)
    }

    #[test]
    fn test_find_script_start_skips_commented_out_candidate() {
        // The first `<script` candidate sits inside a closed comment; the second one is real.
        let source = "<!-- <script>nope</script> --> <p/> <script>yes</script>";
        let expected = source.find("<script>yes").unwrap() + SCRIPT_START.len();
        assert_eq!(find(source, 0), Some(expected));
    }

    #[test]
    fn test_find_script_start_inside_unclosed_comment_is_none() {
        // The only `<script` candidate sits inside a comment that never closes.
        let source = "<p/>\n<!--\n<script>nope</script>\n";
        assert_eq!(find(source, 0), None);
    }

    #[test]
    fn test_find_script_start_closed_then_unclosed_comment() {
        let source = "<!-- a --> <script>x</script> <!-- <script>y</script>";
        // The comment before `x` is closed, so `x` is found.
        let expected = source.find("<script>x").unwrap() + SCRIPT_START.len();
        assert_eq!(find(source, 0), Some(expected));
        // Searching after `x` only sees `y`, which is inside an unclosed comment.
        let after_x = source.find("</script>").unwrap() + "</script>".len();
        assert_eq!(find(source, after_x), None);
    }

    #[test]
    fn test_find_script_start_consecutive_calls_advance_past_closed_comments() {
        // Two real scripts separated by a closed comment that contains a fake one.
        let source = "<script>a</script><!-- <script>b</script> --><script>c</script>";
        assert_eq!(find(source, 0), Some(SCRIPT_START.len()));
        // Resume right after `a`'s `</script>`, exactly like the callers do.
        let after_a = source.find("</script>").unwrap() + "</script>".len();
        let expected = source.rfind("<script>c").unwrap() + SCRIPT_START.len();
        assert_eq!(find(source, after_a), Some(expected));
    }
}
