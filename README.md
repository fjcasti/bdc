# BDC 1.6 - Base de Conocimiento

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
- **Enter** — nueva línea (el cursor vuelve a la primera columna)
- **Ctrl+V** — pegar desde el portapapeles, incluido texto multilínea
- **Backspace** — borrar el último carácter; sobre un salto de línea vuelve al final de la línea anterior
- **ESC** — aceptar el texto e introducir etiquetas

A continuación se solicitan etiquetas (una por línea). **Enter vacío** termina.

También se puede pasar texto directamente como argumento:

```
bdc /a esto es una nota rápida
```

El editor se abre con ese texto pre-cargado para continuar escribiendo.

### Buscar

Varias palabras — se devuelven entradas que contengan todas ellas (AND), en caso de no encontrar ninguna coincidencias se busca por cualquiera de ellas (OR):

```
bdc /b rust sqlite
```

Frase exacta — se encierra entre comillas:

```
bdc /b "base de datos"
```

Si se proporciona texto pero no parámetros se considera el parámetro **/b**. Es decir por defecto busca.
Estos dos comandos son equivalentes

```
bdc /b rust sqlite
bdc rust sqlite
```


La búsqueda es insensible a mayúsculas y acentos. Se busca tanto en el contenido del texto como en las etiquetas.

### Configurar la base de datos

La ruta de la base de datos se resuelve en este orden de prioridad:

1. **Parámetro `/BD`** en línea de comandos:
   ```
   bdc /BD C:\ruta\mi_bd.db /b texto
   ```

2. **Fichero `bdc.ini`** en la misma carpeta que el ejecutable, con la sección y la clave:
   ```ini
   [base_de_datos]
   ruta=C:\ruta\mi_bd.db
   ```

3. **Valor por defecto**: `bdc.db` en la misma carpeta que el ejecutable.

Si la ruta indicada no existe o el fichero está vacío se avisa por pantalla y se pasa a la siguiente opción de la lista.

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

## Cambios

### 1.6

- Corregidos los retornos de carro del editor: en modo raw el terminal no traduce `\n` a CR+LF, por lo que al pulsar **Enter** el cursor bajaba de línea sin volver a la primera columna (efecto escalera). Afectaba tanto al texto como a la petición de etiquetas.
- El pegado con **Ctrl+V** ya no conmuta el modo del terminal: los saltos de línea se expanden al escribir y el texto se almacena con `\n` limpio. Al pegar en una etiqueta los saltos se convierten en espacios.
- El texto pasado como argumento (`bdc /a texto`) se pinta respetando sus saltos de línea.
- **Backspace** sobre un salto de línea coloca el cursor al final de la línea anterior en vez de dejarlo parado en la columna 0.

### 1.5

- Si se proporciona texto sin parámetros se busca por defecto (equivale a `/b`).

### 1.4

- Búsqueda AND con repliegue a OR cuando no hay coincidencias.
- Ruta de la base de datos configurable por `bdc.ini` y por `/BD`.
