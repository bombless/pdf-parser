use clap::{arg, command, ArgAction, ArgMatches};

pub fn parse_options() -> ArgMatches {
    let matches = command!() // requires `cargo` feature
        .arg(arg!([name] "Optional name to operate on"))
        .arg(
            arg!(-t --texts "Print texts")
            .required(false)
            .action(ArgAction::SetTrue)
        )
        .arg(
            arg!(-o --operations ... "Print operations")
            .required(false)
            .action(ArgAction::SetTrue)
        )
        .arg(
            arg!(-g --grand_kids ... "Print grand kids")
            .required(false)
            .action(ArgAction::SetTrue)
        )
        .arg(
            arg!(-m --meta ... "Print meta")
            .required(false)
            .action(ArgAction::SetTrue)
        )
        .arg(
            arg!(-c --cmap ... "Print CMaps")
            .required(false)
            .action(ArgAction::SetTrue)
        )
        .get_matches();
    matches
}
