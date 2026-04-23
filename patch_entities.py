with open('src/entities/mod.rs', 'r') as f:
    lines = f.readlines()

unique_lines = []
seen = set()
for line in lines:
    if line.strip() and line in seen:
        continue
    seen.add(line)
    unique_lines.append(line)

new_modules = [
    "pub mod vod_movie;\n",
    "pub mod vod_series;\n",
    "pub mod vod_m3uvodcategoryrelation;\n"
]

for mod in new_modules:
    if mod not in unique_lines:
        unique_lines.append(mod)

with open('src/entities/mod.rs', 'w') as f:
    f.writelines(unique_lines)
