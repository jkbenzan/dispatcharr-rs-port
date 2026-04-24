💡 **What:** Eliminated redundant `.find_by_id()` database queries for `m3u_account` fetching.
🎯 **Why:** Inside asynchronous tasks spawned for refreshing M3U accounts and their groups, the code was needlessly re-querying the database to fetch the exact `m3u_account` model that was already available in scope from the preceding query block.
📊 **Measured Improvement:** As there's no live database or benchmark framework attached to test these asynchronous spawn tasks in isolation with real volume, we document the theoretical improvement:
- **Baseline Complexity:** `O(N)` queries per list of `N` accounts inside the loop of `refresh_m3u_all`, plus extra queries during single `add_m3u_account` and `refresh_m3u_account`.
- **Improved Complexity:** The operations inside the task now take `O(1)` memory lookup (using `.clone()`) prior to task spawning, eliminating 1 unnecessary query per account refresh task.
- **Change:** For an account refresh list of `N` length, we save exactly `N` independent SeaORM lookup queries.
