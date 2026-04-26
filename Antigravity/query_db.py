import psycopg2
conn = psycopg2.connect("postgresql://gemini:password@192.168.0.36:5432/dispatcharr-rs")
cur = conn.cursor()
cur.execute("SELECT id, name FROM dispatcharr_channels_channel LIMIT 5;")
print("Channels:", cur.fetchall())
cur.execute("SELECT * FROM dispatcharr_channels_stream LIMIT 5;")
print("Streams:", cur.fetchall())
cur.execute("SELECT * FROM dispatcharr_channels_channelstream LIMIT 5;")
print("ChannelStreams:", cur.fetchall())
