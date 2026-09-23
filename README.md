# mdbook-track

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


## Styling

Completed tracker rows use a solid `#84ad24` border and a 25% alpha fill (`rgb(132 173 36 / 25%)`).

The workbook overview uses the same accent colour for overall and section progress bars, with compact chapter status indicators for not-started, in-progress, and completed chapters.
