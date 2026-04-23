with open("src/proxy.rs", "r") as f:
    content = f.read()

if "use std::sync::Arc;" not in content:
    content = "use std::sync::Arc;\n" + content

if "use hex::decode;" not in content and "use jsonwebtoken::decode;" not in content:
    content = "use jsonwebtoken::{decode, DecodingKey, Validation};\n" + content

with open("src/proxy.rs", "w") as f:
    f.write(content)
