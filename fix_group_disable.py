with open('src/api.rs', 'r', encoding='utf-8') as f:
    content = f.read()

old = '''                    let mut active: channel_group_m3u_account::ActiveModel = mapping.into();
                    if let Some(enabled) = setting.get("enabled").and_then(|v| v.as_bool()) {
                        active.enabled = sea_orm::Set(enabled);
                    }
                    if let Some(auto_sync) =
                        setting.get("auto_channel_sync").and_then(|v| v.as_bool())
                    {
                        active.auto_channel_sync = sea_orm::Set(auto_sync);
                    }
                    let _ = active.update(&state.db).await;'''

new = '''                    let was_enabled = mapping.enabled;
                    let mut active: channel_group_m3u_account::ActiveModel = mapping.into();
                    let new_enabled = setting.get("enabled").and_then(|v| v.as_bool());
                    if let Some(enabled) = new_enabled {
                        active.enabled = sea_orm::Set(enabled);
                    }
                    if let Some(auto_sync) =
                        setting.get("auto_channel_sync").and_then(|v| v.as_bool())
                    {
                        active.auto_channel_sync = sea_orm::Set(auto_sync);
                    }
                    let _ = active.update(&state.db).await;

                    // When a group is disabled, delete its streams from the DB
                    if was_enabled && new_enabled == Some(false) {
                        tracing::info!(
                            "[M3U] Group {} disabled - deleting streams for account {}",
                            cg_id, account_id
                        );
                        let streams_to_delete: Vec<i64> = stream::Entity::find()
                            .filter(stream::Column::M3uAccountId.eq(account_id))
                            .filter(stream::Column::ChannelGroupId.eq(cg_id))
                            .all(&state.db)
                            .await
                            .unwrap_or_default()
                            .into_iter()
                            .map(|s| s.id)
                            .collect();

                        if !streams_to_delete.is_empty() {
                            // Remove from channel assignments first
                            let _ = state.db.execute(
                                sea_orm::Statement::from_sql_and_values(
                                    sea_orm::DatabaseBackend::Postgres,
                                    "DELETE FROM dispatcharr_channels_channelstream WHERE stream_id = ANY($1)",
                                    vec![streams_to_delete.clone().into()],
                                )
                            ).await;
                            // Delete the streams themselves
                            let _ = stream::Entity::delete_many()
                                .filter(stream::Column::Id.is_in(streams_to_delete))
                                .exec(&state.db)
                                .await;
                        }
                    }'''

if old in content:
    content = content.replace(old, new, 1)
    with open('src/api.rs', 'w', encoding='utf-8') as f:
        f.write(content)
    print('Done - group disable stream deletion added')
else:
    print('NOT FOUND')
    idx = content.find('channel_group_m3u_account::ActiveModel')
    print(repr(content[max(0,idx-30):idx+200]))
