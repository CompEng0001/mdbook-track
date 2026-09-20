<div align="center">
  <h1 align="center"><b>mdbook-track</b></h1>
</div>

<p align="center">
  <a href="https://crates.io/crates/mdbook-track">
    <img src="https://img.shields.io/crates/v/mdbook-track?style=for-the-badge" alt="Crates.io version" />
  </a>
  <a href="https://crates.io/crates/mdbook-track">
    <img src="https://img.shields.io/crates/d/mdbook-track?style=for-the-badge" alt="Downloads" />
  </a>
  <a href="https://docs.rs/mdbook-track">
    <img src="https://img.shields.io/docsrs/mdbook-track?style=for-the-badge" alt="Docs.rs" />
  </a>
  <a href="https://github.com/CompEng0001/mdbook-track/actions">
    <img src="https://img.shields.io/github/actions/workflow/status/CompEng0001/mdbook-track/release.yml?&style=for-the-badge&label=CI" alt="CI status" />
  </a>
  <img src="https://img.shields.io/badge/Built%20with-Rust-orange?logo=rust&style=for-the-badge" alt="Built with Rust" />
</p>

An <a href="https://github.com/rust-lang/mdBook">mdBook</a> preprocessor that injects Git metadata (commit hash, full hash, tag, date/time, branch) into each chapter — as a header, a footer, or both — with flexible templates, alignment, and CSS-style margins.

`mdbook-track` adds browser-local progress tracking to teaching and learning mdBooks.
Progress is stored in `localStorage`; there is no account, backend, export/import, or analytics.

## Requirements

This version targets mdBook 0.5.4 and Rust 1.88+.

## Install

```sh
cargo install --path .
```

Install the JavaScript and CSS into a book:

```sh
mdbook-track init /path/to/book
```

Then add this to `book.toml`:

```toml
[preprocessor.track]
book-id = "elee1147-2026"
renderers = ["html"]

[output.html]
additional-js = ["mdbook-track.js"]
additional-css = ["mdbook-track.css"]
```

If `[output.html]` already exists, add the two `additional-*` entries to the existing table rather than creating another table.

## Tracking model

There are three tracker directives:

- `{{#track}} ... {{/track}}` creates a compact, self-contained chapter checklist.
- `{{#track-item id}} ... {{/track-item}}` creates an individually placed trackable item.
- `{{#track-checklist}}` renders a chapter checklist containing all `track-item` entries declared in that chapter.

The workbook-level `{{#track-overview}}` reads all tracked chapter state.

## Self-contained chapter checklist

The original compact form remains available:

```md
# Variables

{{#track}}
- Read the chapter
- Compile and run the example
- Complete Exercise 1
{{/track}}
```

Item identifiers in `{{#track}}` are derived deterministically from their labels.

## Individually placed tracked items

Use an explicit stable ID when an item should appear at a meaningful position within the chapter:

```md
# Pointers

{{#track-checklist}}

## Pointer fundamentals

Read this section and work through the examples.

{{#track-item read-pointers}}
Read the pointers chapter
{{/track-item}}

## Pointer arithmetic

Complete the practical activity.

{{#track-item pointer-exercise}}
Complete the pointer arithmetic exercise
{{/track-item}}
```

This renders the two `track-item` rows inline and also places both items in the chapter checklist at `{{#track-checklist}}`.

The explicit ID is the persistent identity of the item. The label can therefore be edited later without losing the student's stored completion state:

```md
{{#track-item pointer-exercise}}
Complete Exercise 5.2: Pointer arithmetic
{{/track-item}}
```

IDs may contain letters, numbers, `-`, `_`, and `.`.

Checking an inline `track-item` immediately updates the matching entry in `track-checklist`, and checking the checklist entry updates the inline item. Both are views of the same `localStorage` value.

### Indentation inside Markdown lists

`mdbook-track` preserves the leading indentation of a tracker directive on every generated HTML line. This allows a tracker to remain structurally inside an ordered or unordered Markdown list item instead of being pulled back to the page margin. Place the directive at the indentation level where you want the tracker to belong:

```md
1. Install Git.

   {{#track-item install-git}}
   Installed Git
   {{/track-item}}

2. Create a GitHub account.

   {{#track-item github-account}}
   GitHub account creation
   {{/track-item}}
```

The tracker row will then align with the content of the corresponding list item. The same indentation-preserving behaviour is applied to `{{#track}}`, `{{#track-checklist}}`, and `{{#track-overview}}`.

## Workbook overview

Place this in `Introduction.md` (or any other page):

```md
# Introduction

{{#track-overview}}
```

The overview reads the same browser-local state and shows progress for every chapter containing tracked items.

Chapters are grouped using their mdBook parent chapter when present; otherwise the first directory in the source path is used. For example:

```text
c-programming/variables.md  -> C Programming
python/modules.md           -> Python
```

## Storage

Progress is stored in browser `localStorage` for the book origin. For local testing, use `mdbook serve`; direct `file://` browsing does not provide reliable cross-page storage semantics.


State is stored under:

```text
mdbook-track::<book-id>
```

A tracked page stores item completion by stable item ID.

## Example layout

> Keep source directory names used by `SUMMARY.md` free of spaces. For example, use `c-programming/` rather than `C Programming/`. The tracker converts hyphenated directory names to display labels.

```text
src/
├── SUMMARY.md
├── Introduction.md
├── c-programming/
│   ├── variables.md
│   ├── pointers.md
│   └── structs.md
└── python/
    ├── modules.md
    └── oop.md
```

## Run

```sh
mdbook serve
```


## Styling note (v0.2.3)

Completed tracker rows use a solid `#84ad24` border and a 25% alpha fill (`rgb(132 173 36 / 25%)`).
