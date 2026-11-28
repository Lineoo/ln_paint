#[cfg(target_os = "linux")]
fn main() {
    use std::path::PathBuf;

    env_logger::init();

    let matches = clap::Command::new("lndb")
        .about("Mount a LnPaint database as a FUSE filesystem")
        .arg(
            clap::Arg::new("database")
                .help("Path to the .lndb database")
                .required(true)
                .value_parser(clap::value_parser!(PathBuf)),
        )
        .arg(
            clap::Arg::new("mount")
                .help("Mount point")
                .required(true)
                .value_parser(clap::value_parser!(PathBuf)),
        )
        .get_matches();

    let database = matches.get_one::<PathBuf>("database").unwrap();
    let mount = matches.get_one::<PathBuf>("mount").unwrap();

    lnpaint::save::fuse::mount(database, mount).unwrap();
}

#[cfg(not(target_os = "linux"))]
fn main() {
    eprintln!("lndb is only supported on Linux");
}
