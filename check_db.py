import sqlite3
import sys

def main():
    conn = sqlite3.connect('db.sqlite3')
    cursor = conn.cursor()
    
    # Get all tables
    cursor.execute("SELECT name FROM sqlite_master WHERE type='table';")
    tables = [x[0] for x in cursor.fetchall()]
    print("Tables:", tables)
    
    # Check if there are streams or accounts
    for table in ["dispatcharr_m3u_account", "dispatcharr_channels_stream", "dispatcharr_channels_channel"]:
        if table in tables:
            cursor.execute(f"SELECT COUNT(*) FROM {table}")
            print(f"{table} count: {cursor.fetchone()[0]}")
        else:
            print(f"{table} not found")

if __name__ == '__main__':
    main()
