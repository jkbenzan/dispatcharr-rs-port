with open("src/entities/mod.rs", "r") as f:
    mod_rs = f.read()

lines = mod_rs.splitlines()
unique_lines = []
for line in lines:
    if line not in unique_lines or not line.startswith("pub mod"):
        unique_lines.append(line)

with open("src/entities/mod.rs", "w") as f:
    f.write("\n".join(unique_lines) + "\n")
