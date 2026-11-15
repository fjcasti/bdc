//! Edición de texto con un editor externo a través de un fichero temporal.

use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use crate::texto::{decodificar, limpiar};

pub struct Edicion {
    /// Texto final, ya limpio (saltos `\n`, sin blancos al principio ni al final).
    pub texto: String,
    /// El texto es igual que el inicial.
    pub sin_cambios: bool,
    /// El editor se cerró casi al instante. Junto con `sin_cambios` suele
    /// indicar un editor gráfico que no espera (p. ej. `code` sin `--wait`).
    pub inmediata: bool,
}

/// Editor a usar: el de bdc.ini; si no, `VISUAL` o `EDITOR`; si no, uno por
/// defecto del sistema.
pub fn resolver_editor(configurado: Option<&str>) -> String {
    configurado
        .map(str::to_owned)
        .or_else(|| ["VISUAL", "EDITOR"].iter().find_map(|v| std::env::var(v).ok()))
        .map(|e| e.trim().to_owned())
        .filter(|e| !e.is_empty())
        .unwrap_or_else(editor_por_defecto)
}

fn editor_por_defecto() -> String {
    if cfg!(windows) {
        "notepad".into()
    } else if en_path("nano") {
        "nano".into()
    } else {
        "vi".into()
    }
}

fn en_path(programa: &str) -> bool {
    std::env::var_os("PATH")
        .is_some_and(|path| std::env::split_paths(&path).any(|d| d.join(programa).is_file()))
}

/// Pasos de la especificación: crear un fichero temporal con el texto inicial,
/// abrirlo con el editor, esperar a que se cierre, volver a leerlo y borrarlo.
pub fn editar(editor: &str, inicial: &str) -> Result<Edicion, String> {
    let temporal =
        FicheroTemporal::crear(inicial).map_err(|e| format!("no se puede crear el fichero temporal: {e}."))?;

    let inicio = Instant::now();
    let estado = lanzar(editor, temporal.ruta()).map_err(|e| {
        format!(
            "no se puede lanzar el editor «{editor}»: {e}.\n\
             Configúralo en bdc.ini, sección [principal], propiedad «editor»."
        )
    })?;
    let inmediata = inicio.elapsed() < Duration::from_secs(2);
    if !estado.success() {
        // 127 (sh) y 9009 (cmd): el intérprete no encuentra el programa.
        if matches!(estado.code(), Some(127) | Some(9009)) {
            return Err(format!(
                "no se encuentra el editor «{editor}».\n\
                 Configúralo en bdc.ini, sección [principal], propiedad «editor»."
            ));
        }
        return Err(format!("el editor «{editor}» ha terminado con error ({estado}). No se guarda nada."));
    }

    let bytes = fs::read(temporal.ruta()).map_err(|e| format!("no se puede leer el fichero temporal: {e}."))?;
    let texto = limpiar(&decodificar(&bytes));
    Ok(Edicion { sin_cambios: texto == limpiar(inicial), inmediata, texto })
}

/// Lanza el editor como proceso hijo y espera a que termine. Se invoca a
/// través del intérprete de comandos para que la propiedad `editor` pueda
/// llevar argumentos (`code --wait`) y rutas con espacios entre comillas.
fn lanzar(editor: &str, fichero: &Path) -> io::Result<ExitStatus> {
    comando(editor, fichero).status()
}

#[cfg(windows)]
fn comando(editor: &str, fichero: &Path) -> Command {
    use std::os::windows::process::CommandExt;
    let mut c = Command::new("cmd");
    // Con /S, cmd quita solo la primera y la última comilla de la línea.
    c.raw_arg(format!("/D /S /C \"{editor} \"{}\"\"", fichero.display()));
    c
}

#[cfg(not(windows))]
fn comando(editor: &str, fichero: &Path) -> Command {
    let mut c = Command::new("sh");
    c.arg("-c").arg(format!("{editor} \"$1\"")).arg("bdc").arg(fichero);
    c
}

/// Fichero temporal que se borra al salir de ámbito, también si hay error.
struct FicheroTemporal {
    ruta: PathBuf,
}

impl FicheroTemporal {
    fn crear(contenido: &str) -> io::Result<Self> {
        let dir = std::env::temp_dir();
        let marca = SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_nanos());
        for intento in 0..100 {
            // Extensión .md para que el editor resalte el Markdown.
            let ruta = dir.join(format!("bdc-{}-{marca}-{intento}.md", std::process::id()));
            match OpenOptions::new().write(true).create_new(true).open(&ruta) {
                Ok(mut f) => {
                    let temporal = FicheroTemporal { ruta };
                    f.write_all(contenido.as_bytes())?;
                    return Ok(temporal);
                }
                Err(e) if e.kind() == io::ErrorKind::AlreadyExists => continue,
                Err(e) => return Err(e),
            }
        }
        Err(io::Error::new(io::ErrorKind::AlreadyExists, "no se encuentra un nombre libre"))
    }

    fn ruta(&self) -> &Path {
        &self.ruta
    }
}

impl Drop for FicheroTemporal {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.ruta);
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;

    #[test]
    fn edita_con_un_comando_y_borra_el_temporal() {
        // «editor» que añade una línea al fichero recibido y guarda su ruta.
        let rastro = std::env::temp_dir().join(format!("bdc-test-{}", std::process::id()));
        let editor = format!("f() {{ printf '\\r\\nañadido\\r\\n' >> \"$1\"; echo \"$1\" > '{}'; }}; f", rastro.display());
        let r = editar(&editor, "Inicial").unwrap();
        assert_eq!(r.texto, "Inicial\nañadido");
        assert!(!r.sin_cambios);

        let usado = fs::read_to_string(&rastro).unwrap();
        fs::remove_file(&rastro).unwrap();
        assert!(usado.trim().ends_with(".md"));
        assert!(!Path::new(usado.trim()).exists(), "el temporal debe borrarse");
    }

    #[test]
    fn detecta_editor_que_no_espera_y_errores() {
        let r = editar("true", "hola\n").unwrap();
        assert!(r.sin_cambios && r.inmediata);
        assert_eq!(r.texto, "hola");
        assert!(editar("false", "x").is_err());
    }
}
