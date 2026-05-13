import io

with io.open('svelte-frontend/src/routes/stream-checker/+page.svelte', 'r', encoding='utf-8') as f:
    content = f.read()

content = content.replace(
    '{@const isTesting = status?.workers?.some(w => w.current_stream_id === stream.id)}',
    '{@const isTesting = status?.workers?.some((w: any) => w.current_stream_id === stream.id)}'
)

with io.open('svelte-frontend/src/routes/stream-checker/+page.svelte', 'w', encoding='utf-8') as f:
    f.write(content)
