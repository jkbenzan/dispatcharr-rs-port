import psycopg2
from datetime import datetime, timezone

def main():
    try:
        conn = psycopg2.connect("postgresql://gemini:password@192.168.0.36:5432/dispatcharr-rs")
        cursor = conn.cursor()

        now = datetime.now(timezone.utc)

        # Insert a test notification
        cursor.execute("""
            INSERT INTO core_systemnotification 
            (notification_key, notification_type, priority, source, title, message, action_data, is_active, admin_only, created_at, updated_at) 
            VALUES 
            (%s, %s, %s, %s, %s, %s, %s, %s, %s, %s, %s)
        """, (
            "test_notification_1", 
            "info", 
            "normal", 
            "system", 
            "Test Notification", 
            "This is a test notification to verify the frontend works properly.", 
            "{}", 
            True, 
            False, 
            now, 
            now
        ))

        conn.commit()
        print("Successfully inserted test notification!")

    except Exception as e:
        print(f"Error: {e}")
    finally:
        if 'conn' in locals():
            cursor.close()
            conn.close()

if __name__ == "__main__":
    main()
