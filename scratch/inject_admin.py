import sqlite3
import uuid
import djangohashers

conn = sqlite3.connect('db.sqlite3')
cursor = conn.cursor()

# Create a test user
cursor.execute("SELECT id FROM dispatcharr_accounts_user WHERE username = 'admin'")
row = cursor.fetchone()
if not row:
    # Use djangohashers to create a password hash for 'admin'
    password_hash = djangohashers.make_password('admin')
    cursor.execute("INSERT INTO dispatcharr_accounts_user (username, password, email, is_superuser, is_staff, is_active, date_joined, first_name, last_name, stream_limit, user_level) VALUES (?, ?, ?, ?, ?, ?, datetime('now'), ?, ?, ?, ?)",
                   ('admin', password_hash, 'admin@example.com', 1, 1, 1, 'Admin', 'User', 10, 1))
    user_id = cursor.lastrowid
else:
    user_id = row[0]

conn.commit()
conn.close()
print(f"User Admin ID: {user_id}")
