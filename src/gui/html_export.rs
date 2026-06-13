use pulldown_cmark::{html, Options, Parser};

const CSS: &str = r#"
  *, *::before, *::after { box-sizing: border-box; margin: 0; padding: 0; }
  body {
    font-family: system-ui, -apple-system, 'Segoe UI', Arial, sans-serif;
    font-size: 10.5pt;
    line-height: 1.55;
    color: #1a1a1a;
    max-width: 800px;
    margin: 0 auto;
    padding: 2.4cm 2.2cm;
  }
  h1 { font-size: 22pt; font-weight: 700; color: #111; margin-bottom: 2px; }
  h2 {
    font-size: 11pt;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: .08em;
    color: #2563eb;
    border-bottom: 1.5px solid #2563eb;
    margin-top: 18px;
    margin-bottom: 8px;
    padding-bottom: 3px;
  }
  h3 { font-size: 10.5pt; font-weight: 600; margin-top: 10px; margin-bottom: 2px; }
  p { margin-bottom: 6px; }
  ul { padding-left: 18px; margin-bottom: 6px; }
  li { margin-bottom: 3px; }
  em { color: #444; }
  strong { font-weight: 600; }
  .chip {
    display: inline-block;
    background: #eff6ff;
    color: #1d4ed8;
    border-radius: 4px;
    padding: 1px 8px;
    margin: 2px 3px 2px 0;
    font-size: 9.5pt;
  }
  @media print {
    body { padding: 0; max-width: 100%; }
    h2 { color: #000; border-color: #000; }
    .chip { background: #f0f0f0; color: #000; }
  }
"#;

/// Converts a Markdown string to a self-contained, print-ready HTML document.
pub fn to_html(markdown: &str) -> String {
    let mut opts = Options::empty();
    opts.insert(Options::ENABLE_STRIKETHROUGH);
    opts.insert(Options::ENABLE_TABLES);

    let parser = Parser::new_ext(markdown, opts);
    let mut body = String::with_capacity(markdown.len() * 2);
    html::push_html(&mut body, parser);

    format!(
        r#"<!DOCTYPE html>
<html lang="fr">
<head>
  <meta charset="UTF-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1.0" />
  <title>CV — HireLens</title>
  <style>{CSS}</style>
</head>
<body>
{body}
</body>
</html>"#
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_markdown_produces_valid_html() {
        let html = to_html("");
        assert!(html.contains("<!DOCTYPE html>"));
        assert!(html.contains("<body>"));
        assert!(html.contains("</body>"));
    }

    #[test]
    fn cv_skills_appear_in_html_output() {
        let md = "## Compétences\n\n- Rust\n- Docker\n- Kubernetes\n";
        let html = to_html(md);
        assert!(html.contains("Rust"));
        assert!(html.contains("Docker"));
        assert!(html.contains("Kubernetes"));
        assert!(html.contains("<li>"));
    }

    #[test]
    fn html_is_self_contained_no_external_links() {
        let html = to_html("# Alice\n\n## Skills\n\n- Rust\n");
        // No src= or href= pointing outside (no external resources)
        let has_external = html
            .split("src=")
            .skip(1)
            .any(|s| s.trim_start_matches('"').starts_with("http"));
        assert!(!has_external, "HTML should not load external resources");
    }
}
