with open('src/api.rs', 'r', encoding='utf-8') as f:
    content = f.read()

old = '    Ok(Json(serde_json::json!({"id": id, "success": true})))\n}'

new = '''    // Return full channel with nested streams so the frontend store updates correctly
    let channel_streams = channel_stream::Entity::find()
        .filter(channel_stream::Column::ChannelId.eq(id))
        .order_by_asc(channel_stream::Column::Order)
        .all(&state.db)
        .await
        .unwrap_or_default();

    let updated_ch = crate::entities::channel::Entity::find_by_id(id)
        .one(&state.db)
        .await
        .ok()
        .flatten();

    let mut ch_json = match updated_ch {
        Some(ch) => serde_json::to_value(&ch).unwrap(),
        None => json!({"id": id}),
    };

    let mut streams_json = Vec::new();
    for cs in channel_streams {
        let mut cs_json = serde_json::to_value(&cs).unwrap();
        if let Ok(Some(s)) = stream::Entity::find_by_id(cs.stream_id).one(&state.db).await {
            cs_json["stream"] = serde_json::to_value(&s).unwrap();
        }
        streams_json.push(cs_json);
    }
    ch_json["streams"] = json!(streams_json);

    Ok(Json(ch_json))
}'''

if old in content:
    content = content.replace(old, new, 1)
    with open('src/api.rs', 'w', encoding='utf-8') as f:
        f.write(content)
    print('Done - update_channel response updated')
else:
    # show what's there
    idx = content.find('"success": true})))')
    print(f'NOT FOUND. Context around success:')
    print(repr(content[max(0,idx-30):idx+60]))
