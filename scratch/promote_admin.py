import sqlite3

conn = sqlite3.connect('db.sqlite3')
cursor = conn.cursor()

cursor.execute("UPDATE dispatcharr_accounts_user SET user_level = 10, is_superuser = 1 WHERE username = 'admin'")
conn.commit()
conn.close()
print("Updated admin user level to 10")
