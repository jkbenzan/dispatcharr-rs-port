import io

with io.open('src/proxy.rs', 'r', encoding='utf-8') as f:
    content = f.read()

target = '''    let stream = futures_util::stream::unfold(
        (ring_data.into_iter(), rx, state_for_stream, channel_id_for_stream, client_id_for_stream),
        move |(mut ring_iter, mut rx, st, ch, cl)| async move {'''

replacement = '''    let stream = futures_util::stream::unfold(
        (ring_data.into_iter(), rx, state_for_stream, channel_id_for_stream, client_id_for_stream, guard),
        move |(mut ring_iter, mut rx, st, ch, cl, _guard)| async move {'''

content = content.replace(target, replacement)

target2 = '''            if let Some(bytes) = ring_iter.next() {
                return Some((Ok::<_, std::io::Error>(bytes), (ring_iter, rx, st, ch, cl)));
            }
            
            loop {
                match rx.recv().await {
                    Ok(bytes) => return Some((Ok(bytes), (ring_iter, rx, st, ch, cl))),'''

replacement2 = '''            if let Some(bytes) = ring_iter.next() {
                return Some((Ok::<_, std::io::Error>(bytes), (ring_iter, rx, st, ch, cl, _guard)));
            }
            
            loop {
                match rx.recv().await {
                    Ok(bytes) => return Some((Ok(bytes), (ring_iter, rx, st, ch, cl, _guard))),'''

content = content.replace(target2, replacement2)

with io.open('src/proxy.rs', 'w', encoding='utf-8') as f:
    f.write(content)
