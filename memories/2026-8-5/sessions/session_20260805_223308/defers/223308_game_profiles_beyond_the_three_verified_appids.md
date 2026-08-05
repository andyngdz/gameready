# Game profiles beyond the three verified appids

Deferred:
M4 part 1 shipped only 3 game profiles (Deadlock 1422450, Cyberpunk 2077 1091500, Slay the Spire 2 2868840). The plan calls for 8-10.

The other profiles were not written because their Steam appids could not be verified. A wrong appid silently applies one game's settings to another and no code path catches it; `embedded_test.rs` asserts the three known ids but cannot vouch for a new one.

Unblock: verify each candidate appid against a real Steam library (`~/.steam/steam/steamapps/appmanifest_<id>.acf` carries the name) or the Steam store, then add `games/<Name>/game.toml` and extend the assertion in `crates/gameready-core/src/infra/games/embedded_test.rs`.
