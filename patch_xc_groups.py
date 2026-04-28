with open('src/m3u.rs', 'r', encoding='utf-8') as f:
    content = f.read()

# In fetch_and_parse_xc, add disabled group loading after group_id_map population
# and skip in the stream insertion loop

old_xc_group_map = '''    let mut category_map = HashMap::new();
    for cat in categories {
        category_map.insert(cat.category_id.clone(), cat.category_name.clone());
        get_or_create_channel_group_id(
            db,
            &cat.category_name,
            account_id,
            auto_sync_live,
            &mut group_id_map,
        )
        .await;
    }

    for s in xc_streams {'''

new_xc_group_map = '''    let mut category_map = HashMap::new();
    for cat in categories {
        category_map.insert(cat.category_id.clone(), cat.category_name.clone());
        get_or_create_channel_group_id(
            db,
            &cat.category_name,
            account_id,
            auto_sync_live,
            &mut group_id_map,
        )
        .await;
    }

    // Load disabled group IDs and purge any existing streams in those groups.
    let xc_disabled_group_ids: HashSet<i64> = channel_group_m3u_account::Entity::find()
        .filter(channel_group_m3u_account::Column::M3uAccountId.eq(account_id))
        .filter(channel_group_m3u_account::Column::Enabled.eq(false))
        .all(db)
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|m| m.channel_group_id as i64)
        .collect();

    if !xc_disabled_group_ids.is_empty() {
        let ids: Vec<i64> = xc_disabled_group_ids.iter().cloned().collect();
        let _ = stream::Entity::delete_many()
            .filter(stream::Column::M3uAccountId.eq(account_id))
            .filter(stream::Column::ChannelGroupId.is_in(ids))
            .exec(db)
            .await;
        tracing::info!(
            "[XC] Purged existing streams for {} disabled groups on account {}",
            xc_disabled_group_ids.len(), account_id
        );
    }

    for s in xc_streams {'''

if old_xc_group_map in content:
    content = content.replace(old_xc_group_map, new_xc_group_map, 1)
    print("XC Step 1 done: added disabled group loading + purge")
else:
    print("ERROR: XC Step 1 target not found")

# In the XC per-stream loop, skip if group is disabled
# The cg_id is resolved via group_id_map; skip if it's in disabled set
old_xc_cg = '''            let cg_id = group_id_map.get(&group_title).cloned();

            let stream_model = stream::ActiveModel {
                m3u_account_id: Set(Some(account_id)),'''

new_xc_cg = '''            let cg_id = group_id_map.get(&group_title).cloned();

            // Skip streams whose group is disabled for this account
            if let Some(gid) = cg_id {
                if xc_disabled_group_ids.contains(&gid) {
                    continue;
                }
            }

            let stream_model = stream::ActiveModel {
                m3u_account_id: Set(Some(account_id)),'''

if old_xc_cg in content:
    content = content.replace(old_xc_cg, new_xc_cg, 1)
    print("XC Step 2 done: added skip in XC stream loop")
else:
    print("ERROR: XC Step 2 target not found")

with open('src/m3u.rs', 'w', encoding='utf-8') as f:
    f.write(content)
print("Done writing m3u.rs (XC)")
