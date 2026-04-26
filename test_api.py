import urllib.request
import urllib.parse
import json

def main():
    # 1. Login to get token
    login_data = json.dumps({
        "username": "admin",
        "password": "password"  # Assuming default password
    }).encode('utf-8')
    
    try:
        req = urllib.request.Request(
            "http://localhost:8000/api/accounts/auth/login/", 
            data=login_data, 
            headers={'Content-Type': 'application/json'}
        )
        with urllib.request.urlopen(req) as response:
            res_data = json.loads(response.read().decode())
            token = res_data.get("access")
            print("Login successful.")
    except Exception as e:
        print(f"Login failed: {e}")
        # Try getting directly if no auth required
        token = "dummy"

    # 2. Get notifications
    try:
        req = urllib.request.Request(
            "http://localhost:8000/api/core/notifications/",
            headers={'Authorization': f'Bearer {token}'}
        )
        with urllib.request.urlopen(req) as response:
            notifs = json.loads(response.read().decode())
            print("Notifications API Response:")
            print(json.dumps(notifs, indent=2))
    except Exception as e:
        print(f"Notifications API failed: {e}")
        if hasattr(e, 'read'):
            print(e.read().decode())

    # 3. Get count
    try:
        req = urllib.request.Request(
            "http://localhost:8000/api/core/notifications/count/",
            headers={'Authorization': f'Bearer {token}'}
        )
        with urllib.request.urlopen(req) as response:
            count = json.loads(response.read().decode())
            print("Count API Response:")
            print(json.dumps(count, indent=2))
    except Exception as e:
        print(f"Count API failed: {e}")

if __name__ == "__main__":
    main()
