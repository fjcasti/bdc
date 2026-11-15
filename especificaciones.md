# Base de conocimiento
Aplicación de consola que permite almacenar y recuperar texto clasificados con etiquetas.

## Características
- La aplicación ha de funcionar en modo consola. 
- Programada en rust.
- Gestionará una base de datos sqlite3 en la que puede leer, escribir, editar y borrar entradas para presentarlas al usuario por consola.
- Renderizará el formato markdown en los resultados.
- fichero de base de datos configurable.
- editor de texto configurable.
- Al terminar, ya sea con éxito o con error, se devolverá el control al interprete de comandos.


## Base de datos

La estructura de la base de datos está en el fichero `bdc.sql`. Está sujeta a cambios si alguna funcionalidad lo requiere.

Si la base de datos no existe la aplicación creará una vacía con el nombre **bdc.bd** en la misma carpeta donde se encuentre el ejecutable.



## Modo de uso
La linea de comandos de la aplicación se ajusta a esta ayuda:

    BDC x.y - Base de Conocimiento

      Uso: BDC [opcion] [texto]

       -?, -h           Muestra esta ayuda.
       -a  [XXX]        Añadir un registro con el texto XXX al fichero de datos.
      [-b] XXX          Busca el texto XXX en el fichero de datos.
       -e  ID           Edita el registro con identificador ID.
       -bd FICHERO      Usa o establece FICHERO como base de datos.
    
    Fichero de datos: /ruta/al/fichero/de/bbdd/bdc.db

### Ayuda

x.y son los números de versión del programa que se muestran en la pantalla de modo de uso.

Al lanzar la aplicación sin opciones y sin texto  muestra la ayuda. El mismo comportamiento tendrán los parámetros -? o -h

Si se combinan -? o -h con cualquier otro parámetro solo se mostrará la ayuda y terminará (anula el resto de parámetros).

Después de mostar el modo de uso se termina dando el control al terminal del usuario.

### Búsqueda

La opción -b no es obligatoria.  

El parámetro -b busca el texto introducido en la BBDD, si no se proporciona texto mostrará un error, el modo de uso y termina
  
Si se lanza sin opciones y con un texto buscará en la BBDD para mostrar todas las entradas que contengan  el texto proporcionado.

Al buscar se mostrarán las entradas encontradas completas mostrando ID, ETIQUETAS y TEXTO COMPLETO

   
*** PENDIENTE DEFINIR COMO ES LA BÚSQUEDA***

### Añadir

Con la opción  -a y sin texto Añadirá una entrada en la bbdd.

Con la opción -a y un texto Añadirá una entrada inicializada con el texto proporcionado.

Añadir una entrada es:
1. Crear un fichero temporal y escribir en él el texto inicial si lo hay.
2. Lanzar un editor como proceso hijo pasándole la ruta del fichero temporal.
3. Esperar a que ese proceso termine, es decir, a que el usuario cierre el editor.
4. Volver a leer el fichero desde la ruta.
5. Guardar el contenido leído del fichero en un registro nuevo de la BBDD.
5. Borrar el fichero temporal.

El editor a usar se configura en el fichero bdc.ini, en la sección "principal" y con el nombre de propiedad "editor".  El fichero bdc.ini en esta carpeta tiene una configuración de ejemplo.

Al terminar de editar solicitará al usuario que añada etiquetas de texto, para clasificar la nota introducida. Pueden ser mas de una. Se introducirán tecleando texto sin espacios y pulsando ENTER y se termina cuando se introduzca un ENTER sin texto.

Las etiquetas se normalizan eliminando acentos y mayúsculas.

Las etiquetas son únicas — si introduces una que ya existe se reutiliza, no se duplica

El texto vacío no se guarda.

### Editar

Editar modifica una entrada ya guardada.

Es obligatorio en introducir el ID de la entrada.

Editar una entrada es:

1. Buscar la nota con el ID proporcionado.
2. Crear un fichero temporal y escribir en él el texto de la nota leída.
2. Lanzar un editor como proceso hijo pasándole la ruta del fichero temporal.
3. Esperar a que ese proceso termine, es decir, a que el usuario cierre el editor.
4. Volver a leer el fichero desde la ruta.
5. Guardar el contenido leído del fichero sobreescribiendo la nota original.
5. Borrar el fichero temporal.

Al guardar la nota se mostrarán por consola las etiquetas de esa nota permitiendo borrar o añadir.

### Opción bd

Si se pasa la opción -bd sin fichero o el fichero pasado no existe o no es válido se  aborta la ejecución muestra error, el modo de uso y termina.

Si se pasa la opción -bd junto con otras opciones se usa esa BBDD para esa ejecución, aunque en  el fichero  INI esté configurada otra BBDD. 

Si se pasa la opción -bd sin otras opciones, verifica que existe y es un BBDD válida para la aplicación y modifica la propiedad del fichero bdc.ini con la BBDD indicada.


