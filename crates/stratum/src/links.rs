//! `stratum links check` — que los links entre documentos lleguen a algún lado.
//!
//! Un link roto en markdown no falla: simplemente no lleva a ningún lado. Eso ya
//! pasó a escala —31 links roto conviviendo con el proyecto sin que nadie se
//! enterara, 27 por contar `../` a mano— y es lo que este comando detecta.
//!
//! **No duplica al proveedor `doc` de lattice.** Ese describe el grafo y reporta un
//! link muerto como información; éste pregunta si el árbol está sano y **falla**. Y
//! mira anchors, que al grafo le son invisibles: su nodo destino es el archivo
//! completo.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

/// Un link que no llega.
#[derive(Debug, Clone, PartialEq)]
pub struct Broken {
    /// El archivo que lo escribe, relativo a la raíz del recorrido.
    pub from: String,
    pub href: String,
    pub why: Why,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Why {
    /// El archivo destino no existe.
    Missing,
    /// El archivo está; el heading que el `#` nombra, no.
    Anchor,
}

impl std::fmt::Display for Why {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Missing => "no existe",
            Self::Anchor => "anchor",
        })
    }
}

pub struct Report {
    pub checked: usize,
    pub broken: Vec<Broken>,
}

/// Recorre los markdown bajo `root` y devuelve los links que no llegan.
pub fn check(root: &Path, anchors: bool) -> Report {
    let mut checked = 0;
    let mut broken = Vec::new();

    for file in markdown_files(root) {
        let Ok(text) = std::fs::read_to_string(&file) else { continue };
        let dir = file.parent().unwrap_or(root).to_path_buf();
        let from = rel(root, &file);

        for (href, _) in links_in(&text) {
            checked += 1;
            let (path, anchor) = split_anchor(&href);
            if path.is_empty() {
                continue; // un `#heading` a secas: es de este mismo archivo
            }
            let target = normalize(&dir.join(decode(path)));
            if !target.exists() {
                broken.push(Broken { from: from.clone(), href, why: Why::Missing });
                continue;
            }
            if !anchors || anchor.is_empty() || target.extension().is_none_or(|e| e != "md") {
                continue;
            }
            let Ok(dest) = std::fs::read_to_string(&target) else { continue };
            if !headings(&dest).contains(&slug(&decode(anchor))) {
                broken.push(Broken { from: from.clone(), href, why: Why::Anchor });
            }
        }
    }

    Report { checked, broken }
}

/// Los links de un markdown, con los de adentro de un bloque de código afuera.
///
/// **Las mismas exclusiones que el proveedor `doc` de lattice**, y por el mismo
/// motivo: un ejemplo no es una referencia, y un embed no es una referencia a otro
/// documento. Que difirieran haría que un link fuera roto para una herramienta y
/// sano para la otra, sin dónde resolver la contradicción.
///
/// No es hipotético: los ejemplos con los que `provider.md` **explica esta regla**
/// son links que no resuelven, y sin la exclusión saldrían como rotos para siempre.
pub fn links_in(text: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let mut fence: Option<String> = None;

    for line in text.lines() {
        let trimmed = line.trim_start();
        // Una cerca se cierra con una de su mismo largo o mayor, así que las cercas
        // de cuatro backticks pueden contener bloques de tres — que es como
        // `provider.md` muestra sus ejemplos.
        if let Some(marker) = fence_marker(trimmed) {
            match &fence {
                Some(open) if marker.len() >= open.len() && marker.starts_with(&open[..1]) => {
                    fence = None;
                }
                Some(_) => {}
                None => fence = Some(marker),
            }
            continue;
        }
        if fence.is_some() {
            continue;
        }
        collect_links(line, &mut out);
    }
    out
}

fn fence_marker(line: &str) -> Option<String> {
    for c in ['`', '~'] {
        let n = line.chars().take_while(|&x| x == c).count();
        if n >= 3 {
            return Some(std::iter::repeat(c).take(n).collect());
        }
    }
    None
}

/// `[texto](destino)`, salteando las imágenes y el código inline.
fn collect_links(line: &str, out: &mut Vec<(String, String)>) {
    let bytes: Vec<char> = line.chars().collect();
    let mut i = 0;
    let mut in_code = false;

    while i < bytes.len() {
        if bytes[i] == '`' {
            in_code = !in_code;
            i += 1;
            continue;
        }
        if in_code || bytes[i] != '[' {
            i += 1;
            continue;
        }
        // `![alt](x.png)` es un embed, no una referencia.
        if i > 0 && bytes[i - 1] == '!' {
            i += 1;
            continue;
        }
        let Some(close) = find_matching(&bytes, i) else { i += 1; continue };
        if close + 1 >= bytes.len() || bytes[close + 1] != '(' {
            i = close + 1;
            continue;
        }
        let Some(paren) = bytes[close + 2..].iter().position(|&c| c == ')') else {
            i = close + 1;
            continue;
        };
        let text: String = bytes[i + 1..close].iter().collect();
        let dest: String = bytes[close + 2..close + 2 + paren].iter().collect();
        // Un título después del destino: `(path "título")`.
        let href = dest.split_whitespace().next().unwrap_or("").to_string();
        if !href.is_empty() && !is_external(&href) {
            out.push((href, text));
        }
        i = close + 2 + paren + 1;
    }
}

/// El `]` que cierra este `[`, contando los anidados — un `[`texto`]` con corchetes
/// adentro es común en estas specs.
fn find_matching(chars: &[char], open: usize) -> Option<usize> {
    let mut depth = 0;
    for (k, &c) in chars.iter().enumerate().skip(open) {
        match c {
            '[' => depth += 1,
            ']' => {
                depth -= 1;
                if depth == 0 {
                    return Some(k);
                }
            }
            _ => {}
        }
    }
    None
}

fn is_external(href: &str) -> bool {
    href.starts_with("http://")
        || href.starts_with("https://")
        || href.starts_with("mailto:")
        || href.starts_with('#')
}

fn split_anchor(href: &str) -> (&str, &str) {
    match href.split_once('#') {
        Some((p, a)) => (p, a),
        None => (href, ""),
    }
}

/// Los slugs de los headings, como los produce un renderer de markdown.
///
/// **Los backticks se sacan sin dejar guion**, y de ahí salieron 3 de los 4 anchors
/// roto del árbol: escribían `#bilinkhead--de-dónde` con dos guiones donde
/// `` `.bilink/head` — de dónde `` produce uno.
pub fn headings(text: &str) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    let mut fence: Option<String> = None;
    for line in text.lines() {
        let t = line.trim_start();
        if let Some(marker) = fence_marker(t) {
            fence = match &fence {
                Some(open) if marker.len() >= open.len() => None,
                Some(o) => Some(o.clone()),
                None => Some(marker),
            };
            continue;
        }
        if fence.is_some() {
            continue;
        }
        let hashes = t.chars().take_while(|&c| c == '#').count();
        if (1..=6).contains(&hashes) && t.chars().nth(hashes) == Some(' ') {
            out.insert(slug(t[hashes + 1..].trim()));
        }
    }
    out
}

pub fn slug(title: &str) -> String {
    let sin_backticks: String = title.chars().filter(|&c| c != '`').collect();
    let limpio: String = sin_backticks
        .to_lowercase()
        .chars()
        .filter(|c| c.is_alphanumeric() || c.is_whitespace() || *c == '-' || *c == '_')
        .collect();
    limpio.split_whitespace().collect::<Vec<_>>().join("-")
}

/// `%C3%A9` → `é`. Los anchors con acentos viajan escapados.
fn decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' && i + 2 < b.len() {
            if let Ok(v) = u8::from_str_radix(&s[i + 1..i + 3], 16) {
                out.push(v);
                i += 3;
                continue;
            }
        }
        out.push(b[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// Resuelve `..` y `.` sin tocar el disco: el destino puede no existir, que es
/// justamente lo que se está por averiguar.
fn normalize(p: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for c in p.components() {
        match c {
            std::path::Component::ParentDir => {
                out.pop();
            }
            std::path::Component::CurDir => {}
            other => out.push(other),
        }
    }
    out
}

fn rel(root: &Path, p: &Path) -> String {
    p.strip_prefix(root).unwrap_or(p).to_string_lossy().into_owned()
}

/// Los markdown bajo `root`, sin `.git`, `target` ni los clones de otras capas.
fn markdown_files(root: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    walk(root, &mut out);
    out.sort();
    out
}

fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    for e in rd.flatten() {
        let p = e.path();
        let name = e.file_name().to_string_lossy().into_owned();
        if p.is_dir() {
            if matches!(name.as_str(), ".git" | "target" | "node_modules" | ".bilink") {
                continue;
            }
            walk(&p, out);
        } else if name.ends_with(".md") {
            out.push(p);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn un_link_normal_se_cuenta() {
        let l = links_in("Ver [node](node.md) para el formato.");
        assert_eq!(l.len(), 1);
        assert_eq!(l[0].0, "node.md");
    }

    /// **Un ejemplo no es una referencia.** Es la exclusión que comparte con el
    /// proveedor `doc`, y sin ella los ejemplos con que `provider.md` explica esta
    /// misma regla saldrían rotos para siempre.
    #[test]
    fn un_link_adentro_de_un_bloque_de_codigo_no_cuenta() {
        let doc = "Real: [node](node.md).\n\n```markdown\nVer [capture.md](capture.md).\n```\n";
        let l = links_in(doc);
        assert_eq!(l.len(), 1, "sólo el de afuera: {l:?}");
        assert_eq!(l[0].0, "node.md");
    }

    /// Una cerca de cuatro backticks contiene bloques de tres — que es como
    /// `provider.md` muestra los suyos.
    #[test]
    fn una_cerca_mas_larga_contiene_a_las_de_adentro() {
        let doc = "````markdown\nUno [a](a.md)\n```\nDos [b](b.md)\n```\nTres [c](c.md)\n````\n[real](d.md)";
        let l = links_in(doc);
        assert_eq!(l.len(), 1, "sólo el de afuera de todo: {l:?}");
        assert_eq!(l[0].0, "d.md");
    }

    #[test]
    fn una_imagen_no_es_una_referencia() {
        assert!(links_in("![alt](x.png)").is_empty());
    }

    #[test]
    fn los_links_externos_no_se_verifican() {
        assert!(links_in("[git](https://git-scm.com) y [mail](mailto:a@b)").is_empty());
    }

    #[test]
    fn el_texto_puede_llevar_corchetes_y_codigo() {
        let l = links_in("Ver [`concepts/ref.md`](../concepts/ref.md#un-commit).");
        assert_eq!(l.len(), 1);
        assert_eq!(l[0].0, "../concepts/ref.md#un-commit");
    }

    /// El caso que produjo 3 de los 4 anchors roto del árbol.
    #[test]
    fn el_backtick_se_saca_sin_dejar_guion() {
        assert_eq!(slug("`.bilink/head` — de dónde salió el árbol"),
                   "bilinkhead-de-dónde-salió-el-árbol");
    }

    #[test]
    fn los_headings_de_adentro_de_un_bloque_no_son_headings() {
        let doc = "# Uno\n\n```md\n# No\n```\n\n## Dos\n";
        let h = headings(doc);
        assert!(h.contains("uno") && h.contains("dos"));
        assert!(!h.contains("no"), "{h:?}");
    }

    #[test]
    fn el_anchor_escapado_se_decodifica() {
        assert_eq!(decode("de-d%C3%B3nde"), "de-dónde");
    }
}
