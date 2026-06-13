use anyhow::Result;

use crate::export::PdfRenderer;
use typst::diag::{FileError, FileResult};
use typst::foundations::{Bytes, Datetime};
use typst::syntax::{FileId, Source, VirtualPath};
use typst::text::{Font, FontBook};
use typst::utils::LazyHash;
use typst::{Library, World};
use typst_kit::fonts::{FontSearcher, FontSlot, Fonts};

// ──────────────────────────────────────────────────────────────
// World implementation
// ──────────────────────────────────────────────────────────────

struct HireLensWorld {
    library: LazyHash<Library>,
    book: LazyHash<FontBook>,
    fonts: Vec<FontSlot>,
    main_id: FileId,
    main_source: Source,
}

impl HireLensWorld {
    fn new(source: String) -> Self {
        let mut searcher = FontSearcher::new();
        let Fonts { book, fonts } = searcher.search();

        let path = VirtualPath::new("main.typ");
        let main_id = FileId::new(None, path);
        let main_source = Source::new(main_id, source);

        Self {
            library: LazyHash::new(Library::builder().build()),
            book: LazyHash::new(book),
            fonts,
            main_id,
            main_source,
        }
    }
}

impl World for HireLensWorld {
    fn library(&self) -> &LazyHash<Library> {
        &self.library
    }

    fn book(&self) -> &LazyHash<FontBook> {
        &self.book
    }

    fn main(&self) -> FileId {
        self.main_id
    }

    fn source(&self, id: FileId) -> FileResult<Source> {
        if id == self.main_id {
            Ok(self.main_source.clone())
        } else {
            Err(FileError::NotFound(id.vpath().as_rooted_path().to_path_buf()))
        }
    }

    fn file(&self, id: FileId) -> FileResult<Bytes> {
        Err(FileError::NotFound(id.vpath().as_rooted_path().to_path_buf()))
    }

    fn font(&self, index: usize) -> Option<Font> {
        self.fonts.get(index)?.get()
    }

    fn today(&self, _offset: Option<i64>) -> Option<Datetime> {
        None
    }
}

// ──────────────────────────────────────────────────────────────
// Markdown → Typst conversion
// ──────────────────────────────────────────────────────────────

/// Convert a CV in Markdown format to a Typst document string.
pub fn markdown_to_typst(markdown: &str) -> String {
    let mut out = String::from(
        "#set page(paper: \"a4\", margin: (x: 2cm, y: 1.5cm))\n\
         #set text(size: 11pt)\n\
         #set heading(numbering: none)\n\
         #show heading.where(level: 1): it => [\n  \
           #text(size: 18pt, weight: \"bold\")[#it.body]\n  \
           #v(4pt)\n\
         ]\n\n",
    );

    for line in markdown.lines() {
        if let Some(rest) = line.strip_prefix("### ") {
            out.push_str("=== ");
            out.push_str(&escape_typst(rest));
            out.push('\n');
        } else if let Some(rest) = line.strip_prefix("## ") {
            out.push_str("== ");
            out.push_str(&escape_typst(rest));
            out.push('\n');
        } else if let Some(rest) = line.strip_prefix("# ") {
            out.push_str("= ");
            out.push_str(&escape_typst(rest));
            out.push('\n');
        } else if let Some(rest) = line.strip_prefix("- ") {
            out.push_str("- ");
            out.push_str(&inline_markup(rest));
            out.push('\n');
        } else if line.trim().is_empty() {
            out.push('\n');
        } else {
            out.push_str(&inline_markup(line));
            out.push('\n');
        }
    }

    out
}

/// Escape a single character that carries syntactic meaning in Typst markup.
///
/// Single source of truth shared by `escape_typst` and `inline_markup` so plain
/// text, bold, and italic content are all escaped identically. Char-by-char
/// (rather than chained `replace`) avoids double-escaping the backslash itself.
fn push_escaped(ch: char, out: &mut String) {
    match ch {
        '\\' => out.push_str("\\\\"),
        '#' => out.push_str("\\#"),
        '@' => out.push_str("\\@"),
        '<' => out.push_str("\\<"),
        '>' => out.push_str("\\>"),
        '"' => out.push_str("\\\""),
        '_' => out.push_str("\\_"),
        '[' => out.push_str("\\["),
        ']' => out.push_str("\\]"),
        c => out.push(c),
    }
}

fn escape_typst(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        push_escaped(ch, &mut out);
    }
    out
}

fn inline_markup(s: &str) -> String {
    let mut result = String::with_capacity(s.len() + 4);
    let bytes = s.as_bytes();
    let mut i = 0;

    while i < bytes.len() {
        if i + 1 < bytes.len() && bytes[i] == b'*' && bytes[i + 1] == b'*' {
            // Bold: **text** → *text*
            i += 2;
            let start = i;
            while i + 1 < bytes.len() && !(bytes[i] == b'*' && bytes[i + 1] == b'*') {
                i += 1;
            }
            result.push('*');
            result.push_str(&escape_typst(&s[start..i]));
            result.push('*');
            i += 2; // skip closing **
        } else if bytes[i] == b'*' {
            // Italic: *text* → _text_
            i += 1;
            let start = i;
            while i < bytes.len() && bytes[i] != b'*' {
                i += 1;
            }
            result.push('_');
            result.push_str(&escape_typst(&s[start..i]));
            result.push('_');
            i += 1; // skip closing *
        } else {
            // char-aware: never split a multi-byte codepoint (e.g. "é").
            let ch = s[i..].chars().next().unwrap_or('\0');
            push_escaped(ch, &mut result);
            i += ch.len_utf8();
        }
    }

    result
}

// ──────────────────────────────────────────────────────────────
// TypstRenderer — implements PdfRenderer
// ──────────────────────────────────────────────────────────────

pub struct TypstRenderer;

impl PdfRenderer for TypstRenderer {
    fn render(&self, markdown: &str) -> anyhow::Result<Vec<u8>> {
        export_pdf(markdown)
    }
}

// ──────────────────────────────────────────────────────────────
// PDF export
// ──────────────────────────────────────────────────────────────

/// Compile a Markdown CV to a PDF byte vector using Typst.
pub fn export_pdf(markdown: &str) -> Result<Vec<u8>> {
    let source = markdown_to_typst(markdown);
    let world = HireLensWorld::new(source);

    let result = typst::compile::<typst::layout::PagedDocument>(&world);

    let document = result.output.map_err(|errors| {
        let msgs: Vec<String> = errors.iter().map(|e| e.message.to_string()).collect();
        anyhow::anyhow!("Typst compilation: {}", msgs.join("; "))
    })?;

    let pdf_bytes = typst_pdf::pdf(
        &document,
        &typst_pdf::PdfOptions {
            ident: typst::foundations::Smart::Auto,
            timestamp: None,
            page_ranges: None,
            standards: typst_pdf::PdfStandards::default(),
        },
    )
    .map_err(|errors| {
        let msgs: Vec<String> = errors.iter().map(|e| e.message.to_string()).collect();
        anyhow::anyhow!("PDF export: {}", msgs.join("; "))
    })?;

    Ok(pdf_bytes)
}

// ──────────────────────────────────────────────────────────────
// Tests
// ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn heading_levels_converted() {
        let md = "# Alice\n## Skills\n### Rust Engineer";
        let typ = markdown_to_typst(md);
        assert!(typ.contains("= Alice"));
        assert!(typ.contains("== Skills"));
        assert!(typ.contains("=== Rust Engineer"));
    }

    #[test]
    fn bullet_points_preserved() {
        let md = "## Skills\n- Rust\n- Docker";
        let typ = markdown_to_typst(md);
        assert!(typ.contains("- Rust"));
        assert!(typ.contains("- Docker"));
    }

    #[test]
    fn bold_converted_to_typst() {
        let result = inline_markup("See **this** example");
        assert!(result.contains("*this*"));
    }

    #[test]
    fn hash_escaped_in_plain_text() {
        let result = escape_typst("C# Developer");
        assert!(result.contains("\\#"));
    }

    #[test]
    fn bold_with_accents_does_not_panic() {
        // "é" / "à" are 2 bytes each — byte-index slicing must not split them.
        let result = inline_markup("**Développeur** confirmé à Paris");
        assert!(result.contains("*Développeur*"));
        assert!(result.contains("confirmé à Paris"));
    }

    #[test]
    fn special_chars_escaped() {
        let result = escape_typst("first_name [remote] path\\to <tag>");
        assert!(result.contains("first\\_name"));
        assert!(result.contains("\\[remote\\]"));
        assert!(result.contains("path\\\\to"));
        assert!(result.contains("\\<tag\\>"));
    }
}
