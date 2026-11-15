//! BDC - Base de Conocimiento.
//!
//! Aplicación de consola para guardar y recuperar notas clasificadas con
//! etiquetas en una base de datos SQLite. Las notas se escriben con un editor
//! de texto externo y se muestran con formato Markdown.

mod cli;
mod config;
mod db;
mod editor;
mod salida;
mod texto;

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use cli::{Accion, Orden};
use config::Config;
use db::Bd;

pub const VERSION: &str = concat!(env!("CARGO_PKG_VERSION_MAJOR"), ".", env!("CARGO_PKG_VERSION_MINOR"));

/// Motivo por el que termina la ejecución con error.
enum Fallo {
    /// Error de uso: se muestra el mensaje y la ayuda.
    Uso(String),
    /// Cualquier otro error: solo el mensaje.
    Error(String),
}

impl From<rusqlite::Error> for Fallo {
    fn from(e: rusqlite::Error) -> Self {
        Fallo::Error(format!("error en la base de datos: {e}"))
    }
}

impl From<std::io::Error> for Fallo {
    fn from(e: std::io::Error) -> Self {
        Fallo::Error(format!("error de entrada/salida: {e}"))
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args_os()
        .skip(1)
        .map(|a| a.to_string_lossy().into_owned())
        .collect();
    let cfg = Config::cargar();
    for aviso in &cfg.avisos {
        eprintln!("Aviso: {aviso}");
    }

    let resultado = cli::analizar(&args)
        .map_err(|e| Fallo::Uso(e.to_string()))
        .and_then(|orden| ejecutar(orden, &cfg));

    match resultado {
        Ok(()) => ExitCode::SUCCESS,
        Err(Fallo::Uso(mensaje)) => {
            eprintln!("Error: {mensaje}");
            eprintln!();
            salida::mostrar_ayuda(&cfg.resolver_bd().0);
            ExitCode::FAILURE
        }
        Err(Fallo::Error(mensaje)) => {
            eprintln!("Error: {mensaje}");
            ExitCode::FAILURE
        }
    }
}

fn ejecutar(orden: Orden, cfg: &Config) -> Result<(), Fallo> {
    if orden.accion == Accion::Ayuda {
        salida::mostrar_ayuda(&cfg.resolver_bd().0);
        return Ok(());
    }

    // Una base de datos indicada con -bd tiene que existir y ser válida.
    if let Some(ruta) = &orden.bd {
        db::validar(ruta).map_err(|e| Fallo::Uso(format!("la base de datos «{}» {e}", ruta.display())))?;
    }

    if orden.accion == Accion::FijarBd {
        return fijar_bd(orden.bd.as_deref().expect("FijarBd lleva ruta"), cfg);
    }

    let bd = abrir_bd(orden.bd.as_deref(), cfg)?;
    match orden.accion {
        Accion::Anadir(inicial) => anadir(&bd, cfg, &inicial),
        Accion::Buscar(args) => buscar(&bd, &args),
        Accion::Editar(id) => editar(&bd, cfg, id),
        Accion::Ayuda | Accion::FijarBd => unreachable!("tratadas antes"),
    }
}

fn abrir_bd(parametro: Option<&Path>, cfg: &Config) -> Result<Bd, Fallo> {
    if let Some(ruta) = parametro {
        return Bd::abrir_existente(ruta).map_err(|e| Fallo::Error(format!("la base de datos «{}» {e}", ruta.display())));
    }
    let (ruta, aviso) = cfg.resolver_bd();
    if let Some(aviso) = aviso {
        eprintln!("Aviso: {aviso}");
    }
    let (bd, nueva) =
        Bd::abrir_o_crear(&ruta).map_err(|e| Fallo::Error(format!("la base de datos «{}» {e}", ruta.display())))?;
    if nueva {
        eprintln!("Creada una base de datos vacía en «{}».", ruta.display());
    }
    Ok(bd)
}

/// `-bd FICHERO` sin más opciones: lo guarda en bdc.ini (ya validado).
fn fijar_bd(ruta: &Path, cfg: &Config) -> Result<(), Fallo> {
    let absoluta: PathBuf = std::path::absolute(ruta)?;
    cfg.guardar_ruta_bd(&absoluta).map_err(|e| {
        Fallo::Error(format!("no se puede escribir {}: {e}", cfg.ruta_ini().display()))
    })?;
    println!("Base de datos establecida: {}", absoluta.display());
    println!("Guardada en {}", cfg.ruta_ini().display());
    Ok(())
}

fn lanzar_editor(cfg: &Config, inicial: &str) -> Result<editor::Edicion, Fallo> {
    let programa = editor::resolver_editor(cfg.editor.as_deref());
    println!("Abriendo el editor ({programa}). Guarda y cierra el editor para continuar…");
    let edicion = editor::editar(&programa, inicial).map_err(Fallo::Error)?;
    if edicion.sin_cambios && edicion.inmediata {
        eprintln!(
            "Aviso: el editor se ha cerrado al momento sin cambios. Si es un editor gráfico, \
             configúralo para que espere a que cierres el fichero (por ejemplo: editor=code --wait)."
        );
    }
    Ok(edicion)
}

fn anadir(bd: &Bd, cfg: &Config, inicial: &str) -> Result<(), Fallo> {
    let edicion = lanzar_editor(cfg, inicial)?;
    if edicion.texto.is_empty() {
        println!("Texto vacío: no se guarda nada.");
        return Ok(());
    }
    let id = bd.anadir_nota(&edicion.texto)?;
    println!("Nota {id} guardada.");

    let etiquetas = salida::pedir_etiquetas(&[])?;
    bd.fijar_etiquetas(id, &etiquetas)?;
    println!("Etiquetas de la nota {id}: {}.", resumen(&bd.etiquetas(id)?));
    Ok(())
}

fn editar(bd: &Bd, cfg: &Config, id: i64) -> Result<(), Fallo> {
    let nota = bd.nota(id)?.ok_or_else(|| Fallo::Error(format!("no existe ninguna nota con ID {id}.")))?;

    let edicion = lanzar_editor(cfg, &nota.contenido)?;
    if edicion.texto.is_empty() {
        println!("La nota ha quedado vacía.");
        if salida::confirmar(&format!("¿Borrar la nota {id}?"))? {
            bd.borrar_nota(id)?;
            println!("Nota {id} borrada.");
        } else {
            println!("La nota {id} no se ha modificado.");
        }
        return Ok(());
    }
    if edicion.sin_cambios {
        println!("El texto de la nota {id} no ha cambiado.");
    } else {
        bd.actualizar_nota(id, &edicion.texto)?;
        println!("Nota {id} actualizada.");
    }

    let etiquetas = salida::pedir_etiquetas(&nota.etiquetas)?;
    if etiquetas != nota.etiquetas {
        bd.fijar_etiquetas(id, &etiquetas)?;
        println!("Etiquetas de la nota {id}: {}.", resumen(&bd.etiquetas(id)?));
    }
    Ok(())
}

fn buscar(bd: &Bd, args: &[String]) -> Result<(), Fallo> {
    let terminos = texto::terminos_busqueda(args);
    if terminos.is_empty() {
        return Err(Fallo::Uso("falta el texto a buscar.".into()));
    }
    let buscado = args.join(" ");

    // Primero las notas con todos los términos; si no hay, las que tengan alguno.
    let mut notas = bd.buscar(&terminos, true)?;
    let mut parcial = false;
    if notas.is_empty() && terminos.len() > 1 {
        notas = bd.buscar(&terminos, false)?;
        parcial = true;
    }

    match (notas.len(), parcial) {
        (0, _) => println!("No se ha encontrado nada con «{buscado}»."),
        (n, false) => println!("{n} nota(s) con «{buscado}»:"),
        (n, true) => println!("Ninguna nota contiene todos los términos; {n} nota(s) contienen alguno de «{buscado}»:"),
    }
    salida::mostrar_notas(&notas);
    Ok(())
}

fn resumen(etiquetas: &[String]) -> String {
    if etiquetas.is_empty() { "ninguna".into() } else { etiquetas.join(", ") }
}
