# Rust for Playdate

You've stumbled across a barely functional, wildly incomplete and basically undocumented Rust crate whose aim is to let you write games for the [Playdate handheld gaming system](https://play.date) in [Rust](https://www.rust-lang.org).

This software is not sponsored or supported by Panic.

## Installation

To use this crate, you'll also need to install the [crank command line tool](https://github.com/rtsuk/crank).

From the crankstart directory where you found this README,

    crank run --release --example hello_world

Should launch the simulator and load in the hello_world sample.

If you have a device attached to your desktop,

    crank run --release --example hello_world --device

Should launch the hello_world sample on the device.

For the sprite_game example one needs to copy the images folder from `"PlaydateSDK/C_API/Examples/Sprite Game/Source/images"` to `"sprite_game_images"`.

## Your Own Project

Using this system for your own project requires some setup:

1. Follow the setup for `crank` with Rust nightly's `no_std` support.
2. Start a new rust library project with `cargo new --lib project_name`
3. `git clone git@github.com:pd-rs/crankstart.git` at the same depth as your new project.
4. Go into the new project, and add the following to your `Cargo.toml`:

```toml
[package.metadata.cargo-xbuild]
memcpy = false
sysroot_path = "target/sysroot"
panic_immediate_abort = false

[profile.dev]
panic = "abort"
opt-level = 'z'
lto = true

[profile.release]
panic = "abort"
opt-level = 'z'
lto = true

[lib]
crate-type = ["staticlib", "cdylib"]

[dependencies]
crankstart = { path = "../crankstart" }
crankstart-sys = { path = "../crankstart/crankstart-sys" }
anyhow = { version = "1.0.31", default-features = false }
euclid = { version = "0.22.9", default-features = false, features = [ "libm" ] }
hashbrown = "0.14.0"

[dependencies.cstr_core]
version = "=0.1.2"
default-features = false
features = [ "alloc" ]
```

5. Add a `Crank.toml` at the same level as your `Cargo.toml`, with this minimum:

```toml
[[target]]
    name = "project_name"
    assets = [
    ]
```

`assets` should be a list of paths to any/all assets you need copied into your project, such as sprites, music, etc.

6. Inside your `lib.rs`, you only need to implement the `crankstart::Game` trait to your game's core state struct, then call `crankstart::crankstart_game!` on that struct. See the `examples` folder for examples.
7. To run the project, from its root, you should now be able to `crank run` successfully!

If you want an example of an independent project following these conventions, go check out [Nine Lives](https://github.com/bravely/nine_lives).

## Updating Bindings

If there's a newer [Playdate SDK](https://play.date/dev/) available that updates the C API, the crankstart bindings should be updated to match.
Here's a guide.

1. Install [the dependencies for bindgen](https://rust-lang.github.io/rust-bindgen/requirements.html).
2. Install [bindgen-cli](https://rust-lang.github.io/rust-bindgen/command-line-usage.html).
3. Install the gcc-arm-none-eabi toolchain, either [manually](https://developer.arm.com/Tools%20and%20Software/GNU%20Toolchain) or through a system package, which may also be named something like "cross-arm-none-eabi-gcc".
4. On Linux, install the 32-bit glibc development package, which will be called something like `glibc-devel-32bit`.
5. Install the new [Playdate SDK](https://play.date/dev/), and if it's not at the default MacOS path, set `PLAYDATE_SDK_PATH` to where you unzipped it.  (This should be the directory that contains `C_API`, `CoreLibs`, etc.)
6. Run `./scripts/generate_bindings.sh`
7. Inspect the changes to `crankstart-sys/src/bindings_*` - they should reflect the updates to the Playdate C API.  If nothing changed, double-check that the C API actually changed and not just the Lua API.
8. Submit a PR with the changes :)
