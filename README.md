# fargo

        fargo v0.1.0
        Fargo is a prototype Fuchsia-specific wrapper around Cargo

        USAGE:
            fargo [FLAGS] [SUBCOMMAND]

        FLAGS:
                --debug-os    Use debug user.bootfs and ssh keys
            -h, --help        Prints help information
            -V, --version     Prints version information
            -v                Print verbose output while performing commands

        SUBCOMMANDS:
            autotest       Auto build and test in Fuchsia device or emulator
            build          Build binary targeting Fuchsia device or emulator
            build-tests    Build tests for Fuchsia device or emulator
            cargo          Run a cargo command for Fuchsia. Use -- to indicate
                           that all following arguments should be passed to
                           cargo.
            configure      Run a configure script for the cross compilation
                           environment
            help           Prints this message or the help of the given
                           subcommand(s)
            pkg-config     Run pkg-config for the cross compilation environment
            restart        Stop all Fuchsia emulators and start a new one
            run            Run binary on Fuchsia device or emulator
            ssh            Open a shell on Fuchsia device or emulator
            start          Start a Fuchsia emulator
            stop           Stop all Fuchsia emulators
            test           Run unit tests on Fuchsia device or emulator

The `fargo-test` directory contains something one can use to test-drive.

## Getting started

Since at the moment fargo requires the FUCHSIA\_ROOT environmental variable be
set to the path to a Fuchsia **release** build, the first step is to build
Fuchsia.

The [Fuchsia Getting
Started](https://fuchsia.googlesource.com/docs/+/HEAD/getting_started.md)
instruction are what you need. Since a release build is what fargo expects to
find you'll want to pass --release to fset. The Rust components that fargo
needs to cross compile are also not built by default, so you'll have to select
something other than the default modules.

If you are planning to use Qemu to run your Fuchsia Rust code, a good choice
for modules is below, in env.sh form or underlying script as one prefers.

    fset x86-64 --release --modules boot_headless,rust

or

    packages/gn/gen.py -m boot_headless,rust --release

What `boot_headless` does in this instance is prevent the user shell from being
launched after boot. Since the user shell requires
[Mozart](https://fuchsia.googlesource.com/mozart), and Mozart has a hard
dependency on the [Vulkan graphics and compute
API](https://www.khronos.org/vulkan), *and* Qemu cannot support Vulkan,
`boot_headless` is pretty much a requirement for Qemu.

Once this build is complete, clone and build fargo.

    git clone https://fuchsia.googlesource.com/fargo
    cd fargo
    cargo install

Fargo uses ssh to communicate between your host computer and either Qemu or a
real device to copy build results and execute them. For Qemu there is a bit of
[tricky set up](https://fuchsia.googlesource.com/magenta/+/master/docs/qemu.md#Enabling-Networking-under-QEMU-x86_64-only) to do.

### Testing if Fargo is working

Now to verify if fargo is working correctly, try starting a fuchsia machine and executing a test.

    fargo start
    cd fargo/fargo-test
    fargo test

If all is well, you should see a successful test pass just as if you had ran cargo test on any other
rust project.

Do note that fargo does not check the fuchsia target env var. Meaning `fargo start` will start a fuchsia
server using x86-64-release unless you pass it the --debug-os option, in which case it will use the
debug build. So make sure you use a fuchsia target you built with the rust module enabled.

Additionally, if you are using qemu you need to enable networking, otherwise fargo won't be able to
copy the binary onto then fuchsia machine to run the tests.

## Getting help

For problems getting the Fuchsia build to complete, the #fuchsia IRC channel on
freenode is the best bet.

For fargo itself, that IRC channel can also work of one of the more Rust-aware
folks happens to be paying attention. More reliable is the
[rust-fuchsia](https://groups.google.com/a/fuchsia.com/forum/#!aboutgroup/rust-fuchsia) Google group.

## Using crates that link with native libraries

Some crates are wrappers around libraries written in other languages. An
example of one such crate is [cairo-rs](https://crates.io/crates/cairo-rs).
Cargo has to know what libraries need to be linked to a binary using such a
crate and where to find those libraries.

Cargo uses build.rs files to locate such libraries. This provides a challenge
for Fargo, as it is unlikely that such build.rs files would know how to cross
compile their libraries for Fuchsia.

Luckily, many of the crates of interest which have native dependencies use
[pkg-config](https://docs.rs/pkg-config/0.3.9/pkg_config/) as one of the ways
to find native dependencies. Fargo provides functions to set up and use a
Fuchsia-specific pkg-config directory.

`fargo pkg-config` is a wrapper around pkg-config that sets the environment so
that only packages found in the Fuchsia-specific pkg-config directory are
visible. This is useful to test if a particular package is already installed.

`fargo configure` is a wrapper around a package's automake configure script.
It takes care of setting up environmental variables such that many automake
based packages will properly cross-compile.

See `scripts/build_cairo_support.sh` for an example of how to use these
functions to build native support.

## Fargo roadmap

The goal is to transition fargo to using something like an SDK instead.
