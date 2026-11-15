//! Configuración: fichero bdc.ini junto al ejecutable.
//!
//! ```ini
//! [principal]
//! editor=code --wait
//!
//! [base_de_datos]
//! ruta=/home/usuario/bdc.db
//! ```

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

pub const NOMBRE_INI: &str = "bdc.ini";
pub const NOMBRE_BD: &str = "bdc.db";

const SECCION_PRINCIPAL: &str = "principal";
const CLAVE_EDITOR: &str = "editor";
const SECCION_BD: &str = "base_de_datos";
const CLAVE_RUTA: &str = "ruta";

pub struct Config {
    /// Carpeta del ejecutable: allí están bdc.ini y la base de datos por defecto.
    pub dir: PathBuf,
    pub editor: Option<String>,
    pub ruta_bd: Option<PathBuf>,
    /// Avisos al leer la configuración, para mostrarlos al usuario.
    pub avisos: Vec<String>,
}

impl Config {
    pub fn cargar() -> Config {
        Self::cargar_desde(&dir_ejecutable())
    }

    pub fn cargar_desde(dir: &Path) -> Config {
        let mut cfg = Config { dir: dir.to_path_buf(), editor: None, ruta_bd: None, avisos: Vec::new() };
        let ruta_ini = cfg.ruta_ini();
        let ini = match fs::read(&ruta_ini) {
            Ok(bytes) => Ini::parsear(&crate::texto::decodificar(&bytes)),
            Err(e) if e.kind() == io::ErrorKind::NotFound => return cfg,
            Err(e) => {
                cfg.avisos.push(format!("no se puede leer {}: {e}", ruta_ini.display()));
                return cfg;
            }
        };
        // El editor se pasa tal cual al intérprete de comandos: sus comillas
        // sirven para rutas con espacios y no se quitan.
        cfg.editor = ini.obtener(SECCION_PRINCIPAL, CLAVE_EDITOR);
        // Una ruta relativa en el INI se toma respecto a la carpeta del ejecutable.
        cfg.ruta_bd = ini
            .obtener(SECCION_BD, CLAVE_RUTA)
            .map(|r| sin_comillas(&r).trim().to_owned())
            .filter(|r| !r.is_empty())
            .map(|r| dir.join(r));
        cfg
    }

    pub fn ruta_ini(&self) -> PathBuf {
        self.dir.join(NOMBRE_INI)
    }

    pub fn ruta_bd_defecto(&self) -> PathBuf {
        self.dir.join(NOMBRE_BD)
    }

    /// Base de datos a usar cuando no se indica `-bd`: la de bdc.ini si existe;
    /// si no, bdc.db junto al ejecutable. Devuelve también un aviso cuando la
    /// configurada en el INI no existe.
    pub fn resolver_bd(&self) -> (PathBuf, Option<String>) {
        match &self.ruta_bd {
            Some(ruta) if ruta.exists() => (ruta.clone(), None),
            Some(ruta) => {
                let defecto = self.ruta_bd_defecto();
                let aviso = format!(
                    "la base de datos «{}» configurada en {NOMBRE_INI} no existe; se usa «{}».",
                    ruta.display(),
                    defecto.display()
                );
                (defecto, Some(aviso))
            }
            None => (self.ruta_bd_defecto(), None),
        }
    }

    /// Guarda la ruta de la base de datos en bdc.ini, conservando el resto del fichero.
    pub fn guardar_ruta_bd(&self, ruta: &Path) -> io::Result<()> {
        let ruta_ini = self.ruta_ini();
        let mut ini = match fs::read(&ruta_ini) {
            Ok(bytes) => Ini::parsear(&crate::texto::decodificar(&bytes)),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ini::parsear(""),
            Err(e) => return Err(e),
        };
        ini.establecer(SECCION_BD, CLAVE_RUTA, &ruta.to_string_lossy());
        fs::write(&ruta_ini, ini.a_texto())
    }
}

fn dir_ejecutable() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(Path::to_path_buf))
        .or_else(|| std::env::current_dir().ok())
        .unwrap_or_else(|| PathBuf::from("."))
}

/// Fichero INI mínimo que conserva comentarios y formato al modificarlo.
pub struct Ini {
    lineas: Vec<String>,
    crlf: bool,
}

fn cabecera(linea: &str) -> Option<&str> {
    let l = linea.trim();
    l.strip_prefix('[')?.strip_suffix(']').map(str::trim)
}

fn clave_valor(linea: &str) -> Option<(&str, &str)> {
    let l = linea.trim();
    if l.starts_with(';') || l.starts_with('#') {
        return None;
    }
    let (clave, valor) = l.split_once('=')?;
    Some((clave.trim(), valor.trim()))
}

fn sin_comillas(valor: &str) -> &str {
    valor
        .strip_prefix('"')
        .and_then(|v| v.strip_suffix('"'))
        .unwrap_or(valor)
}

impl Ini {
    pub fn parsear(texto: &str) -> Ini {
        Ini {
            lineas: texto.lines().map(str::to_owned).collect(),
            crlf: texto.contains("\r\n"),
        }
    }

    /// Rango de líneas (cabecera excluida) de una sección, si existe.
    fn seccion(&self, nombre: &str) -> Option<(usize, usize)> {
        let inicio = self
            .lineas
            .iter()
            .position(|l| cabecera(l).is_some_and(|c| c.eq_ignore_ascii_case(nombre)))?
            + 1;
        let fin = self.lineas[inicio..]
            .iter()
            .position(|l| cabecera(l).is_some())
            .map_or(self.lineas.len(), |p| inicio + p);
        Some((inicio, fin))
    }

    /// Valor de una clave; `None` si no existe o está vacío.
    pub fn obtener(&self, seccion: &str, clave: &str) -> Option<String> {
        let (inicio, fin) = self.seccion(seccion)?;
        self.lineas[inicio..fin]
            .iter()
            .rev()
            .filter_map(|l| clave_valor(l))
            .find(|(c, _)| c.eq_ignore_ascii_case(clave))
            .map(|(_, v)| v.to_owned())
            .filter(|v| !v.is_empty())
    }

    pub fn establecer(&mut self, seccion: &str, clave: &str, valor: &str) {
        let nueva = format!("{clave}={valor}");
        let Some((inicio, fin)) = self.seccion(seccion) else {
            if self.lineas.last().is_some_and(|l| !l.trim().is_empty()) {
                self.lineas.push(String::new());
            }
            self.lineas.push(format!("[{seccion}]"));
            self.lineas.push(nueva);
            return;
        };
        let existente = (inicio..fin)
            .rev()
            .find(|&i| clave_valor(&self.lineas[i]).is_some_and(|(c, _)| c.eq_ignore_ascii_case(clave)));
        match existente {
            Some(i) => self.lineas[i] = nueva,
            None => {
                // Tras la última línea con contenido de la sección.
                let pos = (inicio..fin)
                    .rev()
                    .find(|&i| !self.lineas[i].trim().is_empty())
                    .map_or(inicio, |i| i + 1);
                self.lineas.insert(pos, nueva);
            }
        }
    }

    pub fn a_texto(&self) -> String {
        let salto = if self.crlf { "\r\n" } else { "\n" };
        let mut s = self.lineas.join(salto);
        s.push_str(salto);
        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const EJEMPLO: &str = "; Ejemplo\n[principal]\neditor=code --wait\n\n[Base_De_Datos]\n;ruta=C:\\x.db\nruta = \"/home/u/bdc.db\"\n";

    #[test]
    fn lee_valores() {
        let ini = Ini::parsear(EJEMPLO);
        assert_eq!(ini.obtener("principal", "editor").as_deref(), Some("code --wait"));
        assert_eq!(ini.obtener("base_de_datos", "ruta").as_deref(), Some("\"/home/u/bdc.db\""));
        assert_eq!(ini.obtener("principal", "ruta"), None);
        assert_eq!(ini.obtener("otra", "editor"), None);
        assert_eq!(Ini::parsear("[principal]\neditor=\n").obtener("principal", "editor"), None);
    }

    #[test]
    fn config_quita_comillas_solo_en_la_ruta() {
        let dir = std::env::temp_dir().join(format!("bdc-cfg-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join(NOMBRE_INI), "[principal]\neditor=\"/x y/ed\" -w\n[base_de_datos]\nruta=\"datos.db\"\n").unwrap();
        let cfg = Config::cargar_desde(&dir);
        fs::remove_dir_all(&dir).unwrap();
        assert_eq!(cfg.editor.as_deref(), Some("\"/x y/ed\" -w"));
        assert_eq!(cfg.ruta_bd, Some(dir.join("datos.db")));
    }

    #[test]
    fn modifica_conservando_comentarios() {
        let mut ini = Ini::parsear(EJEMPLO);
        ini.establecer("base_de_datos", "ruta", "/otra/bd.db");
        assert_eq!(
            ini.a_texto(),
            "; Ejemplo\n[principal]\neditor=code --wait\n\n[Base_De_Datos]\n;ruta=C:\\x.db\nruta=/otra/bd.db\n"
        );
    }

    #[test]
    fn crea_clave_y_seccion() {
        let mut ini = Ini::parsear("[base_de_datos]\n; sin ruta\n\n[principal]\neditor=vi\r\n");
        ini.establecer("base_de_datos", "ruta", "a.db");
        assert_eq!(ini.a_texto(), "[base_de_datos]\r\n; sin ruta\r\nruta=a.db\r\n\r\n[principal]\r\neditor=vi\r\n");

        let mut ini = Ini::parsear("[principal]\neditor=vi\n");
        ini.establecer("base_de_datos", "ruta", "a.db");
        assert_eq!(ini.a_texto(), "[principal]\neditor=vi\n\n[base_de_datos]\nruta=a.db\n");

        let mut ini = Ini::parsear("");
        ini.establecer("base_de_datos", "ruta", "a.db");
        assert_eq!(ini.a_texto(), "[base_de_datos]\nruta=a.db\n");
    }
}
