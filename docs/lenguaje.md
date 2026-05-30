# Decisión: lenguaje y estructura del proyecto

## Lenguaje: Rust

- Binario sin runtime — `stratum` se instala como cualquier herramienta de sistema.
- `std::path` maneja paths multiplataforma sin dependencias externas.
- Misma decisión que bilinker — consistencia en el ecosistema.

## Estructura: workspace con dos crates

```
impl/
  Cargo.toml          ← workspace
  crates/
    stratum/          ← biblioteca: lógica pura de navegación
    stratum-cli/      ← binario: CLI encima de la biblioteca
```

La biblioteca no depende de clap ni de I/O de terminal. Expone funciones puras que operan sobre paths. Cualquier sistema externo (Accreta, un editor, scripts) puede usar `stratum` como crate sin arrastrar el CLI.

`stratum-cli` parsea la query con clap, llama a la biblioteca, imprime resultado a stdout y errores a stderr.

## Dependencias

| Crate | Usado en | Propósito |
|---|---|---|
| `clap` | `stratum-cli` | parsing del argumento query |

Sin I/O asíncrono ni base de datos — todo es síncrono sobre el filesystem.
