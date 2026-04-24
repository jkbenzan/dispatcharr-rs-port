with open("src/main.rs", "r") as f:
    main_rs = f.read()

resolved_main_rs = main_rs.replace("""<<<<<<< HEAD
        .route(
            "/api/channels/streams/by-ids/",
            post(api::get_streams_by_ids),
        )
        .route(
            "/api/channels/streams/filter-options/",
            get(api::get_stream_filter_options),
        )
        .route(
            "/api/channels/dashboard-stats/",
            get(api::get_dashboard_stats),
        )
        .route(
            "/api/channels/streams/",
            get(api::get_streams).post(api::post_stub),
        )
=======
        .route("/api/channels/streams/by-ids/", post(api::get_streams_by_ids))
        .route("/api/channels/streams/filter-options/", get(api::get_stream_filter_options))
        .route("/api/channels/dashboard-stats/", get(api::get_dashboard_stats))
        .route("/api/channels/streams/", get(api::get_streams).post(api::create_stream))
>>>>>>> origin/main""", """        .route("/api/channels/streams/by-ids/", post(api::get_streams_by_ids))
        .route("/api/channels/streams/filter-options/", get(api::get_stream_filter_options))
        .route("/api/channels/dashboard-stats/", get(api::get_dashboard_stats))
        .route("/api/channels/streams/", get(api::get_streams).post(api::create_stream))""")

with open("src/main.rs", "w") as f:
    f.write(resolved_main_rs)
