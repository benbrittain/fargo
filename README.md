# fargo

Fargo is a prototype Fuchsia-specific wrapper around Cargo.

    fargo v0.1.0

    USAGE:
        fargo [FLAGS] [SUBCOMMAND]

    FLAGS:
            --debug-os    Use debug user.bootfs and ssh keys
        -h, --help        Prints help information
        -V, --version     Prints version information
        -v                Print verbose output while performing commands

    SUBCOMMANDS:
        build          Build binary targeting Fuchsia device or emulator
        build-tests    Build for Fuchsia device or emulator
        cargo          Run a cargo command for Fuchsia. Use -- to indicate that all following
                       arguments should be passed to cargo.
        help           Prints this message or the help of the given subcommand(s)
        restart        Stop all Fuchsia emulators and start a new one
        run            Run binary on Fuchsia device or emulator
        ssh            Open a shell on Fuchsia device or emulator
        start          Start a Fuchsia emulator
        stop           Stop all Fuchsia emulators
        test           Run unit tests on Fuchsia device or emulator

The `fargo-test` directory contains something one can use to test-drive.

## Getting Started

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

### Testing If Fargo Is working

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

## Getting Help

For problems getting the Fuchsia build to complete, the #fuchsia IRC channel on
freenode is the best bet.

For fargo itself, that IRC channel can also work of one of the more Rust-aware
folks happens to be paying attention. More reliable is the
[rust-fuchsia](https://groups.google.com/a/fuchsia.com/forum/#!aboutgroup/rust-fuchsia) Google group.

## Fargo Roadmap

The goal is to transition fargo to using something like an SDK instead.

Currently fargo does not support building artifacts that need additional
libraries.
