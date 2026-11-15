# BDC 2.0 - Base de Conocimiento

Aplicación de consola (Windows y Linux) para guardar y recuperar notas clasificadas con etiquetas, usando SQLite como base de datos embebida.

## Características

- Las notas se escriben con el **editor de texto que prefieras** (configurable).
- Etiquetado de notas con múltiples etiquetas reutilizables, sin mayúsculas ni acentos.
- Búsqueda por texto libre en el contenido y en las etiquetas, insensible a mayúsculas y acentos.
- Edición y borrado de notas existentes.
- Renderizado de formato Markdown en los resultados.
- Base de datos configurable por fichero INI o por línea de comandos.

## Uso

```
BDC 2.0 - Base de Conocimiento

  Uso: BDC [opcion] [texto]

   -?, -h           Muestra esta ayuda.
   -a  [XXX]        Añadir un registro con el texto XXX al fichero de datos.
  [-b] XXX          Busca el texto XXX en el fichero de datos.
   -e  ID           Edita el registro con identificador ID.
   -bd FICHERO      Usa o establece FICHERO como base de datos.

Fichero de datos: /ruta/al/fichero/de/bbdd/bdc.db
```

Sin opciones ni texto se muestra la ayuda. `-?` o `-h` junto con cualquier otra opción también muestran solo la ayuda.

Las opciones se aceptan con guion (`-a`) o con barra (`/a`), sin distinguir mayúsculas. Las opciones de acción (`-a`, `-b`, `-e`) no se pueden combinar entre sí, pero sí con `-bd`.

### Añadir una nota

```
bdc -a
bdc -a esto es una nota rápida
```

1. Se abre el editor con un fichero temporal (`.md`), vacío o con el texto indicado.
2. Escribe la nota, guarda y **cierra el editor**.
3. La nota se guarda y se piden las etiquetas: una por línea; **ENTER vacío** termina.

Si el texto queda vacío no se guarda nada.

### Buscar

```
bdc rust sqlite
bdc -b rust sqlite
bdc "base de datos"
```

- Varias palabras: se muestran las notas que contienen **todas**; si no hay ninguna, las que contienen **alguna**.
- Entre comillas: frase exacta.
- Se busca en el texto y en las etiquetas, sin distinguir mayúsculas ni acentos.
- De cada nota se muestra el ID, la fecha, las etiquetas y el texto completo con formato Markdown.

### Editar una nota

```
bdc -e 12
```

Se abre el editor con el texto de la nota. Al cerrarlo se guarda el texto y se muestran las etiquetas actuales para modificarlas:

- `etiqueta` + ENTER — la añade.
- `-etiqueta` + ENTER — la quita.
- ENTER vacío — termina.

Si se deja la nota vacía se pregunta si se quiere **borrar**.

### Etiquetas

- Se normalizan quitando acentos y mayúsculas: `Programación` → `programacion`.
- Son únicas: si ya existe se reutiliza. Las que se quedan sin notas se eliminan.
- Si se escriben varias palabras en una línea, cada una es una etiqueta.

### Base de datos

La base de datos se elige en este orden:

1. **Opción `-bd FICHERO`** junto con otra opción: se usa solo en esa ejecución. Debe existir y ser una base de datos de BDC; si no, se muestra el error y la ayuda.
   ```
   bdc -bd /ruta/otra.db -b texto
   ```
2. **Fichero `bdc.ini`** junto al ejecutable, propiedad `ruta` de la sección `[base_de_datos]`.
3. **`bdc.db` junto al ejecutable**, que se crea vacía si no existe. También se usa, con un aviso, si la configurada en `bdc.ini` no existe.

`bdc -bd FICHERO` **sin más opciones** comprueba la base de datos y la guarda en `bdc.ini` como la base de datos por defecto.

Las bases de datos de las versiones 1.x son compatibles.

### Configuración: `bdc.ini`

Se lee de la carpeta del ejecutable. Ver el ejemplo [`bdc.ini`](bdc.ini).

```ini
[principal]
editor=code --wait

[base_de_datos]
ruta=/home/usuario/Documentos/bdc/bdc.db
```

**Editor**: la propiedad `editor` puede llevar argumentos y rutas con espacios entre comillas. Los editores gráficos tienen que esperar a que se cierre el fichero (`code --wait`); si no, BDC avisa de que el editor se cerró al momento sin cambios. Sin `editor` se usan las variables de entorno `VISUAL` o `EDITOR` y, en último caso, `notepad` en Windows y `nano` (o `vi`) en Linux.

## Compilar

```
cargo build --release
cargo test
```

El ejecutable queda en `target/release/bdc` (`bdc.exe` en Windows). Copia junto a él el `bdc.ini`.

## Estructura

| Fichero | Contenido |
|---|---|
| `src/main.rs` | Punto de entrada y acciones (añadir, buscar, editar, `-bd`) |
| `src/cli.rs` | Análisis de la línea de comandos |
| `src/config.rs` | Lectura y escritura de `bdc.ini` |
| `src/db.rs` | Base de datos SQLite |
| `src/editor.rs` | Fichero temporal y editor externo |
| `src/salida.rs` | Ayuda, resultados en Markdown y petición de etiquetas |
| `src/texto.rs` | Normalización y términos de búsqueda |
| `bdc.sql` | Esquema de la base de datos |

## Tecnología

| Crate | Uso |
|---|---|
| `rusqlite` | Base de datos SQLite embebida |
| `termimad` | Renderizado de Markdown en consola |
| `unicode-normalization` | Normalización de etiquetas y búsquedas (acentos, mayúsculas) |

## Cambios

### 2.0

Reescritura completa según [`especificaciones.md`](especificaciones.md).

- El editor integrado se sustituye por un **editor externo configurable** (`[principal] editor` en `bdc.ini`).
- Nueva opción **`-e ID`** para editar una nota y sus etiquetas. Vaciar la nota permite borrarla.
- **`-bd FICHERO`** sin más opciones guarda la base de datos en `bdc.ini`. Con otras opciones, el fichero debe existir y ser válido.
- La búsqueda ignora los acentos también en el texto de las notas, no solo en las etiquetas.
- Los errores de uso muestran el mensaje y la ayuda, y terminan con código de salida 1.
- Desaparecen las dependencias `crossterm` y `arboard`.

### 1.6

- Corregidos los retornos de carro del editor en modo raw.

### 1.5

- Si se proporciona texto sin parámetros se busca por defecto (equivale a `/b`).

### 1.4

- Búsqueda AND con repliegue a OR cuando no hay coincidencias.
- Ruta de la base de datos configurable por `bdc.ini` y por `/BD`.
