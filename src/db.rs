//! Acceso a la base de datos SQLite.
//!
//! El esquema es el de `bdc.sql`, compatible con las bases de datos de la
//! versión 1.x.

use std::path::Path;

use rusqlite::functions::FunctionFlags;
use rusqlite::{Connection, OpenFlags, OptionalExtension, params, params_from_iter};

use crate::texto::{normalizar, normalizar_etiqueta};

const ESQUEMA: &str = "
    CREATE TABLE IF NOT EXISTS textos (
        id         INTEGER PRIMARY KEY AUTOINCREMENT,
        contenido  TEXT    NOT NULL,
        creado_en  TEXT    NOT NULL DEFAULT (datetime('now'))
    );
    CREATE TABLE IF NOT EXISTS etiquetas (
        id     INTEGER PRIMARY KEY AUTOINCREMENT,
        nombre TEXT    NOT NULL UNIQUE
    );
    CREATE TABLE IF NOT EXISTS texto_etiquetas (
        texto_id    INTEGER NOT NULL REFERENCES textos(id),
        etiqueta_id INTEGER NOT NULL REFERENCES etiquetas(id),
        PRIMARY KEY (texto_id, etiqueta_id)
    );
";

/// Tablas y columnas que necesita la aplicación.
const TABLAS: [(&str, &[&str]); 3] = [
    ("textos", &["id", "contenido", "creado_en"]),
    ("etiquetas", &["id", "nombre"]),
    ("texto_etiquetas", &["texto_id", "etiqueta_id"]),
];

pub type Resultado<T> = rusqlite::Result<T>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Nota {
    pub id: i64,
    pub contenido: String,
    /// Fecha de creación en hora local.
    pub creado_en: String,
    pub etiquetas: Vec<String>,
}

pub struct Bd {
    conn: Connection,
}

/// Comprueba que un fichero existe y es una base de datos válida para BDC.
/// El error es una frase que completa «La base de datos «X» …».
pub fn validar(ruta: &Path) -> Result<(), String> {
    if !ruta.exists() {
        return Err("no existe.".into());
    }
    if !ruta.is_file() {
        return Err("no es un fichero.".into());
    }
    let conn = Connection::open_with_flags(ruta, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .map_err(|e| format!("no se puede abrir: {e}."))?;
    comprobar_esquema(&conn)
}

fn comprobar_esquema(conn: &Connection) -> Result<(), String> {
    for (tabla, columnas) in TABLAS {
        let existentes: Vec<String> = conn
            .prepare("SELECT name FROM pragma_table_info(?1)")
            .and_then(|mut st| st.query_map([tabla], |f| f.get(0))?.collect())
            .map_err(|e| match e.sqlite_error_code() {
                Some(rusqlite::ErrorCode::NotADatabase) => "no es una base de datos SQLite.".to_owned(),
                _ => format!("no se puede leer: {e}."),
            })?;
        if existentes.is_empty() {
            return Err(format!("no es una base de datos de BDC (falta la tabla «{tabla}»)."));
        }
        if let Some(falta) = columnas.iter().find(|c| !existentes.iter().any(|e| e.eq_ignore_ascii_case(c))) {
            return Err(format!("no es una base de datos de BDC (a la tabla «{tabla}» le falta la columna «{falta}»)."));
        }
    }
    Ok(())
}

impl Bd {
    /// Abre una base de datos que ya debe existir y ser válida (opción `-bd`).
    pub fn abrir_existente(ruta: &Path) -> Result<Bd, String> {
        validar(ruta)?;
        let conn = Connection::open_with_flags(ruta, OpenFlags::SQLITE_OPEN_READ_WRITE)
            .map_err(|e| format!("no se puede abrir: {e}."))?;
        Self::preparar(conn).map_err(|e| format!("no se puede abrir: {e}."))
    }

    /// Abre la base de datos y la crea si no existe. Devuelve si se ha creado.
    pub fn abrir_o_crear(ruta: &Path) -> Result<(Bd, bool), String> {
        let nueva = !ruta.exists();
        let conn = Connection::open(ruta).map_err(|e| format!("no se puede abrir ni crear: {e}."))?;
        conn.execute_batch(ESQUEMA).map_err(|e| match e.sqlite_error_code() {
            Some(rusqlite::ErrorCode::NotADatabase) => "no es una base de datos SQLite.".to_owned(),
            _ => format!("no se puede preparar: {e}."),
        })?;
        comprobar_esquema(&conn)?;
        let bd = Self::preparar(conn).map_err(|e| format!("no se puede abrir: {e}."))?;
        Ok((bd, nueva))
    }

    #[cfg(test)]
    pub fn en_memoria() -> Bd {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(ESQUEMA).unwrap();
        Self::preparar(conn).unwrap()
    }

    fn preparar(conn: Connection) -> Resultado<Bd> {
        conn.execute_batch("PRAGMA foreign_keys = ON;")?;
        // normalizar(texto) en SQL, para buscar sin distinguir mayúsculas ni acentos.
        conn.create_scalar_function(
            "normalizar",
            1,
            FunctionFlags::SQLITE_UTF8 | FunctionFlags::SQLITE_DETERMINISTIC,
            |ctx| Ok(normalizar(&ctx.get::<Option<String>>(0)?.unwrap_or_default())),
        )?;
        Ok(Bd { conn })
    }

    pub fn anadir_nota(&self, contenido: &str) -> Resultado<i64> {
        self.conn.execute("INSERT INTO textos (contenido) VALUES (?1)", [contenido])?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn actualizar_nota(&self, id: i64, contenido: &str) -> Resultado<()> {
        self.conn.execute("UPDATE textos SET contenido = ?2 WHERE id = ?1", params![id, contenido])?;
        Ok(())
    }

    pub fn nota(&self, id: i64) -> Resultado<Option<Nota>> {
        let fila = self
            .conn
            .query_row(
                "SELECT id, contenido, COALESCE(datetime(creado_en, 'localtime'), creado_en)
                 FROM textos WHERE id = ?1",
                [id],
                |f| Ok((f.get(0)?, f.get(1)?, f.get(2)?)),
            )
            .optional()?;
        fila.map(|(id, contenido, creado_en)| self.completar(id, contenido, creado_en)).transpose()
    }

    fn completar(&self, id: i64, contenido: String, creado_en: Option<String>) -> Resultado<Nota> {
        Ok(Nota { id, contenido, creado_en: creado_en.unwrap_or_default(), etiquetas: self.etiquetas(id)? })
    }

    pub fn etiquetas(&self, id: i64) -> Resultado<Vec<String>> {
        let mut st = self.conn.prepare_cached(
            "SELECT e.nombre FROM etiquetas e
             JOIN texto_etiquetas te ON te.etiqueta_id = e.id
             WHERE te.texto_id = ?1 ORDER BY e.nombre",
        )?;
        st.query_map([id], |f| f.get(0))?.collect()
    }

    /// Sustituye las etiquetas de una nota. Las etiquetas se normalizan y se
    /// reutilizan si ya existen; las que quedan sin uso se eliminan.
    pub fn fijar_etiquetas(&self, id: i64, etiquetas: &[String]) -> Resultado<()> {
        let tx = self.conn.unchecked_transaction()?;
        tx.execute("DELETE FROM texto_etiquetas WHERE texto_id = ?1", [id])?;
        for etiqueta in etiquetas.iter().filter_map(|e| normalizar_etiqueta(e)) {
            tx.execute("INSERT OR IGNORE INTO etiquetas (nombre) VALUES (?1)", [&etiqueta])?;
            let etiqueta_id: i64 =
                tx.query_row("SELECT id FROM etiquetas WHERE nombre = ?1", [&etiqueta], |f| f.get(0))?;
            tx.execute(
                "INSERT OR IGNORE INTO texto_etiquetas (texto_id, etiqueta_id) VALUES (?1, ?2)",
                [id, etiqueta_id],
            )?;
        }
        borrar_etiquetas_sin_uso(&tx)?;
        tx.commit()
    }

    /// Borra una nota y sus etiquetas. Devuelve si existía.
    pub fn borrar_nota(&self, id: i64) -> Resultado<bool> {
        let tx = self.conn.unchecked_transaction()?;
        tx.execute("DELETE FROM texto_etiquetas WHERE texto_id = ?1", [id])?;
        let borradas = tx.execute("DELETE FROM textos WHERE id = ?1", [id])?;
        borrar_etiquetas_sin_uso(&tx)?;
        tx.commit()?;
        Ok(borradas > 0)
    }

    /// Notas que contienen los términos (ya normalizados) en el texto o en
    /// alguna etiqueta. Con `todos` deben aparecer todos; si no, basta uno.
    /// Las más recientes primero.
    pub fn buscar(&self, terminos: &[String], todos: bool) -> Resultado<Vec<Nota>> {
        if terminos.is_empty() {
            return Ok(Vec::new());
        }
        let condiciones: Vec<String> = (1..=terminos.len())
            .map(|i| {
                format!(
                    "(instr(normalizar(t.contenido), ?{i}) > 0 OR EXISTS (
                        SELECT 1 FROM texto_etiquetas te JOIN etiquetas e ON e.id = te.etiqueta_id
                        WHERE te.texto_id = t.id AND instr(normalizar(e.nombre), ?{i}) > 0))"
                )
            })
            .collect();
        let sql = format!(
            "SELECT t.id, t.contenido, COALESCE(datetime(t.creado_en, 'localtime'), t.creado_en)
             FROM textos t WHERE {} ORDER BY t.id DESC",
            condiciones.join(if todos { " AND " } else { " OR " })
        );
        let mut st = self.conn.prepare(&sql)?;
        let filas: Vec<(i64, String, Option<String>)> = st
            .query_map(params_from_iter(terminos), |f| Ok((f.get(0)?, f.get(1)?, f.get(2)?)))?
            .collect::<Resultado<_>>()?;
        filas.into_iter().map(|(id, c, f)| self.completar(id, c, f)).collect()
    }
}

fn borrar_etiquetas_sin_uso(conn: &Connection) -> Resultado<()> {
    conn.execute(
        "DELETE FROM etiquetas WHERE id NOT IN (SELECT etiqueta_id FROM texto_etiquetas)",
        [],
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(v: &[&str]) -> Vec<String> {
        v.iter().map(|x| x.to_string()).collect()
    }

    fn ids(notas: &[Nota]) -> Vec<i64> {
        notas.iter().map(|n| n.id).collect()
    }

    fn bd_con_datos() -> Bd {
        let bd = Bd::en_memoria();
        let a = bd.anadir_nota("Cómo crear una **Base de Datos** en SQLite").unwrap();
        bd.fijar_etiquetas(a, &s(&["SQLite", "Bases"])).unwrap();
        let b = bd.anadir_nota("Compilar con cargo build").unwrap();
        bd.fijar_etiquetas(b, &s(&["rust"])).unwrap();
        let c = bd.anadir_nota("Notas sueltas").unwrap();
        bd.fijar_etiquetas(c, &s(&["Canción"])).unwrap();
        bd
    }

    #[test]
    fn etiquetas_normalizadas_y_unicas() {
        let bd = Bd::en_memoria();
        let a = bd.anadir_nota("uno").unwrap();
        let b = bd.anadir_nota("dos").unwrap();
        bd.fijar_etiquetas(a, &s(&["Línux", "linux", "  "])).unwrap();
        bd.fijar_etiquetas(b, &s(&["LINUX"])).unwrap();
        assert_eq!(bd.etiquetas(a).unwrap(), s(&["linux"]));
        let total: i64 = bd.conn.query_row("SELECT count(*) FROM etiquetas", [], |f| f.get(0)).unwrap();
        assert_eq!(total, 1);
    }

    #[test]
    fn busqueda_sin_acentos_ni_mayusculas() {
        let bd = bd_con_datos();
        assert_eq!(ids(&bd.buscar(&s(&["como"]), true).unwrap()), vec![1]);
        assert_eq!(ids(&bd.buscar(&s(&["base de datos"]), true).unwrap()), vec![1]);
        // En etiquetas
        assert_eq!(ids(&bd.buscar(&s(&["rust"]), true).unwrap()), vec![2]);
        assert_eq!(ids(&bd.buscar(&s(&["cancion"]), true).unwrap()), vec![3]);
        // Y / O
        assert_eq!(ids(&bd.buscar(&s(&["sqlite", "crear"]), true).unwrap()), vec![1]);
        assert!(bd.buscar(&s(&["sqlite", "cargo"]), true).unwrap().is_empty());
        assert_eq!(ids(&bd.buscar(&s(&["sqlite", "cargo"]), false).unwrap()), vec![2, 1]);
        // Los comodines de LIKE no tienen efecto especial
        assert!(bd.buscar(&s(&["%"]), true).unwrap().is_empty());
    }

    #[test]
    fn editar_y_borrar() {
        let bd = bd_con_datos();
        bd.actualizar_nota(1, "nuevo").unwrap();
        bd.fijar_etiquetas(1, &s(&["sqlite"])).unwrap();
        let n = bd.nota(1).unwrap().unwrap();
        assert_eq!((n.contenido.as_str(), n.etiquetas), ("nuevo", s(&["sqlite"])));
        // «bases» se queda sin uso y desaparece
        let total: i64 = bd.conn.query_row("SELECT count(*) FROM etiquetas", [], |f| f.get(0)).unwrap();
        assert_eq!(total, 3);

        assert!(bd.borrar_nota(2).unwrap());
        assert!(!bd.borrar_nota(2).unwrap());
        assert_eq!(bd.nota(2).unwrap(), None);
    }
}
