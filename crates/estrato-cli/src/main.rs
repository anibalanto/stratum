use clap::{Parser, Subcommand};
use stratum::{parse, resolve, Context, Query};
use std::process;

#[derive(Parser)]
#[command(name = "stratum", about = "Navegación y gestión de capas Stratum")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
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

        Some(Commands::Add { name, remote, branch, force }) => {
            if let Err(e) = cmd_add(&cwd, &name, &remote, &branch, force) {
                eprintln!("error: {e}");
                process::exit(1);
            }
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
                        let display = path.strip_prefix(&cwd).unwrap_or(&path);
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

fn cmd_add(
    cwd: &std::path::Path,
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
