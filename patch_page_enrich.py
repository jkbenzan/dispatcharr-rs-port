import sys

def patch_file(path):
    with open(path, 'r', encoding='utf-8') as f:
        content = f.read()

    # Patch 1: getImageUrl
    content = content.replace(
        "  function getImageUrl(item: any) {\n    const directPoster",
        "  function getImageUrl(item: any) {\n    if (item.custom_properties) {\n      let props;\n      try {\n        props = typeof item.custom_properties === 'string' ? JSON.parse(item.custom_properties) : item.custom_properties;\n        if (props.local_poster) {\n          return `/api/vod/images/${props.local_poster}`;\n        }\n      } catch (e) {}\n    }\n\n    const directPoster"
    )

    # Patch 2: enrich progress
    content = content.replace(
        "    if (sentinel) observer.observe(sentinel);\n  });\n\n  onDestroy(() => {\n    if (observer) observer.disconnect();\n  });",
        "    if (sentinel) observer.observe(sentinel);\n\n    fetchEnrichProgress();\n    enrichInterval = setInterval(fetchEnrichProgress, 5000);\n  });\n\n  let enrichStats = { movies_remaining: 0, series_remaining: 0, total_remaining: 0 };\n  let enrichInterval: any;\n\n  async function fetchEnrichProgress() {\n      try {\n          const res = await fetch('/api/vod/enrich_progress/');\n          if (res.ok) {\n              enrichStats = await res.json();\n              if (enrichStats.total_remaining === 0 && enrichInterval) {\n                  clearInterval(enrichInterval);\n              }\n          }\n      } catch (err) {\n          console.error(\"Failed to fetch enrich progress\", err);\n      }\n  }\n\n  onDestroy(() => {\n    if (observer) observer.disconnect();\n    if (enrichInterval) clearInterval(enrichInterval);\n  });"
    )

    # Patch 3: UI
    content = content.replace(
        "\t<main class=\"content-area\">\n    <!-- Tabs -->",
        "\t<main class=\"content-area\">\n    <!-- Enrichment Progress -->\n    {#if enrichStats.total_remaining > 0}\n      <div class=\"enrichment-banner\">\n        <div class=\"enrichment-info\">\n          <span class=\"pulse-dot\"></span>\n          <span>Background Enrichment: {enrichStats.total_remaining} items remaining (Downloading Posters & Metadata)</span>\n        </div>\n      </div>\n    {/if}\n\n    <!-- Tabs -->"
    )

    with open(path, 'w', encoding='utf-8') as f:
        f.write(content)

patch_file('svelte-frontend/src/routes/vod/+page.svelte')
