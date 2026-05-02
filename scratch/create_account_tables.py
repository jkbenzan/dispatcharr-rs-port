import sqlite3

conn = sqlite3.connect('db.sqlite3')
cursor = conn.cursor()

# accounts_user
cursor.execute("""
CREATE TABLE IF NOT EXISTS accounts_user (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    password TEXT NOT NULL,
    last_login TEXT,
    is_superuser BOOLEAN NOT NULL DEFAULT 0,
    username TEXT NOT NULL UNIQUE,
    first_name TEXT NOT NULL DEFAULT '',
    last_name TEXT NOT NULL DEFAULT '',
    email TEXT NOT NULL DEFAULT '',
    is_staff BOOLEAN NOT NULL DEFAULT 0,
    is_active BOOLEAN NOT NULL DEFAULT 1,
    date_joined TEXT NOT NULL,
    avatar_config TEXT,
    user_level INTEGER NOT NULL DEFAULT 1,
    custom_properties TEXT,
    api_key TEXT,
    stream_limit INTEGER NOT NULL DEFAULT 0
)
""")

# accounts_user_groups
cursor.execute("""
CREATE TABLE IF NOT EXISTS accounts_user_groups (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id INTEGER NOT NULL,
    group_id INTEGER NOT NULL
)
""")

# auth_group
cursor.execute("""
CREATE TABLE IF NOT EXISTS auth_group (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL UNIQUE
)
""")

# auth_group_permissions
cursor.execute("""
CREATE TABLE IF NOT EXISTS auth_group_permissions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    group_id INTEGER NOT NULL,
    permission_id INTEGER NOT NULL
)
""")

# auth_permission
cursor.execute("""
CREATE TABLE IF NOT EXISTS auth_permission (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    content_type_id INTEGER NOT NULL,
    codename TEXT NOT NULL
)
""")

conn.commit()
conn.close()
print("Account tables created successfully")
