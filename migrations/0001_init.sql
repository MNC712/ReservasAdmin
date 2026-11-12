-- Migración inicial: esquema completo del sistema de reservas hoteleras.
-- Fechas como TEXT ISO 'YYYY-MM-DD', timestamps TEXT ISO con hora.

PRAGMA foreign_keys = ON;

-- Datos generales del hotel (fila única id = 1)
CREATE TABLE hotel_config (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    nombre TEXT NOT NULL DEFAULT '',
    direccion TEXT NOT NULL DEFAULT '',
    telefono TEXT NOT NULL DEFAULT '',
    logo_path TEXT NOT NULL DEFAULT '',
    moneda TEXT NOT NULL DEFAULT 'EUR'
);

CREATE TABLE habitaciones (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    numero TEXT UNIQUE NOT NULL,
    tipo TEXT NOT NULL,
    capacidad INTEGER NOT NULL DEFAULT 2,
    precio_base REAL NOT NULL DEFAULT 0,
    activa INTEGER NOT NULL DEFAULT 1
);

CREATE TABLE clientes (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    nombre TEXT NOT NULL,
    apellidos TEXT NOT NULL DEFAULT '',
    documento TEXT NOT NULL DEFAULT '',
    email TEXT NOT NULL DEFAULT '',
    telefono TEXT NOT NULL DEFAULT ''
);

CREATE TABLE huespedes (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    nombre TEXT NOT NULL,
    apellidos TEXT NOT NULL DEFAULT '',
    documento TEXT NOT NULL DEFAULT '',
    fecha_nacimiento TEXT NOT NULL DEFAULT '',
    nacionalidad TEXT NOT NULL DEFAULT ''
);

CREATE TABLE reservas (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    sync_id TEXT UNIQUE NOT NULL,
    cliente_id INTEGER NOT NULL REFERENCES clientes(id),
    fecha_inicio TEXT NOT NULL,
    fecha_fin TEXT NOT NULL,
    estado TEXT NOT NULL DEFAULT 'pendiente' CHECK (estado IN ('pendiente','confirmada','checkin','cancelada')),
    tarifa_total REAL NOT NULL DEFAULT 0,
    notas TEXT NOT NULL DEFAULT '',
    origen TEXT NOT NULL DEFAULT 'local',
    creado_en TEXT NOT NULL,
    actualizado_en TEXT NOT NULL
);

-- Tabla puente reserva <-> habitaciones (multi-habitación por reserva)
CREATE TABLE reserva_habitaciones (
    reserva_id INTEGER NOT NULL REFERENCES reservas(id) ON DELETE CASCADE,
    habitacion_id INTEGER NOT NULL REFERENCES habitaciones(id),
    PRIMARY KEY (reserva_id, habitacion_id)
);

-- Tabla puente reserva <-> huéspedes/acompañantes
CREATE TABLE reserva_huespedes (
    reserva_id INTEGER NOT NULL REFERENCES reservas(id) ON DELETE CASCADE,
    huesped_id INTEGER NOT NULL REFERENCES huespedes(id),
    PRIMARY KEY (reserva_id, huesped_id)
);

-- Auditoría de movimientos (quién/cuándo/qué cambió)
CREATE TABLE movimientos (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    tipo TEXT NOT NULL CHECK (tipo IN ('creacion','modificacion','cancelacion')),
    entidad TEXT NOT NULL,
    id_entidad INTEGER NOT NULL,
    datos_antes TEXT NOT NULL DEFAULT '',
    datos_despues TEXT NOT NULL DEFAULT '',
    usuario TEXT NOT NULL DEFAULT 'sistema',
    timestamp TEXT NOT NULL
);

-- Colores configurables por estado de reserva (para calendario)
CREATE TABLE config_colores (
    estado TEXT PRIMARY KEY,
    color_hex TEXT NOT NULL
);

-- Configuración de copias de seguridad (fila única id = 1)
CREATE TABLE config_backup (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    carpeta TEXT NOT NULL DEFAULT 'backups',
    intervalo_min INTEGER NOT NULL DEFAULT 60,
    retencion_dias INTEGER NOT NULL DEFAULT 30
);

-- Mapeo configurable campo interno -> columna de Excel (letra o nombre de cabecera)
CREATE TABLE excel_column_map (
    campo TEXT PRIMARY KEY,
    columna_excel TEXT NOT NULL
);

-- Seeds -----------------------------------------------------------------

INSERT INTO hotel_config (id, nombre, direccion, telefono, logo_path, moneda)
VALUES (1, 'Mi Hotel', '', '', '', 'EUR');

INSERT INTO habitaciones (numero, tipo, capacidad, precio_base, activa) VALUES
    ('101', 'Doble', 2, 60.0, 1),
    ('102', 'Doble', 2, 60.0, 1),
    ('103', 'Individual', 1, 45.0, 1),
    ('201', 'Suite', 4, 120.0, 1),
    ('202', 'Suite', 4, 120.0, 1);

INSERT INTO config_colores (estado, color_hex) VALUES
    ('pendiente', '#E6B800'),
    ('confirmada', '#2E86DE'),
    ('checkin', '#27AE60'),
    ('cancelada', '#C0392B');

INSERT INTO config_backup (id, carpeta, intervalo_min, retencion_dias)
VALUES (1, 'backups', 60, 30);

INSERT INTO excel_column_map (campo, columna_excel) VALUES
    ('titular_nombre', 'A'),
    ('titular_documento', 'B'),
    ('fecha_inicio', 'C'),
    ('fecha_fin', 'D'),
    ('habitacion', 'E'),
    ('tarifa', 'F'),
    ('estado', 'G'),
    ('notas', 'H');
