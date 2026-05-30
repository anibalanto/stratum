# Decisión: plan de tests

## Estrategia

Tests unitarios en `stratum` (biblioteca). Tests de integración en `stratum-cli` invocando el binario compilado con `std::process::Command`.

Los tests operan sobre directorios temporales construidos en el test mismo con `tempfile::tempdir()` — sin depender del filesystem del proyecto.

## Tests unitarios — crate `stratum`

### Parsing de query

| Caso | Query | Resultado |
|---|---|---|
| down un nivel | `'>impl'` | down `["impl"]` |
| down dos niveles | `'>tech-decisions>impl'` | down `["tech-decisions", "impl"]` |
| up uno | `'<'` | up 1 |
| up tres | `'<<<'` | up 3 |
| consulta down | `'>?'` | query down |
| consulta up | `'<?'` | query up |
| inválida | `'xyz'` | `Err(sintaxis)` |
| vacía | `''` | `Err(sintaxis)` |

### Down

| Caso | Segmentos | Filesystem | Esperado |
|---|---|---|---|
| un nivel, existe | `["impl"]` | `.stratum/impl/` presente | `.stratum/impl` |
| dos niveles, existe | `["a", "b"]` | `.stratum/a/.stratum/b/` presente | `.stratum/a/.stratum/b` |
| no existe | `["noexiste"]` | sin `.stratum/noexiste/` | `Err` |

### Up

| Caso | N | Profundidad actual | Esperado |
|---|---|---|---|
| N=1, profundidad 1 | 1 | 1 | `../..` |
| N=2, profundidad 2 | 2 | 2 | `../../../..` |
| N=1, profundidad 0 | 1 | 0 | `Err` |
| N mayor que profundidad | 3 | 1 | `Err` |

### Detección de profundidad

| Path de trabajo | Profundidad esperada |
|---|---|
| `/proyecto` | 0 |
| `/proyecto/.stratum/impl` | 1 |
| `/proyecto/.stratum/tech-decisions/.stratum/impl` | 2 |

## Tests de integración — crate `stratum-cli`

| Caso | Argumento | stdout | código |
|---|---|---|---|
| down válido | `'>impl'` | `.stratum/impl\n` | 0 |
| down inválido | `'>noexiste'` | vacío | 1 |
| up válido (profundidad 1) | `'<'` | `../..` | 0 |
| up excede profundidad | `'<<<'` (profundidad 1) | vacío | 1 |
| consulta down | `'>?'` | nombres de sub-capas | 0 |
| consulta up | `'<?'` | número de profundidad | 0 |
| sin argumentos | — | líneas up/down | 0 |
| query inválida | `'xyz'` | vacío | 2 |

## Dependencias de test

| Crate | Propósito |
|---|---|
| `tempfile` | directorios temporales para fixtures |
