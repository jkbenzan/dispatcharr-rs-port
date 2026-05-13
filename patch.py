import io

with io.open('src/stream_checker/checker.rs', 'r', encoding='utf-8') as f:
    content = f.read()

content = content.replace(
    'pub current_stream_name: String,\n    pub completed: usize,',
    'pub current_stream_id: Option<i64>,\n    pub current_stream_name: String,\n    pub completed: usize,'
)

content = content.replace(
    'm3u_account_name: acc_name.clone(),\n                            current_stream_name: String::new(),',
    'm3u_account_name: acc_name.clone(),\n                            current_stream_id: None,\n                            current_stream_name: String::new(),'
)

content = content.replace(
    'w.current_stream_name = stream_obj.name.clone();\n                                w.completed = idx;',
    'w.current_stream_id = Some(stream_obj.id);\n                                w.current_stream_name = stream_obj.name.clone();\n                                w.completed = idx;'
)

content = content.replace(
    'w.completed = total_in_group;\n                            w.current_stream_name = "Finished".to_string();',
    'w.completed = total_in_group;\n                            w.current_stream_id = None;\n                            w.current_stream_name = "Finished".to_string();'
)

with io.open('src/stream_checker/checker.rs', 'w', encoding='utf-8') as f:
    f.write(content)
