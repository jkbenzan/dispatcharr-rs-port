import sqlite3
import uuid
import hashlib

def hash_password(password):
    # Simple PBKDF2 with SHA256 (matches common rust implementations)
    # Actually, verify_password in auth.rs might use bcrypt or argon2.
    # Let me check src/auth.rs
    pass

conn = sqlite3.connect('db.sqlite3')
cursor = conn.cursor()

# Create a test channel if not exists
cursor.execute("SELECT id, uuid FROM dispatcharr_channels_channel WHERE name = 'Test Channel'")
row = cursor.fetchone()
if not row:
    channel_uuid = str(uuid.uuid4())
    cursor.execute("INSERT INTO dispatcharr_channels_channel (name, uuid, channel_number, is_adult, auto_created, user_level, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, datetime('now'), datetime('now'))", 
                   ('Test Channel', channel_uuid, 1, 0, 0, 0))
    channel_id = cursor.lastrowid
else:
    channel_id = row[0]
    channel_uuid = row[1]

# Create a test stream for this channel
cursor.execute("SELECT id FROM dispatcharr_channels_stream WHERE url = 'http://localhost:9999'")
stream_row = cursor.fetchone()
if not stream_row:
    cursor.execute("INSERT INTO dispatcharr_channels_stream (name, url, is_custom, current_viewers, is_stale, is_adult, updated_at, last_seen) VALUES (?, ?, ?, ?, ?, ?, datetime('now'), datetime('now'))",
                   ('Mock Stream', 'http://localhost:9999', 1, 0, 0, 0))
    stream_id = cursor.lastrowid
else:
    stream_id = stream_row[0]

# Link stream to channel
cursor.execute("SELECT id FROM dispatcharr_channels_channelstream WHERE channel_id = ? AND stream_id = ?", (channel_id, stream_id))
if not cursor.fetchone():
    cursor.execute("INSERT INTO dispatcharr_channels_channelstream (channel_id, stream_id, \"order\") VALUES (?, ?, ?)",
                   (channel_id, stream_id, 0))

# Create a test user with no password (or known hash)
# Let's check src/auth.rs to see what it expects
conn.commit()
conn.close()
print(f"Test Channel ID: {channel_id}")
print(f"Test Channel UUID: {channel_uuid}")
