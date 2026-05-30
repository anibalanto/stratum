use clap::{Parser, Subcommand};
use stratum::{parse, parse_path, resolve, cwd_as_stratum_path, format_path, Context, Query};
use std::path::{Path, PathBuf};
use std::process;

#[derive(Parser)]
#[command(name = "stratum", about = "Navegación y gestión de capas Stratum")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Muestra el path Stratum del directorio actual (desde *)
    Pws,
    /// Registra una nueva sub-capa con su repositorio git
    Add {
        /// Nombre de la sub-capa
        name: String,
        /// URL del repositorio git (ssh o https)
        remote: String,
        /// Rama (default: main)
        #[arg(default_value = "main")]
        branch: String,
        /// Sobreescribir si ya existe
        #[arg(long)]
        force: bool,
    },
    /// Muestra el árbol de capas a partir de un path Stratum
    Tree {
        /// Path Stratum de inicio (default: *)
        #[arg(default_value = "*")]
        path: String,
    },
    /// Path Stratum o consulta de navegación
    #[command(external_subcommand)]
    Query(Vec<String>),
}

fn main() {
    let cli = Cli::parse();

    let cwd = std::env::current_dir().unwrap_or_else(|e| {
        eprintln!("error: {e}");
        process::exit(1);
    });

    match cli.command {
        None => {
            let ctx = Context::from_dir(&cwd);
            match ctx.up_path() {
                Some(p) => println!("up:   {}  ({})", ctx.depth, p),
                None    => println!("up:   0"),
            }
            if ctx.sub_layers.is_empty() {
                println!("down: (ninguna)");
            } else {
                println!("down: {}", ctx.sub_layers.join("  "));
            }
        }

        Some(Commands::Pws) => {
            match cwd_as_stratum_path(&cwd) {
                Some(tokens) => println!("{}", format_path(&tokens)),
                None => { eprintln!("error: no se encontró raíz .git"); process::exit(1); }
            }
        }

        Some(Commands::Add { name, remote, branch, force }) => {
            if let Err(e) = cmd_add(&cwd, &name, &remote, &branch, force) {
                eprintln!("error: {e}");
                process::exit(1);
            }
        }

        Some(Commands::Tree { path }) => {
            let tokens = match parse_path(&path) {
                Ok(t)  => t,
                Err(e) => { eprintln!("error: {e}"); process::exit(2); }
            };
            let root = match resolve(&cwd, &cwd, &tokens) {
                Ok(p)  => p,
                Err(e) => { eprintln!("error: {e}"); process::exit(1); }
            };
            println!("{}", path);
            print_stratum_children(&root, "");
        }

        Some(Commands::Query(args)) => {
            let raw = args.first()
                .map(|s| s.as_str())
                .unwrap_or_else(|| { eprintln!("error: query vacía"); process::exit(2); });

            let query = match parse(raw) {
                Ok(q)  => q,
                Err(e) => { eprintln!("error: {e}"); process::exit(2); }
            };

            match query {
                Query::Path(tokens) => match resolve(&cwd, &cwd, &tokens) {
                    Ok(path) => {
                        let rel = path.strip_prefix(&cwd).unwrap_or(&path);
                        let display = if rel.as_os_str().is_empty() { &path } else { rel };
                        println!("{}", display.display());
                    }
                    Err(e) => { eprintln!("error: {e}"); process::exit(1); }
                },
                Query::ListDown => {
                    let ctx = Context::from_dir(&cwd);
                    if ctx.sub_layers.is_empty() {
                        println!("(ninguna)");
                    } else {
                        println!("{}", ctx.sub_layers.join("  "));
                    }
                }
                Query::ListUp => {
                    let ctx = Context::from_dir(&cwd);
                    println!("{}", ctx.depth);
                }
            }
        }
    }
}

fn print_stratum_children(dir: &Path, prefix: &str) {
    let children = stratum_children(dir);
    let n = children.len();
    for (i, (label, path)) in children.into_iter().enumerate() {
        let is_last = i == n - 1;
        let conn   = if is_last { "└── " } else { "├── " };
        let extend = if is_last { "    " } else { "│   " };
        println!("{}{}{}", prefix, conn, label);
        print_stratum_children(&path, &format!("{}{}", prefix, extend));
    }
}

fn stratum_children(dir: &Path) -> Vec<(String, PathBuf)> {
    let mut children = vec![];

    let stratum_dir = dir.join(".stratum");
    if let Ok(entries) = std::fs::read_dir(&stratum_dir) {
        let mut layers: Vec<_> = entries
            .flatten()
            .filter(|e| e.path().is_dir())
            .filter_map(|e| e.file_name().into_string().ok())
            .collect();
        layers.sort();
        for layer in layers {
            children.push((format!(">{}", layer), stratum_dir.join(&layer)));
        }
    }

    if let Ok(entries) = std::fs::read_dir(dir) {
        let mut sub_dirs: Vec<_> = entries
            .flatten()
            .filter(|e| {
                let p = e.path();
                let s = e.file_name();
                let name = s.to_string_lossy();
                p.is_dir() && !name.starts_with('.') && has_stratum_content(&p)
            })
            .collect();
        sub_dirs.sort_by_key(|e| e.file_name());
        for entry in sub_dirs {
            children.push((entry.file_name().into_string().unwrap_or_default(), entry.path()));
        }
    }

    children
}

fn has_stratum_content(dir: &Path) -> bool {
    let stratum_dir = dir.join(".stratum");
    if stratum_dir.is_dir() {
        if let Ok(entries) = std::fs::read_dir(&stratum_dir) {
            if entries.flatten().any(|e| e.path().is_dir()) {
                return true;
            }
        }
    }
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let p = entry.path();
            let n = entry.file_name();
            if p.is_dir() && !n.to_string_lossy().starts_with('.') && has_stratum_content(&p) {
                return true;
            }
        }
    }
    false
}

fn cmd_add(
    cwd: &Path,
    name: &str,
    remote: &str,
    branch: &str,
    force: bool,
) -> anyhow::Result<()> {
    if name.is_empty() || name.contains('/') || name.contains('\\') {
        anyhow::bail!("nombre de capa inválido: '{name}'");
    }

    let stratum_dir = cwd.join(".stratum");
    std::fs::create_dir_all(&stratum_dir)?;

    let config_path = stratum_dir.join(format!(".{name}.toml"));
    if config_path.exists() && !force {
        anyhow::bail!(".stratum/.{name}.toml ya existe — usa --force para sobreescribir");
    }

    let content = format!("remote = \"{remote}\"\nbranch = \"{branch}\"\n");
    std::fs::write(&config_path, &content)?;

    eprintln!("creado: .stratum/.{name}.toml");
    Ok(())
}
