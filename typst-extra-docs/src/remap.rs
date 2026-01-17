use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use mdbook_markdown::pulldown_cmark::CowStr;
use url::Url;

use crate::Metadata;

pub struct Remapper {
    /// Map from `main` URLs to files.
    url_to_file: HashMap<String, PathBuf>,
    /// Map from files to permanent URLs.
    file_to_url: HashMap<PathBuf, Url>,
}

impl Remapper {
    pub fn new(meta: &Metadata) -> Self {
        let mut url_to_file = HashMap::new();
        let mut file_to_url = HashMap::new();

        for (file, url_str) in &meta.map {
            let url = Url::parse(url_str).expect("valid URL");

            url_to_file.insert(as_main(&url).to_string(), file.clone());
            file_to_url.insert(file.clone(), url);
        }

        Remapper {
            url_to_file,
            file_to_url,
        }
    }

    /// Remap `dst_url` to the correct relative URL.
    ///
    /// Returns `None` if no remapping is needed.
    pub fn remap_link<'a>(&self, dst_url: CowStr<'a>, src_file: &'a PathBuf) -> Option<CowStr<'a>> {
        /*
          Flow of information:
            src_file → src_url
                         ↓
            dst_file ← dst_url
               ↓
            relative link from src_file to dst_file
        */

        // In-page link, no remapping needed.
        if dst_url.starts_with('#') {
            return None;
        }

        // Only remap links from known files.
        let src_url = self.file_to_url.get(src_file)?;

        // Canonicalize `dst_url` based on `src_url`.
        let (dst_url, result_must_some) = match Url::parse(&dst_url) {
            Ok(dst_url) => (dst_url, false),
            Err(url::ParseError::RelativeUrlWithoutBase) => (
                match dst_url.strip_prefix("/") {
                    Some(rel_to_root) => get_repo(src_url)
                        .expect("all sources are in repos")
                        .join(rel_to_root)
                        .expect("valid URL relative to the repo root"),
                    None => src_url
                        .join(&dst_url)
                        .expect("valid URL relative to the source file"),
                },
                true,
            ),
            Err(_) => todo!("Cannot handle invalid destination URLs yet"),
        };

        // Extract anchor from the URL.
        let (dst_url, dst_anchor) = match dst_url.fragment() {
            Some(anchor) => {
                let mut url = dst_url.clone();
                url.set_fragment(None);
                (url, Some(anchor))
            }
            None => (dst_url, None),
        };

        // Map `dst_url` to `dst_file` if possible.
        let Some(dst_file) = self.url_to_file.get(&as_main(&dst_url).to_string()) else {
            // We meet an unknown `dst_url`, skip further processing.
            if result_must_some {
                let mut url = dst_url.clone();
                url.set_fragment(dst_anchor);
                return Some(url.to_string().into());
            } else {
                return None;
            }
        };

        let rel = make_relative(src_file, dst_file)?;
        Some(
            match dst_anchor {
                Some(anchor) => format!("{}#{}", rel, anchor),
                None => rel,
            }
            .into(),
        )
    }
}

/// Change a GitHub file URL from `/blob/COMMIT/` to `/blob/main/`.
fn switch_to_main(url: &mut Url) {
    if url.domain() == Some("github.com")
        && let Some(segments) = url.path_segments()
    {
        let mut segments: Vec<&str> = segments.collect();
        // Segments: `OWNER/REPO/blob/COMMIT/*`
        if segments.len() >= 5 && segments[2] == "blob" {
            segments[3] = "main";
            url.set_path(&segments.join("/"));
        }
    }
}

/// Same as `switch_to_main`, but returns a new URL.
fn as_main(url: &Url) -> Url {
    let mut main_url = url.clone();
    switch_to_main(&mut main_url);
    main_url
}

/// Get the repo base from a GitHub file URL.
fn get_repo(file: &Url) -> Option<Url> {
    if file.domain() == Some("github.com")
        && let Some(segments) = file.path_segments()
    {
        let mut url = file.clone();
        // `OWNER/REPO/blob/COMMIT/`
        let mut path = segments.take(4).collect::<Vec<_>>().join("/");
        path.push('/');
        url.set_path(&path);
        Some(url)
    } else {
        None
    }
}

/// A poor clone of [python's `PurePath.relative_to`](https://docs.python.org/3/library/pathlib.html#pathlib.PurePath.relative_to).
fn make_relative(base: &Path, target: &Path) -> Option<String> {
    let root = Url::parse("magic://ROOT/").unwrap();

    let mut base_url = root.clone();
    base_url.set_path(base.to_str()?);

    let mut target_url = root.clone();
    target_url.set_path(target.to_str()?);

    let rel = base_url.make_relative(&target_url)?;

    assert!(!rel.starts_with("magic://ROOT/"));
    assert!(!rel.starts_with("/"));
    assert!(!rel.is_empty());

    Some(if rel.starts_with("../") {
        rel
    } else {
        format!("./{}", rel)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_make_relative() {
        assert_eq!(
            make_relative(&PathBuf::from("a/b/c.md"), &PathBuf::from("a/d/e.md")),
            Some(String::from("../d/e.md"))
        );
        assert_eq!(
            make_relative(&PathBuf::from("a/b/c/d.md"), &PathBuf::from("a/b/e.md")),
            Some(String::from("../e.md"))
        );
        assert_eq!(
            make_relative(
                &PathBuf::from("a/b/c/d.md"),
                &PathBuf::from("a/b/c/d/e/f.md")
            ),
            Some(String::from("./d/e/f.md"))
        );
    }

    fn build_remapper() -> Remapper {
        let map = r#"{
  "typst/index.md": "https://github.com/typst/typst/blob/701c7f9b2853857cde6f4dd76763b9bb118aff14/README.md",
  "typst/dev/architecture.md": "https://github.com/typst/typst/blob/701c7f9b2853857cde6f4dd76763b9bb118aff14/docs/dev/architecture.md",
  "hayagriva/index.md": "https://github.com/typst/hayagriva/blob/a137441413a5907c15ced44d1502dfb9fa1a3014/README.md",
  "hayagriva/file-format.md": "https://github.com/typst/hayagriva/blob/a137441413a5907c15ced44d1502dfb9fa1a3014/docs/file-format.md",
  "hayagriva/selectors.md": "https://github.com/typst/hayagriva/blob/a137441413a5907c15ced44d1502dfb9fa1a3014/docs/selectors.md",
  "packages/index.md": "https://github.com/typst/packages/blob/8f21d920ae6389359e4734335a107cca0f57c181/README.md",
  "packages/manifest.md": "https://github.com/typst/packages/blob/8f21d920ae6389359e4734335a107cca0f57c181/docs/manifest.md",
  "packages/categories.md": "https://github.com/typst/packages/blob/8f21d920ae6389359e4734335a107cca0f57c181/docs/CATEGORIES.md",
  "packages/disciplines.md": "https://github.com/typst/packages/blob/8f21d920ae6389359e4734335a107cca0f57c181/docs/DISCIPLINES.md",
  "license/typst/LICENSE": "https://github.com/typst/typst/blob/701c7f9b2853857cde6f4dd76763b9bb118aff14/LICENSE"
}"#;
        let map = serde_json::from_str(map).unwrap();
        let meta = Metadata { map };
        Remapper::new(&meta)
    }

    #[test]
    fn test_unchanged() {
        let remapper = build_remapper();

        for unchanged in &[
            "https://repology.org/project/typst/versions",
            "https://github.com/typst/typst/blob/main/CONTRIBUTING.md",
        ] {
            let link = CowStr::Borrowed(unchanged);
            assert_eq!(
                remapper.remap_link(link.clone(), &PathBuf::from("typst/index.md")),
                None,
            );
        }
    }
    #[test]
    fn test_rel_to_rel() {
        let remapper = build_remapper();

        assert_eq!(
            remapper.remap_link(
                CowStr::from("../README.md#local-packages"),
                &PathBuf::from("packages/manifest.md"),
            ),
            Some(CowStr::from("./index.md#local-packages"))
        );
    }
    #[test]
    fn test_abs_to_rel() {
        let remapper = build_remapper();

        assert_eq!(
            remapper.remap_link(
                CowStr::from("https://github.com/typst/packages/blob/main/docs/CATEGORIES.md"),
                &PathBuf::from("packages/manifest.md"),
            ),
            Some(CowStr::from("./categories.md"))
        );
        assert_eq!(
            remapper.remap_link(
                CowStr::from(
                    "https://github.com/typst/hayagriva/blob/main/docs/file-format.md#entry-type"
                ),
                &PathBuf::from("hayagriva/selectors.md"),
            ),
            Some(CowStr::from("./file-format.md#entry-type"))
        );
        assert_eq!(
            remapper.remap_link(
                CowStr::from("https://github.com/typst/typst/blob/main/LICENSE"),
                &PathBuf::from("typst/index.md"),
            ),
            Some(CowStr::from("../license/typst/LICENSE"))
        );
    }
    #[test]
    fn test_rel_to_abs() {
        let remapper = build_remapper();

        assert_eq!(
            remapper.remap_link(
                CowStr::from("/tests"),
                &PathBuf::from("typst/dev/architecture.md"),
            ),
            Some(CowStr::from(
                "https://github.com/typst/typst/blob/701c7f9b2853857cde6f4dd76763b9bb118aff14/tests"
            ))
        );
    }
}
