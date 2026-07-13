use arboard::Clipboard; 
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
    terminal::{disable_raw_mode, enable_raw_mode},
};
use rusqlite::{Connection, Result as SqlResult};
use std::io::{self, Write};
use std::path::PathBuf;
use termimad::MadSkin;
use unicode_normalization::UnicodeNormalization;

fn normalizar(s: &str) -> String {
    s.nfd()
        .filter(|c| !('\u{0300}'..='\u{036F}').contains(c))
        .collect::<String>()
        .to_lowercase()
}

fn init_db(conn: &Connection) -> SqlResult<()> {
    conn.execute_batch("
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
    ")
}

fn guardar(conn: &Connection, texto: &str, tags: &[String]) -> SqlResult<()> {
    conn.execute("INSERT INTO textos (contenido) VALUES (?1)", [texto])?;
    let texto_id = conn.last_insert_rowid();

    for tag in tags {
        let tag = normalizar(tag);
        conn.execute("INSERT OR IGNORE INTO etiquetas (nombre) VALUES (?1)", [&tag])?;
        let etiqueta_id: i64 = conn.query_row(
            "SELECT id FROM etiquetas WHERE nombre = ?1",
            [&tag],
            |row| row.get(0),
        )?;
        conn.execute(
            "INSERT INTO texto_etiquetas (texto_id, etiqueta_id) VALUES (?1, ?2)",
            [texto_id, etiqueta_id],
        )?;
    }

    Ok(())
}

fn buscar(conn: &Connection, termino: &str) -> SqlResult<()> {
    let skin = MadSkin::default();
    let patron = format!("%{}%", normalizar(termino));

    let mut stmt = conn.prepare("
        SELECT DISTINCT t.id, t.contenido, t.creado_en
        FROM textos t
        LEFT JOIN texto_etiquetas te ON te.texto_id = t.id
        LEFT JOIN etiquetas e ON e.id = te.etiqueta_id
        WHERE t.contenido LIKE ?1
           OR e.nombre    LIKE ?1
        ORDER BY t.id DESC
    ")?;

    let filas: Vec<(i64, String, String)> = stmt
        .query_map([&patron], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))?
        .filter_map(|r| r.ok())
        .collect();

    if filas.is_empty() {
        println!("Sin resultados para \"{}\".", termino);
        return Ok(());
    }

    println!("{} resultado(s) para \"{}\":", filas.len(), termino);

    for (id, contenido, creado_en) in &filas {
        let mut tag_stmt = conn.prepare("
            SELECT e.nombre FROM etiquetas e
            JOIN texto_etiquetas te ON te.etiqueta_id = e.id
            WHERE te.texto_id = ?1
        ")?;
        let tags: Vec<String> = tag_stmt
            .query_map([id], |row| row.get(0))?
            .filter_map(|r| r.ok())
            .collect();

        println!("{}", "─".repeat(60));
        println!("ID: {}  |  {}", id, creado_en);
        if !tags.is_empty() {
            println!("Etiquetas: {}", tags.join(", "));
        }
        println!();
        skin.print_text(contenido);
    }
    println!("{}", "─".repeat(60));

    Ok(())
}

fn mostrar_ayuda(db_path: &PathBuf) {
    println!("BDC 1.0 - Base de Conocimiento");
    println!();
    println!("  Uso: BDC [opcion] [texto]");
    println!();
    println!("  /?         Muestra esta ayuda");
    println!("  /a [XXX]   Añadir el texto XXX al fichero de datos");
    println!("  /b XXX     Busca el texto XXX en el fichero de datos");
    println!();
    println!("  Fichero de datos: {}", db_path.display());
}

fn pedir_etiquetas() -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let mut stdout = io::stdout();
    println!("{}", "-".repeat(60));
    println!("Introduce etiquetas. Enter = añadir etiqueta. Enter vacío = terminar.");
    println!("{}", "-".repeat(60));
    stdout.flush()?;

    let mut tags: Vec<String> = Vec::new();

    loop {
        print!("> ");
        stdout.flush()?;

        match read_line(&mut stdout)? {
            Some(tag) if !tag.is_empty() => tags.push(tag),
            _ => break,
        }
    }

    Ok(tags)
}

fn read_line(stdout: &mut io::Stdout) -> Result<Option<String>, Box<dyn std::error::Error>> {
    let mut buf = String::new();

    loop {
        if let Event::Key(key) = event::read()? {
            if key.kind != KeyEventKind::Press {
                continue;
            }
            match (key.modifiers, key.code) {
                (KeyModifiers::NONE, KeyCode::Esc) => return Ok(None),

                (KeyModifiers::NONE, KeyCode::Enter) => {
                    println!();
                    return Ok(Some(buf));
                }

                (KeyModifiers::CONTROL, KeyCode::Char('v')) => {
                    disable_raw_mode()?;
                    if let Ok(mut clipboard) = Clipboard::new() {
                        if let Ok(content) = clipboard.get_text() {
                            print!("{}", content);
                            stdout.flush()?;
                            buf.push_str(&content);
                        }
                    }
                    enable_raw_mode()?;
                }

                (KeyModifiers::NONE | KeyModifiers::SHIFT, KeyCode::Char(c)) => {
                    print!("{c}");
                    stdout.flush()?;
                    buf.push(c);
                }

                (KeyModifiers::NONE, KeyCode::Backspace) => {
                    if buf.pop().is_some() {
                        print!("\x08 \x08");
                        stdout.flush()?;
                    }
                }

                _ => {}
            }
        }
    }
}

fn modo_anadir(conn: &Connection, inicial: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mut stdout = io::stdout();

    println!("Escribe texto. Enter = nueva línea. Ctrl+V = pegar. ESC = aceptar.");
    println!("{}", "-".repeat(60));

    // Mostrar y pre-rellenar el texto inicial si lo hay
    if !inicial.is_empty() {
        print!("{}", inicial);
    }
    stdout.flush()?;

    let mut text = inicial.to_string();
    enable_raw_mode()?;

    loop {
        if let Event::Key(key) = event::read()? {
            if key.kind != KeyEventKind::Press {
                continue;
            }
            match (key.modifiers, key.code) {
                (KeyModifiers::NONE, KeyCode::Esc) => break,

                (KeyModifiers::CONTROL, KeyCode::Char('v')) => {
                    disable_raw_mode()?;
                    if let Ok(mut clipboard) = Clipboard::new() {
                        if let Ok(content) = clipboard.get_text() {
                            print!("{}", content);
                            stdout.flush()?;
                            text.push_str(&content);
                        }
                    }
                    enable_raw_mode()?;
                }

                (KeyModifiers::NONE | KeyModifiers::SHIFT, KeyCode::Char(c)) => {
                    print!("{c}");
                    stdout.flush()?;
                    text.push(c);
                }

                (KeyModifiers::NONE, KeyCode::Enter) => {
                    println!();
                    text.push('\n');
                }

                (KeyModifiers::NONE, KeyCode::Backspace) => {
                    if text.pop().is_some() {
                        print!("\x08 \x08");
                        stdout.flush()?;
                    }
                }

                _ => {}
            }
        }
    }

    let tags = pedir_etiquetas()?;
    disable_raw_mode()?;

    if !text.is_empty() {
        guardar(conn, &text, &tags)?;
        println!("Guardado correctamente.");
    } else {
        println!("Texto vacío, nada guardado.");
    }

    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let db_path = std::env::current_exe()?
        .parent()
        .expect("ejecutable sin directorio padre")
        .join("bdc.db");

    let conn = Connection::open(&db_path)?;
    init_db(&conn)?;

    let args: Vec<String> = std::env::args().skip(1).collect();

    match args.as_slice() {
        [] | [_] if args.first().map(|s| s.as_str()) == Some("/?") => {
            mostrar_ayuda(&db_path);
        }
        [] => {
            mostrar_ayuda(&db_path);
        }
        [opcion, resto @ ..] if opcion == "/a" => {
            let texto = resto.join(" ");
            modo_anadir(&conn, &texto)?;
        }
        [opcion, resto @ ..] if opcion == "/b" => {
            let termino = resto.join(" ");
            if termino.is_empty() {
                eprintln!("Uso: BDC /b <texto>");
            } else {
                buscar(&conn, &termino)?;
            }
        }
        _ => {
            eprintln!("Opción no reconocida. Use /? para ver la ayuda.");
        }
    }

    Ok(())
}
