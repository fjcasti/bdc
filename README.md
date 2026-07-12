# BDC - Base de Conocimiento

Aplicación de consola para Windows que permite almacenar y recuperar textos con etiquetas, usando SQLite como base de datos embebida.

## Características

- Introducción de texto multilínea con soporte de copiar y pegar
- Etiquetado de entradas con múltiples etiquetas reutilizables
- Búsqueda por texto libre o por etiqueta
- Renderizado de formato Markdown en los resultados
- Base de datos SQLite local, creada automáticamente junto al ejecutable

## Uso

```
BDC [opcion] [texto]

  /?         Muestra esta ayuda
  /a [XXX]   Añadir el texto XXX al fichero de datos
  /b XXX     Busca el texto XXX en el fichero de datos
```

### Añadir una entrada

```
bdc /a
```

Se abre el editor interactivo:
- Escribe el texto libremente
- **Enter** — nueva línea
- **Ctrl+V** — pegar desde el portapapeles
- **Backspace** — borrar el último carácter
- **ESC** — aceptar el texto e introducir etiquetas

A continuación se solicitan etiquetas (una por línea). **Enter vacío** termina.

También se puede pasar texto directamente como argumento:

```
bdc /a esto es una nota rápida
```

El editor se abre con ese texto pre-cargado para continuar escribiendo.

### Buscar

```
bdc /b rust
```

Busca en el contenido de los textos y en las etiquetas. Muestra los resultados completos con formato Markdown.

## Compilar

```
cargo build --release
```

El ejecutable queda en `target\release\bdc.exe`.

## Tecnología

| Crate | Uso |
|---|---|
| `crossterm` | Control del terminal y lectura de teclado |
| `arboard` | Acceso al portapapeles del sistema |
| `rusqlite` | Base de datos SQLite embebida |
| `termimad` | Renderizado de Markdown en consola |
| `unicode-normalization` | Normalización de etiquetas (acentos, mayúsculas) |
