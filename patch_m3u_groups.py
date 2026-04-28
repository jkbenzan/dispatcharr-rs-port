with open('src/m3u.rs', 'r', encoding='utf-8') as f:
    content = f.read()

# 1. After loading existing_records hash_set, load the disabled group IDs for this account
# and add a cleanup of any existing streams in disabled groups.
# Insert AFTER the hash_set population block (after line with "hash_set.insert(h)")

old_hash_block = '''    let mut hash_set: HashSet<String> = HashSet::new();
    for rec in existing_records {
        if let Some(h) = rec.stream_hash {
            hash_set.insert(h);
        }
    }

    let attr_re'''

new_hash_block = '''    let mut hash_set: HashSet<String> = HashSet::new();
    for rec in existing_records {
        if let Some(h) = rec.stream_hash {
            hash_set.insert(h);
        }
    }

    // Load disabled group IDs for this account so we can skip their streams during ingestion.
    // This is the primary enforcement point - disabled group streams must never enter the DB.
    let disabled_group_ids: HashSet<i64> = if is_initial {
        // On first import all groups are new (enabled by default), nothing to skip yet.
        HashSet::new()
    } else {
        channel_group_m3u_account::Entity::find()
            .filter(channel_group_m3u_account::Column::M3uAccountId.eq(account_id))
            .filter(channel_group_m3u_account::Column::Enabled.eq(false))
            .all(db)
            .await
            .unwrap_or_default()
            .into_iter()
            .map(|m| m.channel_group_id as i64)
            .collect()
    };

    // Purge any existing streams that now belong to a disabled group
    // (handles groups that were disabled since the last sync).
    if !disabled_group_ids.is_empty() {
        let ids: Vec<i64> = disabled_group_ids.iter().cloned().collect();
        let _ = stream::Entity::delete_many()
            .filter(stream::Column::M3uAccountId.eq(account_id))
            .filter(stream::Column::ChannelGroupId.is_in(ids))
            .exec(db)
            .await;
        tracing::info!(
            "[M3U] Purged existing streams for {} disabled groups on account {}",
            disabled_group_ids.len(), account_id
        );
    }

    let attr_re'''

if old_hash_block in content:
    content = content.replace(old_hash_block, new_hash_block, 1)
    print("Step 1 done: added disabled group loading + purge")
else:
    print("ERROR: Step 1 target not found")

# 2. After the M3UFilter Logic block, before hashing/inserting, skip if group is disabled
old_filter_end = '''                if is_excluded || !is_included {
                    // Skip inserting this stream
                    continue;
                }
                // --- End M3UFilter Logic ---'''

new_filter_end = '''                if is_excluded || !is_included {
                    // Skip inserting this stream
                    continue;
                }
                // --- End M3UFilter Logic ---

                // Skip streams whose group is disabled for this account
                if let sea_orm::ActiveValue::Set(Some(cg_id)) = &stream_model.channel_group_id {
                    if disabled_group_ids.contains(cg_id) {
                        continue;
                    }
                }'''

if old_filter_end in content:
    content = content.replace(old_filter_end, new_filter_end, 1)
    print("Step 2 done: added disabled group skip in insertion loop")
else:
    print("ERROR: Step 2 target not found")

with open('src/m3u.rs', 'w', encoding='utf-8') as f:
    f.write(content)
print("Done writing m3u.rs")
