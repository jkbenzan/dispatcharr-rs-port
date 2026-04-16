with open("docker.log", "r", encoding="utf-16", errors="ignore") as f:
    lines = f.readlines()
print("".join(lines[-150:]))
