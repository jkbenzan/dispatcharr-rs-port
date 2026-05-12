import re

with open('svelte-frontend/src/routes/stream-checker/+page.svelte', 'r', encoding='utf-8') as f:
    content = f.read()

content = content.replace("streams.every(s =>", "streams.every((s: any) =>")
content = content.replace("allStreams.every(s =>", "allStreams.every((s: any) =>")

with open('svelte-frontend/src/routes/stream-checker/+page.svelte', 'w', encoding='utf-8') as f:
    f.write(content)
