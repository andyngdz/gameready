# DXVK_ASYNC is dead and must not go in launch options

Decision:
Decision:
gameready does not set `DXVK_ASYNC` in any game profile, and no default should add it.

Reason:
The variable belongs to the out-of-tree dxvk-async patch, which GE-Proton dropped in 7-45 (January 2023) once DXVK 2.0 covered the same job with `VK_EXT_graphics_pipeline_library`. GPL is on by default, so there is nothing to enable.

Measured on this machine rather than taken from docs: the string `DXVK_ASYNC` appears 0 times in the DXVK `d3d11.dll` of both Proton Experimental and GE-Proton11-3. Nothing reads it. A user reporting "it works for me" is observing GPL, not the variable.

    strings ~/.steam/steam/compatibilitytools.d/GE-Proton11-3/files/lib/wine/dxvk/x86_64-windows/d3d11.dll | grep -c DXVK_ASYNC

Use the same `strings | grep -c` check before trusting any other `DXVK_*` variable. The 13 that binary does read include `DXVK_HUD`, `DXVK_CONFIG`, `DXVK_SHADER_CACHE`, and `DXVK_LOG_LEVEL`.

On a build that still carries the patch the effect is worse than nothing: geometry draws through walls while shaders compile, which is why it was called a ban risk for competitive games. `games/Deadlock/game.toml` shipped it with a comment recommending it *because* Deadlock is competitive, which was backwards.

`games/Deadlock/game.toml` now carries a comment saying why the variable is absent, so the next person does not add it back as a well-known optimization. Test fixtures use `DXVK_HUD=fps` as the canonical env var example for the same reason.
