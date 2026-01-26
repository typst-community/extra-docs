use anyhow::anyhow;
use mdbook_markdown::{
    MarkdownOptions, new_cmark_parser,
    pulldown_cmark::{Event, LinkType, Tag, TagEnd},
};
use mdbook_preprocessor::errors::Result;
use pulldown_cmark_to_cmark::cmark;
use unscanny::Scanner;
use url::Url;

/// Get the issue URL from a `num` in a `file`.
fn as_issue_url(num: &str, file_url: &str) -> Result<String> {
    let file = Url::parse(file_url)?;

    if file.domain() == Some("github.com")
        && let Some(segments) = file.path_segments()
    {
        let mut url = file.clone();
        // `OWNER/REPO/`
        let mut path = segments.take(2).collect::<Vec<_>>().join("/");
        path.push_str("/issues/");
        path.push_str(num);
        url.set_path(&path);
        Ok(url.to_string())
    } else {
        Err(anyhow!("Expect a file URL on GitHub, got: {}", file_url))
    }
}

/// Link issues in `content`.
/// Returns `None` if unchanged.
pub fn link_issues(
    content: &str,
    source_url: &str,
    markdown_options: &MarkdownOptions,
) -> Result<Option<String>> {
    // Texts in links, codes, etc. should be ignored.
    let mut in_tag = false;

    let mut changed = false;
    let mut new_events = Vec::new();

    for event in new_cmark_parser(content, markdown_options) {
        match event {
            Event::Start(Tag::Link { .. } | Tag::CodeBlock(..) | Tag::HtmlBlock) => {
                in_tag = true;
                new_events.push(event);
            }
            Event::End(TagEnd::Link | TagEnd::CodeBlock | TagEnd::HtmlBlock) => {
                in_tag = false;
                new_events.push(event);
            }
            Event::Text(text) if !in_tag => {
                let mut s = Scanner::new(&text);
                while !s.done() {
                    let start = s.cursor();

                    // Eat until `#{num}`
                    while !s.done() {
                        s.eat_until("#");
                        // Allow texts like `rpath.rs#L116-L158`.
                        if s.eat_if("#") && s.peek().is_some_and(|c| c.is_ascii_digit()) {
                            // We meet `#{num}`. Terminate before `#`.
                            s.uneat(); 
                            break;
                        }
                    }
                    new_events.push(Event::Text(s.from(start).to_owned().into()));

                    let issue_start = s.cursor();
                    if s.eat_if("#") {
                        let num = s.eat_while(char::is_ascii_digit);
                        if !num.is_empty() {
                            let url = as_issue_url(num, source_url)?;
                            changed = true;
                            new_events.push(Event::Start(Tag::Link {
                                link_type: LinkType::Inline,
                                dest_url: url.into(),
                                title: "".into(),
                                id: "".into(),
                            }));
                            new_events.push(Event::Text(s.from(issue_start).to_owned().into()));
                            new_events.push(Event::End(TagEnd::Link));
                        } else {
                            unreachable!("eaten `#` but no number follows");
                        }
                    }
                }
            }
            _ => new_events.push(event),
        }
    }

    if !changed {
        Ok(None)
    } else {
        let mut buf = String::with_capacity(content.len());
        Ok(Some(cmark(new_events.into_iter(), &mut buf).map(|_| buf)?))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    const SOURCE_URL: &str = "https://github.com/typst/hayagriva/blob/a137441/CHANGELOG.md";

    #[test]
    fn test_as_issue_url() {
        let issue_url = as_issue_url("123", SOURCE_URL).unwrap();
        assert_eq!(issue_url, "https://github.com/typst/hayagriva/issues/123");
    }

    #[test]
    fn test_link_issues() {
        let content = r#"
`chapter#361` (#361, #383)

offline ([#2089](https://github.com/typst/typst/issues/2089 "Typst documentation available offline? · Issue #2089 · typst/typst"))
"#;
        let expected = r#"
`chapter#361` ([\#361](https://github.com/typst/hayagriva/issues/361), [\#383](https://github.com/typst/hayagriva/issues/383))

offline ([\#2089](https://github.com/typst/typst/issues/2089 "Typst documentation available offline? · Issue #2089 · typst/typst"))
"#.strip_prefix("\n").unwrap().strip_suffix("\n").unwrap().to_string();

        let actual = link_issues(content, SOURCE_URL, &Default::default());
        if let Ok(Some(actual)) = actual {
            assert_eq!(actual, expected);

            let again = link_issues(&actual, SOURCE_URL, &Default::default());
            assert!(matches!(again, Ok(None)));
        } else {
            panic!("link_issues failed: {:?}", actual);
        }
    }

    #[test]
    fn test_unchanged() {
        for content in &[
            r#"offline ([#2089](https://github.com/typst/typst/issues/2089 "Typst documentation available offline? · Issue #2089 · typst/typst"))"#,
            r"function (https://github.com/rust-lang/rust/blob/e1d0de82cc40b666b88d4a6d2c9dcbc81d7ed27f/src/librustc_back/rpath.rs#L116-L158)",
        ] {
            let actual = link_issues(content, SOURCE_URL, &Default::default());
            assert!(
                matches!(actual, Ok(None)),
                "expected unchanged, got {:?}",
                actual
            );
        }
    }
}
