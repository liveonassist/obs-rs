
init:
  git submodule update --init
  cargo build --all


[doc("ex: just example avatar-plugin avatar_render --features wayland")]
example package example_name *ARGS:
  cargo run --release --package {{package}} --example {{example_name}} {{ARGS}}

[doc("ex: just test avatar-plugin --features wayland")]
test package *ARGS:
  cargo test --package={{package}} {{ARGS}}

[doc("ex: just build avatar-plugin --features wayland")]
build plugin *ARGS:
  cargo build --release --package={{plugin}} {{ARGS}}

[doc("build the workspace against OBS v30 (use inside `nix develop .#obs-v30`)")]
build-v30 *ARGS:
  cargo build --workspace --no-default-features --features obs-30 {{ARGS}}

[doc("build the workspace against OBS v31 (use inside `nix develop .#obs-v31`)")]
build-v31 *ARGS:
  cargo build --workspace --no-default-features --features obs-31 {{ARGS}}

[doc("build the workspace against OBS v32 (default; works in the default shell)")]
build-v32 *ARGS:
  cargo build --workspace --no-default-features --features obs-32 {{ARGS}}

@obs-test:
  env OBS_PLUGINS_PATH=$(pwd)/target/release \
    OBS_PLUGINS_DATA_PATH=$(pwd)/target/release \
    OBS_WINDOW_TITLE="DebugApplication" \
    obs

