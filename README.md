<p align="center">
  <img src="https://raw.githubusercontent.com/anibalanto/accreta/main/images/stratum.png" alt="Stratum" width="200"/>
</p>

Stratum es un modelo de navegación por capas para proyectos de software. Define cómo estructurar especificaciones, decisiones técnicas e implementación en repositorios git independientes, y provee una CLI para navegar entre ellos con una sintaxis concisa.

## El problema

El conocimiento de un proyecto existe en múltiples niveles — specs funcionales, ADRs, implementación, tests — que típicamente viven en herramientas inconexas. Stratum define una convención de estructura que hace ese conocimiento navegable sin imponer formatos ni herramientas.

## Instalación

```bash
cargo install --path crates/estrato-cli
```

## Concepto: capas

Cada capa vive en un repositorio git independiente. Las capas internas de un proyecto se ubican en `.stratum/<nombre>/`:

```
mi-proyecto/              ← spec layer (git repo)
  .stratum/
    impl/                 ← impl layer (git repo independiente)
      .stratum/
        tests/            ← tests layer (git repo independiente)
```

## Paths Stratum

Un path Stratum es una expresión que identifica ubicaciones en el árbol de capas. Se pasa siempre entre comillas simples porque contiene caracteres especiales del shell:

```bash
cd $(stratum '>impl')
```

### Tokens

| Expresión | Resuelve a | Descripción |
|-----------|-----------|-------------|
| `>name` | `.stratum/name` | Bajar a la sub-capa `name` |
| `>a>b` | `.stratum/a/.stratum/b` | Bajar múltiples niveles |
| `>name/src` | `.stratum/name/src` | Bajar y continuar con path |
| `<` | `../..` | Subir un nivel Stratum |
| `<<` | `../../../..` | Subir dos niveles |
| `<*` | raíz del proyecto | Ancestro `.git` más cercano |
| `*` | raíz del proyecto externo | Ancestro `.git` más lejano |
| `*/subsystems/foo` | `<raíz>/subsystems/foo` | Path absoluto desde raíz externa |

### Ejemplos

```bash
# Navegar a la capa impl
cd $(stratum '>impl')

# Volver al nivel anterior
cd $(stratum '<')

# Ir a una ruta dentro de impl
ls $(stratum '>impl/src/main.rs')

# Referenciar desde la raíz del proyecto
cat $(stratum '*/docs/architecture.md')

# Navegar a una capa anidada
cd $(stratum '>tech-decisions>impl')
```

## Comandos

### `stratum` — contexto actual

Sin argumentos, muestra la profundidad y las sub-capas disponibles:

```
$ stratum
up:   1  (../..)
down: impl  tech-decisions
```

```
$ stratum
up:   0
down: (ninguna)
```

### `stratum pws` — path Stratum del directorio actual (print working stratum)

```bash
$ stratum pws
*/subsystems/stratum>impl/crates/estrato-cli/src
```

### `stratum tree [path]` — árbol de capas

Muestra el árbol de sub-capas a partir del path dado (default: `*`):

```
$ stratum tree '*'
*
└── subsystems
    ├── bilinker
    │   └── >impl
    ├── impact
    │   └── >tech-decisions
    │       └── >impl
    └── stratum
        └── >impl
```

```
$ stratum tree '*/subsystems/stratum'
*/subsystems/stratum
└── >impl
```

### `stratum add <nombre> <remote> [rama]` — registrar sub-capa

Crea el archivo de configuración `.stratum/.<nombre>.toml` con la referencia al repo git de la capa:

```bash
stratum add impl git@github.com:org/mi-proyecto-impl.git
stratum add impl git@github.com:org/mi-proyecto-impl.git develop
stratum add impl git@github.com:org/mi-proyecto-impl.git main --force
```

### Consultas de navegación

```bash
# Resolver un path y usarlo en un comando
ls $(stratum '>impl/src')

# Listar sub-capas disponibles
stratum '>?'
#→ impl  tech-decisions

# Ver profundidad actual
stratum '<?'
#→ 2
```

## Códigos de salida

| Código | Condición |
|--------|-----------|
| 0 | Éxito |
| 1 | Path no encontrado o profundidad insuficiente |
| 2 | Expresión inválida |

## Integración con el ecosistema

```
stratum      ← estructura y navegación entre capas
bilinker     ← referencias verificables entre fragmentos
impact       ← análisis de cambios entre capas
worklist     ← trabajo pendiente entre capas
```

Los paths Stratum son el lenguaje de referencia del ecosistema. `bilinker` los usa en los endpoints layer de los archivos `.bilink`.
