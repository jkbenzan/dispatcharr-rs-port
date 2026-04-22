sed -i '/<<<<<<< Updated upstream/d' src/proxy.rs
sed -i '/=======/,/>>>>>>> Stashed changes/d' src/proxy.rs
git add src/proxy.rs
