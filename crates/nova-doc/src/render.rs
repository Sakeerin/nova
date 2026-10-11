//! Pages (spec §6, §7; plan decisions 12 and 13).

use crate::escape;
use crate::markdown::Docs;
use crate::model::{
    relative, Impl, Item, ItemKind, Member, MemberKind, Module, Package, Primitive, Site,
};

/// The version every page's footer names.
const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Every page of `site`: its path from the site's root, and its HTML.
pub(crate) fn pages(site: &Site) -> Vec<(String, String)> {
    let mut out = vec![("index.html".to_string(), front(site))];
    for p in &site.packages {
        if p.std {
            out.push(("std/index.html".to_string(), std_front(p)));
            for pr in &p.primitives {
                out.push((pr.page.clone(), primitive(p, pr)));
            }
        }
        for m in &p.modules {
            out.push((m.page.clone(), module(p, m)));
        }
    }
    out
}

/// One `../` for each `/` in `page`: from it back to the site's root.
pub(crate) fn up(page: &str) -> String {
    "../".repeat(page.matches('/').count())
}

/// A whole page: the header, the sidebar, the main part and the footer.
fn page(path: &str, title: &str, sidebar: &str, main: &str) -> String {
    let root = up(path);
    format!(
        "<!DOCTYPE html>\n\
         <html lang=\"en\">\n\
         <head>\n\
         <meta charset=\"utf-8\">\n\
         <meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n\
         <title>{title}</title>\n\
         <link rel=\"stylesheet\" href=\"{root}nova.css\">\n\
         <script>try{{var t=localStorage.getItem(\"nova-doc-theme\");\
         if(t)document.documentElement.setAttribute(\"data-theme\",t)}}catch(e){{}}</script>\n\
         <script src=\"{root}search-index.js\" defer></script>\n\
         <script src=\"{root}nova.js\" defer></script>\n\
         </head>\n\
         <body data-root=\"{root}\">\n\
         <header class=\"top\"><a class=\"site\" href=\"{root}index.html\">nova doc</a>\
         <input id=\"search\" type=\"search\" placeholder=\"Search (press /)\" \
         aria-label=\"Search\" autocomplete=\"off\">\
         <button id=\"theme\" type=\"button\" \
         aria-label=\"Switch between the light and the dark theme\">Theme</button></header>\n\
         <div class=\"layout\">\n\
         <nav class=\"sidebar\">\n{sidebar}</nav>\n\
         <main id=\"main\">\n{main}</main>\n\
         <section id=\"results\" class=\"results\" hidden></section>\n\
         </div>\n\
         <footer>Written by nova {version}.</footer>\n\
         </body>\n\
         </html>\n",
        title = escape(title),
        version = VERSION,
    )
}

/// The front page: every package (spec §7.1).
fn front(site: &Site) -> String {
    let mut main = String::from("<h1 class=\"title\">Documentation</h1>\n<ul class=\"list\">\n");
    let mut side = String::from("<h2>Packages</h2>\n<ul>\n");
    for p in &site.packages {
        let description = p
            .description
            .as_deref()
            .map_or(String::new(), |d| format!(" {}", escape(d)));
        main.push_str(&format!(
            "<li><a href=\"{n}/index.html\">{n}</a> <span class=\"version\">{v}</span>{description}</li>\n",
            n = escape(&p.name),
            v = escape(&p.version),
        ));
        side.push_str(&format!(
            "<li><a href=\"{n}/index.html\">{n}</a></li>\n",
            n = escape(&p.name)
        ));
    }
    main.push_str("</ul>\n");
    side.push_str("</ul>\n");
    page("index.html", "Documentation", &side, &main)
}

/// The sidebar of a page at `path` in package `p`, with the page's own
/// anchors and their labels.
fn sidebar(path: &str, p: &Package, on_page: &[(String, String)]) -> String {
    let mut s = format!(
        "<p class=\"package\"><a href=\"{}\">{}</a> <span class=\"version\">{}</span></p>\n",
        relative(path, &format!("{}/index.html", p.name)),
        escape(&p.name),
        escape(&p.version)
    );
    let others: Vec<&Module> = p.modules.iter().filter(|m| !m.root).collect();
    if !others.is_empty() {
        s.push_str("<h2>Modules</h2>\n<ul>\n");
        for m in others {
            s.push_str(&format!(
                "<li><a href=\"{}\">{}</a></li>\n",
                relative(path, &m.page),
                escape(&m.name)
            ));
        }
        s.push_str("</ul>\n");
    }
    if !p.primitives.is_empty() {
        s.push_str("<h2>Primitive types</h2>\n<ul>\n");
        for pr in &p.primitives {
            s.push_str(&format!(
                "<li><a href=\"{}\">{}</a></li>\n",
                relative(path, &pr.page),
                pr.name
            ));
        }
        s.push_str("</ul>\n");
    }
    if !on_page.is_empty() {
        s.push_str("<h2>On this page</h2>\n<ul>\n");
        for (anchor, label) in on_page {
            s.push_str(&format!(
                "<li><a href=\"#{anchor}\">{}</a></li>\n",
                escape(label)
            ));
        }
        s.push_str("</ul>\n");
    }
    s
}

/// A module's page; a root module's is its package's (spec §7.2, §7.3).
fn module(p: &Package, m: &Module) -> String {
    let mut main = String::new();
    if m.root {
        main.push_str(&format!(
            "<h1 class=\"title\">Package <span class=\"name\">{}</span> \
             <span class=\"version\">{}</span></h1>\n",
            escape(&p.name),
            escape(&p.version)
        ));
        if let Some(d) = &p.description {
            main.push_str(&format!("<p class=\"description\">{}</p>\n", escape(d)));
        }
    } else {
        main.push_str(&format!(
            "<h1 class=\"title\">Module <span class=\"name\">{}</span></h1>\n",
            escape(&m.qual)
        ));
    }
    docs(&mut main, &m.docs);
    let others: Vec<&Module> = p.modules.iter().filter(|o| !o.root).collect();
    if m.root && !others.is_empty() {
        main.push_str("<h2 id=\"modules\">Modules</h2>\n<ul class=\"list\">\n");
        for o in others {
            main.push_str(&format!(
                "<li><a href=\"{}\">{}</a> {}</li>\n",
                relative(&m.page, &o.page),
                escape(&o.name),
                escape(&o.docs.summary)
            ));
        }
        main.push_str("</ul>\n");
    }
    let on_page = sections(&mut main, &m.items, &m.other_impls);
    let title = if m.root { &p.name } else { &m.qual };
    page(&m.page, title, &sidebar(&m.page, p, &on_page), &main)
}

/// A module's items, by section, and its impls on other types (spec
/// §7.3). Returns the page's anchors for the sidebar.
fn sections(out: &mut String, items: &[Item], other: &[Impl]) -> Vec<(String, String)> {
    let mut on_page = Vec::new();
    for kind in [
        ItemKind::Record,
        ItemKind::Sum,
        ItemKind::Trait,
        ItemKind::Function,
        ItemKind::Constant,
    ] {
        let of: Vec<&Item> = items.iter().filter(|i| i.kind == kind).collect();
        if of.is_empty() {
            continue;
        }
        let (title, id) = kind.section();
        out.push_str(&format!("<h2 id=\"{id}\">{title}</h2>\n"));
        for item in of {
            item_html(out, item);
            on_page.push((item.anchor.clone(), item.name.clone()));
        }
    }
    if !other.is_empty() {
        out.push_str("<h2 id=\"implementations\">Implementations on other types</h2>\n");
        for i in other {
            impl_html(out, i);
        }
    }
    on_page
}

fn item_html(out: &mut String, item: &Item) {
    out.push_str(&format!(
        "<section class=\"item\">\n<h3 id=\"{a}\"><a class=\"anchor\" href=\"#{a}\">{w} {n}</a></h3>\n\
         <pre class=\"signature\"><code>{s}</code></pre>\n",
        a = item.anchor,
        w = item.kind.word(),
        n = escape(&item.name),
        s = item.signature
    ));
    docs(out, &item.docs);
    for kind in [
        MemberKind::Field,
        MemberKind::Variant,
        MemberKind::AssocType,
        MemberKind::Method,
    ] {
        let of: Vec<&Member> = item.members.iter().filter(|m| m.kind == kind).collect();
        if of.is_empty() {
            continue;
        }
        out.push_str(&format!("<h4>{}</h4>\n", kind.heading()));
        for m in of {
            member_html(out, m);
        }
    }
    if item.hidden_fields {
        out.push_str("<p class=\"note\">It has private fields, which are not documented.</p>\n");
    }
    if !item.impls.is_empty() {
        out.push_str("<h4>Implementations</h4>\n");
        for i in &item.impls {
            impl_html(out, i);
        }
    }
    out.push_str("</section>\n");
}

fn member_html(out: &mut String, m: &Member) {
    out.push_str(&format!(
        "<div class=\"member\" id=\"{}\">\n<pre class=\"signature\"><code>{}</code></pre>\n",
        m.anchor, m.signature
    ));
    docs(out, &m.docs);
    out.push_str("</div>\n");
}

fn impl_html(out: &mut String, i: &Impl) {
    out.push_str(&format!(
        "<div class=\"impl\">\n<pre class=\"signature\"><code>{}</code></pre>\n",
        i.header
    ));
    docs(out, &i.docs);
    for m in &i.methods {
        member_html(out, m);
    }
    out.push_str("</div>\n");
}

fn docs(out: &mut String, d: &Docs) {
    if !d.html.is_empty() {
        out.push_str("<div class=\"docs\">\n");
        out.push_str(&d.html);
        out.push_str("</div>\n");
    }
}

/// std's front page: its modules, primitive types and builtins (spec
/// §7.4).
fn std_front(p: &Package) -> String {
    let path = "std/index.html";
    let mut main = format!(
        "<h1 class=\"title\">Package <span class=\"name\">std</span> \
         <span class=\"version\">{}</span></h1>\n",
        escape(&p.version)
    );
    if let Some(d) = &p.description {
        main.push_str(&format!("<p class=\"description\">{}</p>\n", escape(d)));
    }
    main.push_str("<h2 id=\"modules\">Modules</h2>\n<ul class=\"list\">\n");
    for m in &p.modules {
        main.push_str(&format!(
            "<li><a href=\"{}\">{}</a> {}</li>\n",
            relative(path, &m.page),
            escape(&m.name),
            escape(&m.docs.summary)
        ));
    }
    main.push_str("</ul>\n<h2 id=\"primitives\">Primitive types</h2>\n<ul class=\"list\">\n");
    for pr in &p.primitives {
        main.push_str(&format!(
            "<li><a href=\"{}\">{}</a> {}</li>\n",
            relative(path, &pr.page),
            pr.name,
            escape(&pr.about.summary)
        ));
    }
    main.push_str("</ul>\n");
    let mut on_page = Vec::new();
    if !p.builtins.is_empty() {
        main.push_str("<h2 id=\"builtins\">Built-in functions</h2>\n");
        for b in &p.builtins {
            main.push_str(&format!(
                "<section class=\"item\">\n<h3 id=\"fn.{n}\"><a class=\"anchor\" href=\"#fn.{n}\">fn {n}</a></h3>\n\
                 <pre class=\"signature\"><code>{s}</code></pre>\n",
                n = b.name,
                s = b.signature
            ));
            docs(&mut main, &b.docs);
            main.push_str("</section>\n");
            on_page.push((format!("fn.{}", b.name), b.name.to_string()));
        }
    }
    page(path, "std", &sidebar(path, p, &on_page), &main)
}

/// A primitive type's page: std's impls on it (spec §7.4).
fn primitive(p: &Package, pr: &Primitive) -> String {
    let mut main = format!(
        "<h1 class=\"title\">Primitive type <span class=\"name\">{}</span></h1>\n",
        pr.name
    );
    docs(&mut main, &pr.about);
    let mut on_page = Vec::new();
    if !pr.impls.is_empty() {
        main.push_str("<h2 id=\"implementations\">Implementations</h2>\n");
        for i in &pr.impls {
            impl_html(&mut main, i);
            for m in &i.methods {
                on_page.push((m.anchor.clone(), m.name.clone()));
            }
        }
    }
    page(&pr.page, pr.name, &sidebar(&pr.page, p, &on_page), &main)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::sample_site;

    /// The values of each `name="…"` attribute in `html`.
    fn attributes<'h>(html: &'h str, name: &str) -> Vec<&'h str> {
        let pattern = format!(" {name}=\"");
        html.match_indices(&pattern)
            .map(|(at, _)| {
                let value = &html[at + pattern.len()..];
                &value[..value.find('"').unwrap()]
            })
            .collect()
    }

    #[test]
    fn every_page_of_the_sample_is_written() {
        let pages: Vec<String> = pages(&sample_site()).into_iter().map(|(p, _)| p).collect();
        assert_eq!(
            pages,
            [
                "index.html",
                "app/index.html",
                "app/util/index.html",
                "std/index.html",
                "std/primitive.Int.html",
                "std/core/index.html",
            ]
        );
    }

    #[test]
    fn every_url_is_relative_with_slashes() {
        // Review Focus 3.
        for (page, html) in pages(&sample_site()) {
            for url in attributes(&html, "href")
                .into_iter()
                .chain(attributes(&html, "src"))
            {
                assert!(!url.starts_with('/'), "{page}: {url}");
                assert!(!url.contains('\\'), "{page}: {url}");
                assert!(!url.contains("://"), "{page}: {url}");
            }
        }
    }

    #[test]
    fn a_package_page_shows_its_item_its_members_and_its_modules() {
        let (_, html) = pages(&sample_site())
            .into_iter()
            .find(|(p, _)| p == "app/index.html")
            .unwrap();
        assert!(html.contains("<title>app</title>"), "{html}");
        assert!(html.contains("<body data-root=\"../\">"), "{html}");
        assert!(
            html.contains("<link rel=\"stylesheet\" href=\"../nova.css\">"),
            "{html}"
        );
        assert!(html.contains("<h2 id=\"records\">Records</h2>"), "{html}");
        assert!(
            html.contains("<h3 id=\"record.Point\"><a class=\"anchor\" href=\"#record.Point\">record Point</a></h3>"),
            "{html}"
        );
        assert!(
            html.contains("<div class=\"member\" id=\"record.Point.field.x\">"),
            "{html}"
        );
        assert!(html.contains("<p class=\"note\">"), "{html}");
        assert!(
            html.contains("<a href=\"../app/util/index.html\">util</a>"),
            "{html}"
        );
    }

    #[test]
    fn std_front_page_lists_modules_primitives_and_builtins() {
        let (_, html) = pages(&sample_site())
            .into_iter()
            .find(|(p, _)| p == "std/index.html")
            .unwrap();
        assert!(
            html.contains("<a href=\"../std/core/index.html\">core</a>"),
            "{html}"
        );
        assert!(
            html.contains("<a href=\"../std/primitive.Int.html\">Int</a>"),
            "{html}"
        );
        assert!(html.contains("<h3 id=\"fn.println\">"), "{html}");
    }
}
