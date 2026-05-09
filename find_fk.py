import sqlite3
db = sqlite3.connect('db.sqlite3')
db.row_factory = sqlite3.Row
tables = [r[0] for r in db.execute("SELECT name FROM sqlite_master WHERE type='table'")]
refs = []
for t in tables:
    for fk in db.execute(f"PRAGMA foreign_key_list({t})"):
        if fk['table'] == 'm3u_m3uaccount':
            refs.append((t, dict(fk)))
print(refs)
