import io

with io.open('svelte-frontend/src/routes/stream-checker/+page.svelte', 'r', encoding='utf-8') as f:
    content = f.read()

content = content.replace(
    'toast.info(Sorting streams in  channels...);',
    'toast.info(Sorting streams in  channels...);'
)

with io.open('svelte-frontend/src/routes/stream-checker/+page.svelte', 'w', encoding='utf-8') as f:
    f.write(content)
