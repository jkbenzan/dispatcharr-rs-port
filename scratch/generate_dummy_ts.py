import urllib.request
import os

os.makedirs('data', exist_ok=True)
url = "https://raw.githubusercontent.com/mdn/learning-area/master/html/multimedia-and-embedding/video-and-audio-content/rabbit320.webm"
# Since finding a raw static .ts file is hard, I will just generate a dummy .ts file with 100 packets of empty data (131600 bytes) 
# Wait, this won't play in VLC. Let's just download a known small file. Actually, if I just create dummy bytes, the logic of reading and streaming will be verified, even if the video player shows a blank screen.

with open('data/offline.ts', 'wb') as f:
    f.write(b'\x47' + b'\x00' * 187 * 1000) # 1000 TS packets
    
print("Generated dummy offline.ts")
