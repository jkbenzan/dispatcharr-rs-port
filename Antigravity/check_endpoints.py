import re

frontend_routes = set()
with open('Dispatcharr-main/frontend/src/api.js', 'r', encoding='utf-8') as f:
    content = f.read()
    matches = re.findall(r'[\'`\"](?:(?:\$\{host\})?(/api/[^\'\`\"\?]+))', content)
    for match in matches:
        clean_route = re.sub(r'\$\{[^\}]+\}', ':id', match)
        clean_route = clean_route.rstrip('/')
        frontend_routes.add(clean_route)

backend_routes = set()
with open('src/main.rs', 'r', encoding='utf-8') as f:
    content = f.read()
    matches = re.findall(r'\.route\(\s*\"(/api/[^\"]+)\"', content)
    for match in matches:
        clean_route = match.replace(':id', ':id').replace(':group_id', ':id').replace(':profile_id', ':id').replace(':filter_id', ':id').replace(':channel_id', ':id')
        clean_route = clean_route.rstrip('/')
        backend_routes.add(clean_route)

print('--- FRONTEND EXPECTS BUT BACKEND IS MISSING ---')
for route in sorted(frontend_routes):
    norm_route = re.sub(r':\w+', ':id', route)
    found = False
    for b_route in backend_routes:
        norm_b_route = re.sub(r':\w+', ':id', b_route)
        if norm_route == norm_b_route or norm_route + '/' == norm_b_route or norm_b_route + '/' == norm_route:
            found = True
            break
    if not found:
        print(route)
