with open("src/proxy.rs", "r") as f:
    content = f.read()

if "use crate::{AppState, entities::{channel, channel_stream, stream}};" not in content:
    content = "use crate::{AppState, entities::{channel, channel_stream, stream}};\n" + content

with open("src/proxy.rs", "w") as f:
    f.write(content)
