use anyhow::{Context, Result, bail};
use mdbook_preprocessor::book::{Book, Chapter};
use mdbook_preprocessor::{Preprocessor, PreprocessorContext};
use serde::Serialize;
use std::collections::HashSet;
use std::path::Path;

const TRACK_OPEN: &str = "{{#track}}";
const TRACK_CLOSE: &str = "{{/track}}";
const TRACK_ITEM_OPEN: &str = "{{#track-item";
const TRACK_ITEM_CLOSE: &str = "{{/track-item}}";
const TRACK_CHECKLIST: &str = "{{#track-checklist}}";
const OVERVIEW: &str = "{{#track-overview}}";

#[derive(Debug, Default)]
pub struct TrackPreprocessor;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
struct TrackItem {
    id: String,
    label: String,
}

#[derive(Debug, Clone)]
struct InlineTrackItem {
    start: usize,
    end: usize,
    item: TrackItem,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
struct PageManifest {
    id: String,
    title: String,
    section: String,
    items: Vec<TrackItem>,
}

#[derive(Debug, Serialize)]
struct TrackManifest<'a> {
    version: u8,
    book_id: &'a str,
    pages: &'a [PageManifest],
}

impl TrackPreprocessor {
    pub fn new() -> Self {
        Self
    }
}

impl Preprocessor for TrackPreprocessor {
    fn name(&self) -> &str {
        "track"
    }

    fn run(&self, ctx: &PreprocessorContext, mut book: Book) -> Result<Book> {
        let book_id = configured_book_id(ctx)?;

        let pages: Vec<PageManifest> = book
            .chapters()
            .filter_map(|chapter| match page_manifest(chapter) {
                Ok(page) => page,
                Err(error) => {
                    eprintln!("mdbook-track: {error:#}");
                    None
                }
            })
            .collect();

        let manifest = TrackManifest {
            version: 1,
            book_id: &book_id,
            pages: &pages,
        };
        let manifest_json = serde_json::to_string(&manifest)?;

        book.for_each_chapter_mut(|chapter| {
            if let Err(error) = process_chapter(chapter, &book_id, &manifest_json) {
                eprintln!("mdbook-track: {error:#}");
            }
        });

        Ok(book)
    }

    fn supports_renderer(&self, renderer: &str) -> Result<bool> {
        Ok(renderer == "html")
    }
}

fn configured_book_id(ctx: &PreprocessorContext) -> Result<String> {
    if let Some(id) = ctx
        .config
        .get::<String>("preprocessor.track.book-id")
        .context("failed to read preprocessor.track.book-id")?
    {
        let id = id.trim();
        if id.is_empty() {
            bail!("preprocessor.track.book-id must not be empty");
        }
        return Ok(id.to_owned());
    }

    let title = ctx
        .config
        .book
        .title
        .as_deref()
        .unwrap_or("mdbook")
        .trim();

    let fallback = slugify(title);
    Ok(if fallback.is_empty() {
        "mdbook".to_owned()
    } else {
        fallback
    })
}

fn page_manifest(chapter: &Chapter) -> Result<Option<PageManifest>> {
    let items = collect_page_items(&chapter.content)?;
    if items.is_empty() {
        return Ok(None);
    }

    let id = chapter_page_id(chapter)?;
    let section = chapter_section(chapter);

    Ok(Some(PageManifest {
        id,
        title: chapter.name.clone(),
        section,
        items,
    }))
}

fn process_chapter(chapter: &mut Chapter, book_id: &str, manifest_json: &str) -> Result<()> {
    let page_id = chapter_page_id(chapter).ok();
    let inline_items = parse_inline_track_items(&chapter.content)?
        .into_iter()
        .map(|entry| entry.item)
        .collect::<Vec<_>>();

    if let (Some(items), Some(page_id)) = (parse_track_items(&chapter.content)?, page_id.as_deref()) {
        let tracker = render_tracker(book_id, page_id, &items);
        chapter.content = replace_track_block(&chapter.content, &tracker)?;
    }

    if chapter.content.contains(TRACK_ITEM_OPEN) {
        let page_id = page_id
            .as_deref()
            .context("chapter containing track-item has no source path")?;
        chapter.content = replace_inline_track_items(&chapter.content, book_id, page_id)?;
    }

    if chapter.content.contains(TRACK_CHECKLIST) {
        if inline_items.is_empty() {
            bail!("found {TRACK_CHECKLIST} but this chapter contains no track-item directives");
        }

        let page_id = page_id
            .as_deref()
            .context("chapter containing track-checklist has no source path")?;
        let checklist = render_tracker(book_id, page_id, &inline_items);
        chapter.content = chapter.content.replace(TRACK_CHECKLIST, &checklist);
    }

    if chapter.content.contains(OVERVIEW) {
        let overview = render_overview(book_id, manifest_json);
        chapter.content = chapter.content.replace(OVERVIEW, &overview);
    }

    Ok(())
}

fn collect_page_items(content: &str) -> Result<Vec<TrackItem>> {
    let mut items = Vec::new();
    let mut seen = HashSet::new();

    if let Some(legacy_items) = parse_track_items(content)? {
        for item in legacy_items {
            if !seen.insert(item.id.clone()) {
                bail!("duplicate track item id '{}'", item.id);
            }
            items.push(item);
        }
    }

    for entry in parse_inline_track_items(content)? {
        if !seen.insert(entry.item.id.clone()) {
            bail!("duplicate track item id '{}'", entry.item.id);
        }
        items.push(entry.item);
    }

    Ok(items)
}

fn parse_track_items(content: &str) -> Result<Option<Vec<TrackItem>>> {
    let Some(start) = content.find(TRACK_OPEN) else {
        return Ok(None);
    };

    let body_start = start + TRACK_OPEN.len();
    let Some(relative_end) = content[body_start..].find(TRACK_CLOSE) else {
        bail!("found {TRACK_OPEN} without matching {TRACK_CLOSE}");
    };
    let end = body_start + relative_end;
    let body = &content[body_start..end];

    let mut items = Vec::new();
    let mut seen = HashSet::new();

    for line in body.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        let label = line
            .strip_prefix("- ")
            .or_else(|| line.strip_prefix("* "))
            .unwrap_or(line)
            .trim();

        if label.is_empty() {
            continue;
        }

        let mut id = item_id(label);
        let base = id.clone();
        let mut suffix = 2usize;
        while !seen.insert(id.clone()) {
            id = format!("{base}-{suffix}");
            suffix += 1;
        }

        items.push(TrackItem {
            id,
            label: label.to_owned(),
        });
    }

    if items.is_empty() {
        bail!("tracker block contains no checklist items");
    }

    Ok(Some(items))
}

fn parse_inline_track_items(content: &str) -> Result<Vec<InlineTrackItem>> {
    let mut items = Vec::new();
    let mut seen = HashSet::new();
    let mut cursor = 0usize;

    while let Some(relative_start) = content[cursor..].find(TRACK_ITEM_OPEN) {
        let start = cursor + relative_start;
        let directive_body_start = start + TRACK_ITEM_OPEN.len();
        let Some(relative_open_end) = content[directive_body_start..].find("}}") else {
            bail!("unterminated track-item directive");
        };
        let open_end = directive_body_start + relative_open_end;
        let id = content[directive_body_start..open_end].trim();

        validate_track_item_id(id)?;
        if !seen.insert(id.to_owned()) {
            bail!("duplicate track-item id '{id}'");
        }

        let body_start = open_end + 2;
        let Some(relative_close_start) = content[body_start..].find(TRACK_ITEM_CLOSE) else {
            bail!("track-item '{id}' has no matching {TRACK_ITEM_CLOSE}");
        };
        let close_start = body_start + relative_close_start;
        let end = close_start + TRACK_ITEM_CLOSE.len();
        let label = normalise_label(&content[body_start..close_start]);

        if label.is_empty() {
            bail!("track-item '{id}' has an empty label");
        }

        items.push(InlineTrackItem {
            start,
            end,
            item: TrackItem {
                id: id.to_owned(),
                label,
            },
        });

        cursor = end;
    }

    Ok(items)
}

fn validate_track_item_id(id: &str) -> Result<()> {
    if id.is_empty() {
        bail!("track-item requires a stable id, for example {{#track-item read-pointers}}");
    }

    if !id
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.'))
    {
        bail!(
            "invalid track-item id '{id}'; use only letters, numbers, '-', '_' or '.'"
        );
    }

    Ok(())
}

fn normalise_label(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn replace_track_block(content: &str, replacement: &str) -> Result<String> {
    let Some(start) = content.find(TRACK_OPEN) else {
        return Ok(content.to_owned());
    };
    let body_start = start + TRACK_OPEN.len();
    let Some(relative_end) = content[body_start..].find(TRACK_CLOSE) else {
        bail!("found {TRACK_OPEN} without matching {TRACK_CLOSE}");
    };
    let end = body_start + relative_end + TRACK_CLOSE.len();

    let mut output = String::with_capacity(content.len() + replacement.len());
    output.push_str(&content[..start]);
    output.push_str(replacement);
    output.push_str(&content[end..]);
    Ok(output)
}

fn replace_inline_track_items(content: &str, book_id: &str, page_id: &str) -> Result<String> {
    let entries = parse_inline_track_items(content)?;
    if entries.is_empty() {
        return Ok(content.to_owned());
    }

    let mut output = content.to_owned();
    for entry in entries.into_iter().rev() {
        let replacement = render_inline_tracker(book_id, page_id, &entry.item);
        output.replace_range(entry.start..entry.end, &replacement);
    }

    Ok(output)
}

fn chapter_page_id(chapter: &Chapter) -> Result<String> {
    let path = chapter
        .source_path
        .as_deref()
        .or(chapter.path.as_deref())
        .context("tracked chapter has no source path")?;

    Ok(normalize_path(path))
}

fn chapter_section(chapter: &Chapter) -> String {
    if let Some(parent) = chapter.parent_names.first() {
        return parent.clone();
    }

    if let Some(path) = chapter.source_path.as_deref().or(chapter.path.as_deref()) {
        if let Some(parent) = path.parent() {
            if let Some(first) = parent.components().next() {
                let section = first.as_os_str().to_string_lossy().trim().to_owned();
                if !section.is_empty() && section != "." {
                    return humanize_section(&section);
                }
            }
        }
    }

    "Other".to_owned()
}

fn humanize_section(value: &str) -> String {
    value
        .split(|ch: char| ch == '-' || ch == '_')
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(first) => format!("{}{}", first.to_uppercase(), chars.as_str()),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn normalize_path(path: &Path) -> String {
    path.components()
        .map(|part| part.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

fn render_tracker(book_id: &str, page_id: &str, items: &[TrackItem]) -> String {
    let item_ids = items
        .iter()
        .map(|item| item.id.as_str())
        .collect::<Vec<_>>()
        .join(",");

    let mut html = format!(
        r#"<section class="mdbook-track mdbook-track--checklist" data-book-id="{}" data-page-id="{}" data-track-items="{}">
<div class="mdbook-track__header">
<strong>Chapter checklist</strong>
<span class="mdbook-track__count" aria-live="polite">0 / {} completed</span>
</div>
<ul class="mdbook-track__items">"#,
        escape_attr(book_id),
        escape_attr(page_id),
        escape_attr(&item_ids),
        items.len()
    );

    for item in items {
        html.push_str(&render_item_row(book_id, page_id, item));
    }

    html.push_str("\n</ul>\n</section>");
    html
}

fn render_inline_tracker(book_id: &str, page_id: &str, item: &TrackItem) -> String {
    format!(
        r#"<div class="mdbook-track mdbook-track--inline" data-book-id="{}" data-page-id="{}" data-track-items="{}">
<ul class="mdbook-track__items">{}
</ul>
</div>"#,
        escape_attr(book_id),
        escape_attr(page_id),
        escape_attr(&item.id),
        render_item_row(book_id, page_id, item)
    )
}

fn render_item_row(book_id: &str, page_id: &str, item: &TrackItem) -> String {
    format!(
        r#"
<li class="mdbook-track__item"><label><input type="checkbox" data-track-item="{}" data-track-book-id="{}" data-track-page-id="{}"><span class="mdbook-track__checkbox" aria-hidden="true"></span><span class="mdbook-track__label">{}</span></label></li>"#,
        escape_attr(&item.id),
        escape_attr(book_id),
        escape_attr(page_id),
        escape_html(&item.label)
    )
}

fn render_overview(book_id: &str, manifest_json: &str) -> String {
    format!(
        r#"<section class="mdbook-track-overview" data-book-id="{}">
<div class="mdbook-track-overview__content">Loading progress…</div>
</section>
<script type="application/json" class="mdbook-track-manifest">{}</script>"#,
        escape_attr(book_id),
        escape_script_json(manifest_json)
    )
}

fn item_id(label: &str) -> String {
    // FNV-1a gives us a deterministic ID without another dependency. The label is
    // included in the slug for easier inspection; the hash avoids slug collisions.
    let mut hash: u64 = 0xcbf29ce484222325;
    for byte in label.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }

    let mut slug = slugify(label);
    if slug.is_empty() {
        slug = "item".to_owned();
    }
    if slug.len() > 32 {
        slug.truncate(32);
        slug = slug.trim_end_matches('-').to_owned();
    }

    format!("{slug}-{hash:08x}")
}

fn slugify(value: &str) -> String {
    let mut out = String::new();
    let mut last_dash = false;

    for ch in value.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
            last_dash = false;
        } else if !last_dash && !out.is_empty() {
            out.push('-');
            last_dash = true;
        }
    }

    out.trim_matches('-').to_owned()
}

fn escape_html(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

fn escape_attr(value: &str) -> String {
    escape_html(value)
}

fn escape_script_json(value: &str) -> String {
    // Prevent a title/label containing </script> from terminating the JSON node.
    value.replace('<', "\\u003c")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_tracker_items() {
        let content = r#"# Variables

{{#track}}
- Read the chapter
- Run the example
- Complete Exercise 1
{{/track}}

More text.
"#;

        let items = parse_track_items(content).unwrap().unwrap();
        assert_eq!(items.len(), 3);
        assert_eq!(items[0].label, "Read the chapter");
        assert_eq!(items[2].label, "Complete Exercise 1");
        assert_ne!(items[0].id, items[1].id);
    }

    #[test]
    fn parses_inline_track_items() {
        let content = r#"# Pointers

{{#track-checklist}}

Text.

{{#track-item read-pointers}}
Read the pointers chapter
{{/track-item}}

{{#track-item pointer-exercise}}Complete the pointer exercise{{/track-item}}
"#;

        let items = parse_inline_track_items(content).unwrap();
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].item.id, "read-pointers");
        assert_eq!(items[0].item.label, "Read the pointers chapter");
        assert_eq!(items[1].item.id, "pointer-exercise");
        assert_eq!(items[1].item.label, "Complete the pointer exercise");
    }

    #[test]
    fn rejects_duplicate_inline_ids() {
        let content = r#"{{#track-item same}}One{{/track-item}}
{{#track-item same}}Two{{/track-item}}"#;
        assert!(parse_inline_track_items(content).is_err());
    }

    #[test]
    fn rejects_invalid_inline_id() {
        let content = r#"{{#track-item bad id}}One{{/track-item}}"#;
        assert!(parse_inline_track_items(content).is_err());
    }

    #[test]
    fn replaces_inline_items() {
        let content = "Before\n{{#track-item read}}Read it{{/track-item}}\nAfter";
        let output = replace_inline_track_items(content, "book", "chapter.md").unwrap();
        assert!(output.contains("mdbook-track--inline"));
        assert!(output.contains("data-track-item=\"read\""));
        assert!(output.contains("data-track-book-id=\"book\""));
        assert!(output.contains("data-track-page-id=\"chapter.md\""));
        assert!(!output.contains("{{#track-item"));
    }

    #[test]
    fn replaces_only_tracker_block() {
        let content = "# Title\n\n{{#track}}\n- One\n{{/track}}\n\nBody\n";
        let output = replace_track_block(content, "<tracker></tracker>").unwrap();
        assert_eq!(output, "# Title\n\n<tracker></tracker>\n\nBody\n");
    }

    #[test]
    fn item_ids_are_stable() {
        assert_eq!(item_id("Read the chapter"), item_id("Read the chapter"));
        assert_ne!(item_id("Read the chapter"), item_id("Run the example"));
    }

    #[test]
    fn humanizes_section_directory() {
        assert_eq!(humanize_section("c-programming"), "C Programming");
        assert_eq!(humanize_section("python"), "Python");
        assert_eq!(humanize_section("network_routing"), "Network Routing");
    }

    #[test]
    fn slugifies_book_title() {
        assert_eq!(
            slugify("ELEE1147: Programming for Engineers"),
            "elee1147-programming-for-engineers"
        );
    }
}
