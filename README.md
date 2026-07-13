# BDC 1.2 - Base de Conocimiento

Aplicación de consola para Windows que permite almacenar y recuperar textos con etiquetas, usando SQLite como base de datos embebida.

## Características

- Introducción de texto multilínea con soporte de copiar y pegar
- Etiquetado de entradas con múltiples etiquetas reutilizables
- Búsqueda por texto libre o por etiqueta, con soporte de múltiples palabras (OR) y frases exactas
- Renderizado de formato Markdown en los resultados
- Ruta de la base de datos configurable por fichero INI o parámetro de línea de comandos

## Uso

```
BDC [opcion] [texto]

  /?         Muestra esta ayuda
  /a [XXX]   Añadir el texto XXX al fichero de datos
  /b XXX     Busca palabras (OR). Entre comillas busca frase exacta
  /BD FILE   Usa FILE como fichero de datos
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

Varias palabras — se devuelven entradas que contengan cualquiera de ellas (OR):

```
bdc /b rust sqlite
```

Frase exacta — se encierra entre comillas:

```
bdc /b "base de datos"
```

La búsqueda es insensible a mayúsculas y acentos. Se busca tanto en el contenido del texto como en las etiquetas.

### Configurar la base de datos

La ruta de la base de datos se resuelve en este orden de prioridad:

1. **Parámetro `/BD`** en línea de comandos:
   ```
   bdc /BD C:\ruta\mi_bd.db /b texto
   ```

2. **Fichero `bdc.ini`** en la misma carpeta que el ejecutable, con la entrada:
   ```
   bbdd=C:\ruta\mi_bd.db
   ```

3. **Valor por defecto** compilado en la constante `DB_PATH`.

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
