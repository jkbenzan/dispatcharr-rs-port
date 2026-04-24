with open("src/vod.rs", "r") as f:
    vod_rs = f.read()

resolved_vod_rs = vod_rs.replace("""<<<<<<< HEAD
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QuerySelect};
=======
use sea_orm::{EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, QuerySelect};
>>>>>>> origin/main""", """use sea_orm::{ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, QuerySelect};""")

with open("src/vod.rs", "w") as f:
    f.write(resolved_vod_rs)
