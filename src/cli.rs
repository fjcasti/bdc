//! Análisis de la línea de comandos.
//!
//! Las opciones se aceptan con guion (`-a`, `--a`) y también con barra
//! (`/a`), la notación clásica de Windows, sin distinguir mayúsculas.

use std::fmt;
use std::path::PathBuf;

#[derive(Debug, PartialEq, Eq)]
pub enum Accion {
    Ayuda,
    /// Añadir una nota, con el texto inicial (puede estar vacío).
    Anadir(String),
    /// Buscar los argumentos indicados.
    Buscar(Vec<String>),
    Editar(i64),
    /// `-bd FICHERO` sin más opciones: guardar la base de datos en bdc.ini.
    FijarBd,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Orden {
    pub accion: Accion,
    /// Base de datos indicada con `-bd`, si la hay.
    pub bd: Option<PathBuf>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum ErrorCli {
    SinTextoBusqueda,
    SinId,
    IdNoValido(String),
    SobranArgumentos(String),
    SinFicheroBd,
    BdRepetida,
    OpcionDesconocida(String),
    OpcionesIncompatibles(&'static str, &'static str),
}

impl fmt::Display for ErrorCli {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::SinTextoBusqueda => write!(f, "falta el texto a buscar."),
            Self::SinId => write!(f, "la opción -e necesita el ID de la nota a editar."),
            Self::IdNoValido(id) => write!(f, "«{id}» no es un ID válido (debe ser un número entero positivo)."),
            Self::SobranArgumentos(resto) => write!(f, "sobran argumentos tras el ID: «{resto}»."),
            Self::SinFicheroBd => write!(f, "la opción -bd necesita el fichero de base de datos."),
            Self::BdRepetida => write!(f, "la opción -bd solo puede indicarse una vez."),
            Self::OpcionDesconocida(op) => write!(f, "opción desconocida «{op}»."),
            Self::OpcionesIncompatibles(a, b) => write!(f, "las opciones {a} y {b} no se pueden combinar."),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Opcion {
    Ayuda,
    Anadir,
    Buscar,
    Editar,
    Bd,
}

impl Opcion {
    fn nombre(self) -> &'static str {
        match self {
            Self::Ayuda => "-h",
            Self::Anadir => "-a",
            Self::Buscar => "-b",
            Self::Editar => "-e",
            Self::Bd => "-bd",
        }
    }
}

/// Reconoce si un argumento es una opción.
///
/// Un argumento con barra que no sea una opción conocida se trata como texto
/// (en Linux puede ser una ruta, p. ej. `/home`). Con guion, en cambio, es un
/// error, salvo que sea un número negativo o un guion solo.
fn reconocer(arg: &str) -> Result<Option<Opcion>, ErrorCli> {
    let (cuerpo, con_guion) = if let Some(r) = arg.strip_prefix("--") {
        (r, true)
    } else if let Some(r) = arg.strip_prefix('-') {
        (r, true)
    } else if let Some(r) = arg.strip_prefix('/') {
        (r, false)
    } else {
        return Ok(None);
    };
    if cuerpo.is_empty() {
        return Ok(None);
    }
    let opcion = match cuerpo.to_lowercase().as_str() {
        "?" | "h" | "help" | "ayuda" => Opcion::Ayuda,
        "a" => Opcion::Anadir,
        "b" => Opcion::Buscar,
        "e" => Opcion::Editar,
        "bd" => Opcion::Bd,
        _ if con_guion && !cuerpo.starts_with(|c: char| c.is_ascii_digit()) => {
            return Err(ErrorCli::OpcionDesconocida(arg.to_owned()));
        }
        _ => return Ok(None),
    };
    Ok(Some(opcion))
}

pub fn analizar(args: &[String]) -> Result<Orden, ErrorCli> {
    let ayuda = Orden { accion: Accion::Ayuda, bd: None };

    // Sin argumentos, o con -?/-h en cualquier posición: solo la ayuda.
    if args.is_empty() || args.iter().any(|a| reconocer(a) == Ok(Some(Opcion::Ayuda))) {
        return Ok(ayuda);
    }

    let mut accion: Option<Opcion> = None;
    let mut bd: Option<PathBuf> = None;
    let mut textos: Vec<String> = Vec::new();

    let mut i = 0;
    while i < args.len() {
        let arg = &args[i];
        match reconocer(arg)? {
            Some(Opcion::Bd) => {
                if bd.is_some() {
                    return Err(ErrorCli::BdRepetida);
                }
                match args.get(i + 1) {
                    Some(sig) if !sig.trim().is_empty() && reconocer(sig) == Ok(None) => {
                        bd = Some(PathBuf::from(sig));
                        i += 1;
                    }
                    _ => return Err(ErrorCli::SinFicheroBd),
                }
            }
            Some(op) => {
                if let Some(anterior) = accion {
                    return Err(ErrorCli::OpcionesIncompatibles(anterior.nombre(), op.nombre()));
                }
                accion = Some(op);
            }
            None if arg.trim().is_empty() => {}
            None => textos.push(arg.clone()),
        }
        i += 1;
    }

    let accion = match accion {
        None if textos.is_empty() => {
            if bd.is_some() {
                Accion::FijarBd
            } else {
                return Ok(ayuda);
            }
        }
        None | Some(Opcion::Buscar) => {
            if textos.is_empty() {
                return Err(ErrorCli::SinTextoBusqueda);
            }
            Accion::Buscar(textos)
        }
        Some(Opcion::Anadir) => Accion::Anadir(textos.join(" ")),
        Some(Opcion::Editar) => match textos.as_slice() {
            [] => return Err(ErrorCli::SinId),
            [id] => match id.trim().parse::<i64>() {
                Ok(n) if n > 0 => Accion::Editar(n),
                _ => return Err(ErrorCli::IdNoValido(id.clone())),
            },
            [_, resto @ ..] => return Err(ErrorCli::SobranArgumentos(resto.join(" "))),
        },
        Some(Opcion::Ayuda) | Some(Opcion::Bd) => unreachable!("tratadas antes"),
    };

    Ok(Orden { accion, bd })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn analiza(args: &[&str]) -> Result<Orden, ErrorCli> {
        analizar(&args.iter().map(|s| s.to_string()).collect::<Vec<_>>())
    }

    fn accion(args: &[&str]) -> Accion {
        analiza(args).unwrap().accion
    }

    #[test]
    fn ayuda() {
        assert_eq!(accion(&[]), Accion::Ayuda);
        assert_eq!(accion(&["-?"]), Accion::Ayuda);
        assert_eq!(accion(&["/H"]), Accion::Ayuda);
        // -h anula el resto, incluso si hay errores en otros argumentos.
        assert_eq!(accion(&["-a", "texto", "-h"]), Accion::Ayuda);
        assert_eq!(accion(&["-bd", "-?"]), Accion::Ayuda);
        assert_eq!(accion(&["-x", "-h"]), Accion::Ayuda);
    }

    #[test]
    fn busqueda() {
        assert_eq!(accion(&["rust", "sqlite"]), Accion::Buscar(vec!["rust".into(), "sqlite".into()]));
        assert_eq!(accion(&["-b", "rust"]), Accion::Buscar(vec!["rust".into()]));
        assert_eq!(accion(&["/B", "rust"]), Accion::Buscar(vec!["rust".into()]));
        assert_eq!(accion(&["/home"]), Accion::Buscar(vec!["/home".into()]));
        assert_eq!(analiza(&["-b"]), Err(ErrorCli::SinTextoBusqueda));
        assert_eq!(analiza(&["-b", " "]), Err(ErrorCli::SinTextoBusqueda));
    }

    #[test]
    fn anadir() {
        assert_eq!(accion(&["-a"]), Accion::Anadir(String::new()));
        assert_eq!(accion(&["-a", "una", "nota"]), Accion::Anadir("una nota".into()));
        assert_eq!(accion(&["--a", "x"]), Accion::Anadir("x".into()));
    }

    #[test]
    fn editar() {
        assert_eq!(accion(&["-e", "12"]), Accion::Editar(12));
        assert_eq!(analiza(&["-e"]), Err(ErrorCli::SinId));
        assert_eq!(analiza(&["-e", "abc"]), Err(ErrorCli::IdNoValido("abc".into())));
        assert_eq!(analiza(&["-e", "0"]), Err(ErrorCli::IdNoValido("0".into())));
        assert_eq!(analiza(&["-e", "-3"]), Err(ErrorCli::IdNoValido("-3".into())));
        assert_eq!(analiza(&["-e", "1", "2"]), Err(ErrorCli::SobranArgumentos("2".into())));
    }

    #[test]
    fn base_de_datos() {
        let o = analiza(&["-bd", "otra.db"]).unwrap();
        assert_eq!(o, Orden { accion: Accion::FijarBd, bd: Some("otra.db".into()) });

        let o = analiza(&["-bd", "otra.db", "-b", "rust"]).unwrap();
        assert_eq!(o.accion, Accion::Buscar(vec!["rust".into()]));
        assert_eq!(o.bd, Some("otra.db".into()));

        // El orden no importa y las rutas absolutas de Linux no son opciones.
        let o = analiza(&["-e", "3", "/BD", "/tmp/a.db"]).unwrap();
        assert_eq!((o.accion, o.bd), (Accion::Editar(3), Some("/tmp/a.db".into())));

        assert_eq!(analiza(&["-bd"]), Err(ErrorCli::SinFicheroBd));
        assert_eq!(analiza(&["-bd", "-a"]), Err(ErrorCli::SinFicheroBd));
        assert_eq!(analiza(&["-bd", "a", "-bd", "b"]), Err(ErrorCli::BdRepetida));
    }

    #[test]
    fn errores() {
        assert_eq!(analiza(&["-x"]), Err(ErrorCli::OpcionDesconocida("-x".into())));
        assert_eq!(analiza(&["-a", "-b", "x"]), Err(ErrorCli::OpcionesIncompatibles("-a", "-b")));
    }
}
