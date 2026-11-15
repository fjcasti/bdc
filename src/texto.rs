//! Tratamiento de texto: normalización de etiquetas y términos de búsqueda,
//! y limpieza del texto que devuelve el editor.

use unicode_normalization::UnicodeNormalization;
use unicode_normalization::char::is_combining_mark;

/// Quita acentos y diacríticos y pasa a minúsculas: «Canción» → «cancion».
///
/// Se usa tanto para guardar etiquetas como para comparar en las búsquedas,
/// de modo que estas no distinguen mayúsculas ni acentos.
pub fn normalizar(s: &str) -> String {
    s.nfd()
        .filter(|c| !is_combining_mark(*c))
        .collect::<String>()
        .to_lowercase()
}

/// Normaliza una etiqueta introducida por el usuario. Devuelve `None` si queda vacía.
pub fn normalizar_etiqueta(s: &str) -> Option<String> {
    let n = normalizar(s.trim());
    (!n.is_empty()).then_some(n)
}

/// Convierte los argumentos de búsqueda en términos normalizados.
///
/// - Cada palabra suelta es un término.
/// - Un argumento que contiene espacios (porque se escribió entre comillas en
///   el terminal, p. ej. `bdc "base de datos"`) es una frase exacta.
/// - Si las comillas llegan literalmente dentro del argumento también se
///   respetan: `"base de datos" rust` → [«base de datos», «rust»].
pub fn terminos_busqueda(args: &[String]) -> Vec<String> {
    let mut terminos: Vec<String> = Vec::new();
    let mut anadir = |t: &str| {
        let t = normalizar(&t.split_whitespace().collect::<Vec<_>>().join(" "));
        if !t.is_empty() && !terminos.contains(&t) {
            terminos.push(t);
        }
    };

    for arg in args {
        if arg.contains('"') {
            for (i, trozo) in arg.split('"').enumerate() {
                if i % 2 == 1 {
                    anadir(trozo); // dentro de comillas: frase
                } else {
                    trozo.split_whitespace().for_each(&mut anadir);
                }
            }
        } else {
            anadir(arg);
        }
    }
    terminos
}

/// Decodifica el contenido del fichero que ha guardado el editor.
///
/// Admite UTF-8 (con o sin BOM) y UTF-16 con BOM. Si no es UTF-8 válido se
/// interpreta como Latin-1, que es lo que suelen producir editores antiguos de
/// Windows, para no perder los acentos.
pub fn decodificar(bytes: &[u8]) -> String {
    if let Some(resto) = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]) {
        return String::from_utf8_lossy(resto).into_owned();
    }
    let utf16 = |resto: &[u8], le: bool| {
        let unidades: Vec<u16> = resto
            .as_chunks::<2>()
            .0
            .iter()
            .map(|&p| if le { u16::from_le_bytes(p) } else { u16::from_be_bytes(p) })
            .collect();
        String::from_utf16_lossy(&unidades)
    };
    if let Some(resto) = bytes.strip_prefix(&[0xFF, 0xFE]) {
        return utf16(resto, true);
    }
    if let Some(resto) = bytes.strip_prefix(&[0xFE, 0xFF]) {
        return utf16(resto, false);
    }
    match std::str::from_utf8(bytes) {
        Ok(s) => s.to_owned(),
        Err(_) => bytes.iter().map(|&b| b as char).collect(),
    }
}

/// Unifica los saltos de línea a `\n`, quita las líneas en blanco del
/// principio y los espacios del final. Conserva la sangría de la primera línea.
pub fn limpiar(texto: &str) -> String {
    let s = texto.replace("\r\n", "\n").replace('\r', "\n");
    let s = s.trim_end();
    let blancos_iniciales = s.len() - s.trim_start().len();
    let corte = s[..blancos_iniciales].rfind('\n').map_or(0, |p| p + 1);
    s[corte..].to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(args: &[&str]) -> Vec<String> {
        args.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn normaliza_acentos_y_mayusculas() {
        assert_eq!(normalizar("Canción ÁRBOL Pingüino"), "cancion arbol pinguino");
        assert_eq!(normalizar_etiqueta("  Línux "), Some("linux".into()));
        assert_eq!(normalizar_etiqueta("   "), None);
    }

    #[test]
    fn terminos_palabras_y_frases() {
        assert_eq!(terminos_busqueda(&v(&["Rust", "SQLite"])), v(&["rust", "sqlite"]));
        assert_eq!(terminos_busqueda(&v(&["base de  datos"])), v(&["base de datos"]));
        assert_eq!(
            terminos_busqueda(&v(&["\"Base de datos\" rust", "rust"])),
            v(&["base de datos", "rust"])
        );
        assert!(terminos_busqueda(&v(&["", "  "])).is_empty());
    }

    #[test]
    fn decodifica_codificaciones() {
        assert_eq!(decodificar("añil".as_bytes()), "añil");
        assert_eq!(decodificar(&[0xEF, 0xBB, 0xBF, b'h', b'i']), "hi");
        assert_eq!(decodificar(&[0xFF, 0xFE, b'h', 0, 0xF1, 0]), "hñ");
        assert_eq!(decodificar(&[b'a', 0xF1, b'o']), "año"); // Latin-1
    }

    #[test]
    fn limpia_texto() {
        assert_eq!(limpiar("\r\n\n  hola\r\nadiós \n\n"), "  hola\nadiós");
        assert_eq!(limpiar(" \n \t"), "");
    }
}
