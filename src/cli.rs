use clap::{arg, command, ArgAction, ArgMatches};

pub fn parse_options() -> ArgMatches {
    let matches = command!() // requires `cargo` feature
        .arg(arg!(FILE: "File to parse"))
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
        .arg(
            arg!(-p --pages ... "Print pages")
            .required(false)
            .action(ArgAction::SetTrue)
        )
        .arg(
            arg!(-f --first_page ... "Print pages")
            .required(false)
            .action(ArgAction::SetTrue)
        )
        .arg(
            arg!(-b --babel ... "Print babel dictionary")
            .required(false)
            .action(ArgAction::SetTrue)
        )
        .arg(
            arg!(-n --nth <n> "Print object")
            .required(false)
        )
        .arg(
            arg!(-a --all <n> "Print all objects")
            .required(false)
            .action(ArgAction::SetTrue)
        )
        .arg(
            arg!(-r --references <n> "Print all references")
            .required(false)
            .action(ArgAction::SetTrue)
        )
        .get_matches();
    matches
}
