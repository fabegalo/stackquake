-- Esquema do banco local (SQLCipher), um banco por conta Google.
-- Versão controlada por PRAGMA user_version.
-- kind: 'crash' | 'anr' | 'nonfatal' (mesmo vocabulário para Play e Crashlytics).

CREATE TABLE IF NOT EXISTS app (
  package_name     TEXT PRIMARY KEY,
  display_name     TEXT,
  firebase_project TEXT,          -- descoberto automaticamente (Firebase Management API)
  firebase_app_id  TEXT,          -- ex.: 1:123:android:abc (usado pela API do Crashlytics)
  enabled          INTEGER NOT NULL DEFAULT 1,
  last_sync_at     TEXT
);

-- Android Vitals (Play) por dia. version_code = '*' guarda o total do app.
CREATE TABLE IF NOT EXISTS vitals_daily (
  package_name        TEXT NOT NULL,
  metric_set          TEXT NOT NULL CHECK (metric_set IN ('crash', 'anr')),
  day                 TEXT NOT NULL,
  version_code        TEXT NOT NULL,
  rate                REAL,
  user_perceived_rate REAL,
  distinct_users      INTEGER,
  PRIMARY KEY (package_name, metric_set, day, version_code)
);

CREATE TABLE IF NOT EXISTS play_issue (
  package_name   TEXT NOT NULL,
  issue_id       TEXT NOT NULL,
  kind           TEXT NOT NULL,
  cause          TEXT,
  location       TEXT,
  title          TEXT NOT NULL,
  signature      TEXT NOT NULL,
  report_count   INTEGER,
  distinct_users INTEGER,
  first_version  TEXT,
  last_version   TEXT,
  last_seen      TEXT,
  issue_uri      TEXT,
  sample_report  TEXT,
  synced_at      TEXT NOT NULL,
  PRIMARY KEY (package_name, issue_id)
);

-- Issues do Crashlytics (API do Crashlytics, sem BigQuery).
CREATE TABLE IF NOT EXISTS crashlytics_issue (
  package_name  TEXT NOT NULL,
  issue_id      TEXT NOT NULL,
  kind          TEXT NOT NULL,
  title         TEXT NOT NULL,
  subtitle      TEXT,
  root_type     TEXT,
  blame_symbol  TEXT,
  signature     TEXT NOT NULL,
  first_version TEXT,             -- versionCode
  last_version  TEXT,             -- versionCode
  first_seen    TEXT,
  last_seen     TEXT,
  issue_uri     TEXT,
  sample_event  TEXT,
  sample_stack  TEXT,
  PRIMARY KEY (package_name, issue_id)
);

-- Eventos e usuários afetados por issue, por dia (UTC).
CREATE TABLE IF NOT EXISTS crashlytics_issue_daily (
  package_name TEXT NOT NULL,
  issue_id     TEXT NOT NULL,
  day          TEXT NOT NULL,
  events       INTEGER NOT NULL,
  users        INTEGER NOT NULL,
  PRIMARY KEY (package_name, issue_id, day)
);

-- Saúde por versão e dia. kind: 'all' (com % sem falha), 'crash', 'anr'.
CREATE TABLE IF NOT EXISTS crashlytics_version_daily (
  package_name        TEXT NOT NULL,
  kind                TEXT NOT NULL CHECK (kind IN ('all', 'crash', 'anr')),
  day                 TEXT NOT NULL,
  build_version       TEXT NOT NULL,   -- versionCode
  display_version     TEXT,            -- versionName
  events              INTEGER NOT NULL,
  users               INTEGER NOT NULL,
  crash_free_users    REAL,            -- fração 0–1
  crash_free_sessions REAL,
  PRIMARY KEY (package_name, kind, day, build_version)
);

-- Distribuição de um issue por versão (buscada sob demanda no detalhe).
CREATE TABLE IF NOT EXISTS crashlytics_issue_version (
  package_name    TEXT NOT NULL,
  issue_id        TEXT NOT NULL,
  build_version   TEXT NOT NULL,
  display_version TEXT,
  events          INTEGER NOT NULL,
  users           INTEGER NOT NULL,
  fetched_at      TEXT NOT NULL,
  PRIMARY KEY (package_name, issue_id, build_version)
);

-- Problema unificado: uma assinatura pode juntar issues das duas fontes.
CREATE TABLE IF NOT EXISTS problem (
  id                  INTEGER PRIMARY KEY,
  package_name        TEXT NOT NULL,
  signature           TEXT NOT NULL,
  kind                TEXT NOT NULL,
  title               TEXT NOT NULL,
  status              TEXT NOT NULL DEFAULT 'open'
                      CHECK (status IN ('open', 'investigating', 'resolved', 'ignored', 'regressed')),
  assignee            TEXT,
  notes               TEXT,
  resolved_in_version TEXT,
  created_at          TEXT NOT NULL,
  updated_at          TEXT NOT NULL,
  UNIQUE (package_name, signature)
);

CREATE TABLE IF NOT EXISTS sync_state (
  source       TEXT NOT NULL,
  package_name TEXT NOT NULL,
  cursor_day   TEXT NOT NULL,
  PRIMARY KEY (source, package_name)
);

CREATE INDEX IF NOT EXISTS idx_play_issue_sig ON play_issue (package_name, signature);
CREATE INDEX IF NOT EXISTS idx_cl_issue_sig ON crashlytics_issue (package_name, signature);
CREATE INDEX IF NOT EXISTS idx_cl_issue_daily_day ON crashlytics_issue_daily (package_name, day);
CREATE INDEX IF NOT EXISTS idx_cl_version_daily_day ON crashlytics_version_daily (package_name, day);
