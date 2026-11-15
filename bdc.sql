CREATE TABLE etiquetas (
            id     INTEGER PRIMARY KEY AUTOINCREMENT,
            nombre TEXT    NOT NULL UNIQUE
        );

CREATE TABLE texto_etiquetas (
            texto_id    INTEGER NOT NULL REFERENCES textos(id),
            etiqueta_id INTEGER NOT NULL REFERENCES etiquetas(id),
            PRIMARY KEY (texto_id, etiqueta_id)
        );

CREATE TABLE textos (
            id         INTEGER PRIMARY KEY AUTOINCREMENT,
            contenido  TEXT    NOT NULL,
            creado_en  TEXT    NOT NULL DEFAULT (datetime('now'))
        );
