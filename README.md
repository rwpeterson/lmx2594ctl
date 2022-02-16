# lmx2594ctl

Initialization of the LMX2594 over SPI, performed by Raspberry Pi Pico.

See the [rp2040-project-template](https://github.com/rp-rs/rp2040-project-template) for
basic walkthroughs of how to do different tasks. This repo is adapted from the template.

## Prerequisites

Get [Rust](https://rustup.rs), then:

```sh
rustup target install thumbv6m-none-eabi
cargo install flip-link
cargo install elf2uf2-rs --locked
```

## Running

Cargo is configured in this repo to use elf2uf2-rs to flash the Pico in the simplest way.
Plug in the Pico to the computer while holding down the BOOTSEL switch. It will appear as
a USB drive (automounted on Windows; on Linux, mount it yourself). Then, `cargo run --release`
will automatically call elf2uf2-rs to flash the program onto the Pico and start running it.