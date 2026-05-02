import requests
import json
import sqlite3

def test_admin_auth():
    base_url = "http://localhost:8080"
    
    # 1. Create a user manually in DB with user_level 10 but is_superuser 0
    conn = sqlite3.connect('db.sqlite3')
    cursor = conn.cursor()
    
    username = "test_admin_level"
    password_hash = "pbkdf2_sha256$260000$some_dummy_hash$dummy" # We'll update this via logic or just use a known one
    # Actually it's easier to just use create_user if we have an existing admin
    
    cursor.execute("DELETE FROM accounts_user WHERE username=?", (username,))
    conn.commit()
    conn.close()

    print("Note: This script assumes the server is RUNNING.")
    
    # Let's just promote an existing user if possible, or create one.
    # We'll use the promotion script logic but set is_superuser to 0.
    
    import subprocess
    
    # First ensure we have a user. Let's use 'admin' if it exists.
    # Or just use sqlite to insert a user with a known password.
    # 'password123' hashed with djangohashers (PBKDF2)
    hashed_pwd = "pbkdf2_sha256$600000$9pS9m7mO9m7mO9m7mO9m7m$BfS9m7mO9m7mO9m7mO9m7mO9m7mO9m7mO9m7mO9m7mO=" # Dummy
    # Better: just use a known working hash from an existing user.
    
    conn = sqlite3.connect('db.sqlite3')
    cursor = conn.cursor()
    cursor.execute("SELECT password FROM accounts_user WHERE is_superuser = 1 LIMIT 1")
    row = cursor.fetchone()
    if not row:
        print("No superuser found to copy password from.")
        return
    admin_pwd_hash = row[0]
    
    import datetime
    now = datetime.datetime.now().isoformat()
    
    cursor.execute("""
        INSERT INTO accounts_user (password, is_superuser, username, first_name, last_name, email, is_staff, is_active, date_joined, user_level, stream_limit)
        VALUES (?, 0, ?, 'Test', 'Admin', 'test@example.com', 0, 1, ?, 10, 0)
    """, (admin_pwd_hash, username, now))
    conn.commit()
    conn.close()
    
    print(f"Created user {username} with user_level 10 and is_superuser 0")
    
    # 2. Login as the new user
    login_data = {"username": username, "password": "admin"} 
    
    try:
        r = requests.post(f"{base_url}/api/accounts/token/", json=login_data)
        if r.status_code != 200:
            print(f"Login failed: {r.status_code} {r.text}")
            return
        
        token = r.json().get("access")
        print("Login successful, got token.")
        
        # 3. Fetch user list
        headers = {"Authorization": f"Bearer {token}"}
        r = requests.get(f"{base_url}/api/accounts/users/", headers=headers)
        
        if r.status_code == 200:
            print("SUCCESS: Admin-level user (user_level 10) successfully fetched user list!")
        elif r.status_code == 403:
            print("FAILURE: Admin-level user received 403 Forbidden.")
        else:
            print(f"Unexpected status code: {r.status_code} {r.text}")
            
    except Exception as e:
        print(f"Error during test: {e}")

if __name__ == "__main__":
    test_admin_auth()
