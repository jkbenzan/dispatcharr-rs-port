import sqlite3
import os

if os.path.exists('test.db'):
    os.remove('test.db')

conn = sqlite3.connect('test.db')
with open('schema.sql', 'r', encoding='utf-8') as f:
    schema = f.read()
    conn.executescript(schema)
conn.commit()
conn.close()
print("test.db created successfully")
