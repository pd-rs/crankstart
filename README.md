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
euclid = { version = "0.20.13", default-features = false, features = [ "libm" ] }
hashbrown = "0.7.2"
heapless = "0.5.5"

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
