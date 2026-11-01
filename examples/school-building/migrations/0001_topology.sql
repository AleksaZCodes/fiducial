-- Derived by fid-topology from topology.toml — do not edit.
-- Readings, roll-ups, settings, command state, warnings and descriptions
-- for 3 node(s) across 2 firmware role(s).

CREATE TABLE IF NOT EXISTS reading (
  node TEXT NOT NULL,
  point INTEGER NOT NULL,
  at INTEGER NOT NULL,
  value REAL,
  PRIMARY KEY (node, point, at)
) WITHOUT ROWID;

CREATE TABLE IF NOT EXISTS reading_hourly (
  node TEXT NOT NULL,
  point INTEGER NOT NULL,
  hour INTEGER NOT NULL,
  mean REAL,
  min REAL,
  max REAL,
  samples INTEGER NOT NULL,
  PRIMARY KEY (node, point, hour)
) WITHOUT ROWID;

CREATE TABLE IF NOT EXISTS parameter_value (
  scope_node TEXT NOT NULL,
  parameter INTEGER NOT NULL,
  value REAL NOT NULL,
  changed_at INTEGER NOT NULL,
  PRIMARY KEY (scope_node, parameter)
) WITHOUT ROWID;

CREATE TABLE IF NOT EXISTS schedule_slot (
  node TEXT NOT NULL,
  schedule TEXT NOT NULL,
  weekday INTEGER NOT NULL CHECK (weekday BETWEEN 1 AND 7),
  from_min INTEGER NOT NULL,
  to_min INTEGER NOT NULL CHECK (to_min > from_min),
  PRIMARY KEY (node, schedule, weekday, from_min)
) WITHOUT ROWID;

CREATE TABLE IF NOT EXISTS device_state (
  node TEXT NOT NULL,
  device INTEGER NOT NULL,
  output TEXT NOT NULL,
  selector TEXT NOT NULL,
  reason TEXT NOT NULL,
  last_command INTEGER,
  last_outcome TEXT,
  at INTEGER NOT NULL,
  PRIMARY KEY (node, device)
) WITHOUT ROWID;

CREATE TABLE IF NOT EXISTS warning (
  id INTEGER PRIMARY KEY,
  node TEXT NOT NULL,
  kind TEXT NOT NULL,
  source INTEGER,
  raised_at INTEGER NOT NULL,
  cleared_at INTEGER
);

CREATE TABLE IF NOT EXISTS description (
  scope TEXT NOT NULL,
  hour INTEGER NOT NULL,
  text TEXT NOT NULL,
  PRIMARY KEY (scope, hour)
) WITHOUT ROWID;
