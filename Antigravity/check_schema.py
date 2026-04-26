import sqlite3
conn = sqlite3.connect('dispatcharr.db')
c = conn.cursor()
c.execute("SELECT sql FROM sqlite_master WHERE type='table' AND name='stream_sorting_rule';")
print(c.fetchone()[0])
