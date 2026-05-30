# Decisión: algoritmos de navegación

## Parsing de la query

La query es un string como `'>tech-decisions>impl'` o `'<<'` o `'>?'`.

Algoritmo de parseo:
1. Si la query es `'>?'` → modo consulta down.
2. Si la query es `'<?'` → modo consulta up.
3. Si comienza con `'<'` → contar `<` consecutivos → modo up N.
4. Si comienza con `'>'` → extraer segmentos entre `>` → modo down.
5. Cualquier otra forma → error de sintaxis, código 2.

## Down `'>name1>name2>...'`

1. Dividir el string por `>` descartando el primer elemento vacío.
2. Construir el path intercalando `.stratum/`:
   ```
   ["a", "b", "c"]  →  .stratum/a/.stratum/b/.stratum/c
   ```
3. Verificar que el path existe (`std::fs::metadata`).
4. Si existe: retornar el path. Si no: `Err`.

## Up `'<<<...'`

1. Contar la cantidad de `<` en el string → N.
2. Detectar profundidad actual: recorrer los componentes del directorio de trabajo
   hacia arriba contando pares `.stratum/<nombre>` consecutivos.
3. Si profundidad < N → `Err`.
4. Construir el path: repetir `../..` N veces unido con el separador de plataforma.

### Detección de profundidad

Recorrer `std::env::current_dir()` hacia la raíz. Por cada par de componentes `(.stratum, <nombre>)` encontrado en orden inverso, incrementar el contador. Detener al encontrar un componente que no sea `.stratum` ni su nombre hijo.

Ejemplo: `/home/user/proyecto/.stratum/tech-decisions/.stratum/impl` → pares: `(.stratum, tech-decisions)`, `(.stratum, impl)` → profundidad 2.

## Consultas `>?` y `<?`

- `>?` → listar entradas de `.stratum/` en el directorio actual, retornar sus nombres.
- `<?` → ejecutar detección de profundidad, retornar el número.

## Sin argumentos

Ejecutar `>?` y `<?` y formatear la salida combinada.
