//! Todo lo que se muestra al usuario y lo que se le pregunta por consola.

use std::io::{self, BufRead, IsTerminal, Write};
use std::path::Path;

use termimad::MadSkin;
use termimad::crossterm::style::Stylize;

use crate::db::Nota;
use crate::texto::normalizar_etiqueta;

pub fn mostrar_ayuda(ruta_bd: &Path) {
    println!(
        "BDC {} - Base de Conocimiento

  Uso: BDC [opcion] [texto]

   -?, -h           Muestra esta ayuda.
   -a  [XXX]        Añadir un registro con el texto XXX al fichero de datos.
  [-b] XXX          Busca el texto XXX en el fichero de datos.
   -e  ID           Edita el registro con identificador ID.
   -bd FICHERO      Usa o establece FICHERO como base de datos.

Fichero de datos: {}",
        crate::VERSION,
        ruta_bd.display()
    );
}

fn es_terminal() -> bool {
    io::stdout().is_terminal()
}

fn separador() -> String {
    let ancho = if es_terminal() { termimad::terminal_size().0 as usize } else { 60 };
    "─".repeat(ancho.clamp(20, 100))
}

/// Muestra notas completas: ID, fecha, etiquetas y el texto con formato
/// Markdown. Si la salida no es un terminal (redirección a fichero o tubería)
/// el texto se escribe tal cual, sin códigos de color.
pub fn mostrar_notas(notas: &[Nota]) {
    let terminal = es_terminal();
    let skin = MadSkin::default();
    let separador = separador();

    for nota in notas {
        let etiquetas = if nota.etiquetas.is_empty() { "(sin etiquetas)".to_owned() } else { nota.etiquetas.join(", ") };
        if terminal {
            println!("{}", separador.as_str().dark_grey());
            println!("{} {}  {}", "ID".bold(), nota.id.to_string().bold().yellow(), nota.creado_en.as_str().dark_grey());
            println!("{} {}", "Etiquetas:".bold(), etiquetas.as_str().cyan());
            println!();
            print!("{}", skin.term_text(&nota.contenido));
        } else {
            println!("{separador}");
            println!("ID {}  {}", nota.id, nota.creado_en);
            println!("Etiquetas: {etiquetas}");
            println!();
            println!("{}", nota.contenido);
        }
    }
    if !notas.is_empty() {
        if terminal {
            println!("{}", separador.as_str().dark_grey());
        } else {
            println!("{separador}");
        }
    }
}

fn leer_linea(pregunta: &str) -> io::Result<Option<String>> {
    print!("{pregunta}");
    io::stdout().flush()?;
    let mut linea = String::new();
    if io::stdin().lock().read_line(&mut linea)? == 0 {
        println!();
        return Ok(None); // fin de la entrada
    }
    Ok(Some(linea.trim().to_owned()))
}

fn lista(etiquetas: &[String]) -> String {
    if etiquetas.is_empty() { "(ninguna)".into() } else { etiquetas.join(", ") }
}

/// Pide etiquetas al usuario partiendo de las actuales. Cada línea es una
/// etiqueta; `-etiqueta` la quita; una línea vacía termina. Si en una línea se
/// escriben varias palabras, cada una se toma como una etiqueta.
pub fn pedir_etiquetas(actuales: &[String]) -> io::Result<Vec<String>> {
    let mut etiquetas = actuales.to_vec();
    println!();
    println!("Etiquetas: {}", lista(&etiquetas));
    println!("Escribe una etiqueta y pulsa ENTER para añadirla, -etiqueta para quitarla, ENTER vacío para terminar.");

    while let Some(linea) = leer_linea("> ")? {
        if linea.is_empty() {
            break;
        }
        for palabra in linea.split_whitespace() {
            let (quitar, nombre) = match palabra.strip_prefix('-') {
                Some(resto) => (true, resto),
                None => (false, palabra),
            };
            let Some(etiqueta) = normalizar_etiqueta(nombre) else { continue };
            let posicion = etiquetas.iter().position(|e| *e == etiqueta);
            match (quitar, posicion) {
                (false, None) => etiquetas.push(etiqueta),
                (false, Some(_)) => println!("  «{etiqueta}» ya estaba."),
                (true, Some(i)) => {
                    etiquetas.remove(i);
                }
                (true, None) => println!("  «{etiqueta}» no estaba."),
            }
        }
        println!("Etiquetas: {}", lista(&etiquetas));
    }
    Ok(etiquetas)
}

/// Pregunta sí/no; por defecto no.
pub fn confirmar(pregunta: &str) -> io::Result<bool> {
    let respuesta = leer_linea(&format!("{pregunta} [s/N] "))?.unwrap_or_default().to_lowercase();
    Ok(matches!(respuesta.as_str(), "s" | "si" | "sí" | "y" | "yes"))
}
