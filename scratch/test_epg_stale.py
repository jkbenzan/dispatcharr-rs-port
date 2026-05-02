import sqlite3
import datetime

conn = sqlite3.connect('db.sqlite3')
c = conn.cursor()

# Find an active EPG source
c.execute("SELECT id, name, updated_at, refresh_interval FROM epg_epgsource WHERE is_active = 1 LIMIT 1")
row = c.fetchone()

if row:
    source_id, name, updated_at, refresh_interval = row
    print(f"Found active source: {name} (ID: {source_id})")
    
    # Set updated_at to 24 hours ago and interval to 1 hour
    # SQLite DateTime format: 2026-05-01 18:15:52+00:00
    old_time = (datetime.datetime.now() - datetime.timedelta(hours=24)).strftime('%Y-%m-%d %H:%M:%S+00:00')
    c.execute("UPDATE epg_epgsource SET updated_at = ?, refresh_interval = 1 WHERE id = ?", (old_time, source_id))
    conn.commit()
    print(f"Updated {name} to be stale (Last update: {old_time}, Interval: 1h)")
else:
    print("No active EPG sources found.")

conn.close()
