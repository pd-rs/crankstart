# Rust for Playdate

You've stumbled across a barely functional, wildly incomplete and basically undocumented Rust crate whose aim is to let you write games for the [Playdate handheld gaming system](https://play.date) in [Rust](https://www.rust-lang.org).

This software is not sponsored or supported by Panic.

## Installation

To use this crate, you'll also need to install the [crank command line tool](https://github.com/rtsuk/crank).

From the crankstart directory where you found this README,

    crank run --release --example hello_world

Should launch the simulator and load in the hello_world sample.

If you have a device attached to your Mac,

    crank build --release --example hello_world --device

Should load but not launch the hello_world sample on the device.

For the sprite_game example one needs to copy the images folder from `"PlaydateSDK/C_API/Examples/Sprite Game/Source/images"` to `"sprite_game_images"`.
