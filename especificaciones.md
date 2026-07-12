La aplicación ha de funcionar en modo consola.

Al arrancarla ha de permitir escribir en la consola, pegar texto en la consola y aceptar un enter como cambio de linea.

Con esc aceptará todo el texto introducido.

Al aceptar texto se solicitará al usuario que ponga etiquetas al texto introducido puede ser mas de una cada una se acepta con enter y para terminar ESC

Al aceptar texto se solicitará al usuario que ponga etiquetas al texto introducido puede ser mas de una cada una se acepta con enter y para terminar se pulsa ESC o ENTER con la etiqueta vacía.

Las etiquetas se normalizan eliminando acentos y mayúsculas.

Las etiquetas son únicas — si introduces una que ya existe se reutiliza, no se duplica

El texto vacío no se guarda

La base de datos se crea automáticamente si no existe

La base de datos se crea al lado del ejecutable.

la linea de comandos de la aplicación se ajusta a esta ayuda:

    BDC 1.0 - Base de Conocimiento

      Uso: BDC [opcion] [texto]

      /?              Muestra esta ayuda
      /a [XXX]        Añadir  el texto XXX al fichero de datos
      /b XXX          Busca el texto XXX en el fichero de datos
    
    Fichero de datos: c:\Users\dars\Desktop\Casti\repos\bdc\target\debug\bdc.db 

Al buscar se mostrarán las entradas encontradas completas

Se renderizará el formato markdown

Al terminar de introducir una entrada con sus etiquetas, finalizará y devolverá el control al interprete de comandos.
