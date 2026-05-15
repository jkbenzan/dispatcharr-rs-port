import sys
import re

def patch_file(path):
    with open(path, 'r', encoding='utf-8') as f:
        content = f.read()

    # 1. Fix JSON.parse bug for custom_properties which might already be an object
    content = content.replace(
        "customProps = vod.custom_properties ? JSON.parse(vod.custom_properties) : {};",
        "customProps = typeof vod.custom_properties === 'string' ? JSON.parse(vod.custom_properties) : (vod.custom_properties || {});"
    )

    # 2. Add local_poster to the background hero section
    content = content.replace(
        "<div class=\"hero-section\" style=\"background-image: url('{vod?.poster_url || ''}');\">",
        "<div class=\"hero-section\" style=\"background-image: url('{customProps.local_poster ? `/api/vod/images/${customProps.local_poster}` : (vod?.poster_url || '')}');\">"
    )

    # 3. Prioritize local_poster in the poster image
    content = content.replace(
        "{#if vod?.poster_url}\n                        <img src={vod.poster_url} alt={vod.name} class=\"poster\" />\n                    {:else}",
        "{#if customProps.local_poster}\n                        <img src={`/api/vod/images/${customProps.local_poster}`} alt={vod?.name} class=\"poster\" />\n                    {:else if vod?.poster_url}\n                        <img src={vod.poster_url} alt={vod.name} class=\"poster\" />\n                    {:else}"
    )

    with open(path, 'w', encoding='utf-8') as f:
        f.write(content)

patch_file('svelte-frontend/src/lib/components/VodDetailModal.svelte')
