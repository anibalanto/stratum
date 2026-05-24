use clap::Parser;
use estrato::{parse, resolve, Context, Query};
use std::process;

#[derive(Parser)]
#[command(name = "estrato", about = "Navegación por capas Estrato")]
struct Cli {
    /// Path Estrato: '>name>name2/fs-path', '<<', '>?', '<?', o path tradicional
    query: Option<String>,
}

fn main() {
    let cli = Cli::parse();

    let cwd = std::env::current_dir().unwrap_or_else(|e| {
        eprintln!("error: {e}");
        process::exit(1);
    });

    let Some(raw) = cli.query else {
        let ctx = Context::from_dir(&cwd);
        match ctx.up_path() {
            Some(p) => println!("up:   {}  ({})", ctx.depth, p),
            None => println!("up:   0"),
        }
        if ctx.sub_layers.is_empty() {
            println!("down: (ninguna)");
        } else {
            println!("down: {}", ctx.sub_layers.join("  "));
        }
        return;
    };

    let query = match parse(&raw) {
        Ok(q) => q,
        Err(e) => {
            eprintln!("error: {e}");
            process::exit(2);
        }
    };

    match query {
        Query::Path(tokens) => match resolve(&cwd, &cwd, &tokens) {
            Ok(path) => {
                let display = path.strip_prefix(&cwd).unwrap_or(&path);
                println!("{}", display.display());
            }
            Err(e) => {
                eprintln!("error: {e}");
                process::exit(1);
            }
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
