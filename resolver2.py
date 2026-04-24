with open("src/entities/mod.rs", "r") as f:
    mod_rs = f.read()

resolved_mod_rs = mod_rs.replace("""<<<<<<< HEAD
pub mod vod_m3uvodcategoryrelation;
pub mod vod_movie;
pub mod vod_series;
=======
pub mod vod_m3uepisoderelation;
pub mod core_useragent;
pub mod core_systemnotification;
pub mod core_notificationdismissal;
>>>>>>> origin/main""", """pub mod vod_m3uvodcategoryrelation;
pub mod vod_movie;
pub mod vod_series;
pub mod vod_m3uepisoderelation;
pub mod core_useragent;
pub mod core_systemnotification;
pub mod core_notificationdismissal;""")

with open("src/entities/mod.rs", "w") as f:
    f.write(resolved_mod_rs)
