# fargo

Fargo is a prototype Fuchsia-specific wrapper around Cargo.

    USAGE:
        fargo [FLAGS] [SUBCOMMAND]

    FLAGS:
        -h, --help       Prints help information
        -V, --version    Prints version information
        -v               Print verbose output while performing commands

    SUBCOMMANDS:
        build          Build binary targeting Fuchsia device or emulator
        build-tests    Build for Fuchsia device or emulator
        help           Prints this message or the help of the given subcommand(s)
        restart        Stop all Fuchsia emulators and start a new one
        run            Run binary on Fuchsia device or emulator
        ssh            Open a shell on Fuchsia device or emulator
        start          Start a Fuchsia emulator
        stop           Stop all Fuchsia emulators
        test           Run unit tests on Fuchsia device or emulator

The `fargo-test` directory contains something one can use to test-drive.

__At the moment fargo requires the FUCHSIA\_ROOT environmental variable be set to the path to a Fuchsia build.__ The goal is to transition
fargo to using something like an SDK instead.

Currently fargo does not support building artifacts that need additional libraries.
