#[macro_use]
extern crate clap;
use clap::{Arg, SubCommand};

fn check(condition: bool, message: &str) {
    if !condition {
        eprintln!("error: {}", message);
        eprintln!("error: Run with --help for usage");
        ::std::process::exit(-1);
    }
}

fn cmd_run(image: &str, rootless: bool, volumes: &[(&str, &str)]) {
    eprint!("Running image {}", image);
    if rootless {
        eprint!(" in rootless mode");
    }
    eprintln!();
    for &(ref orig, ref dest) in volumes {
        eprintln!("  with volume {}:{}", orig, dest);
    }
    unimplemented!();
}

fn main() {
    let matches = app_from_crate!()
        .about("Unpack and run oci images")
        .subcommand(SubCommand::with_name("run")
                    .about("Run an oci image")
                    .arg(Arg::with_name("image")
                         .required(true))
                    .arg(Arg::with_name("rootless")
                         .help("Run in rootless mode")
                         .long("rootless"))
                    .arg(Arg::with_name("volume")
                         .help("Rebind volumes")
                         .long("volume")
                         .short("v")
                         .takes_value(true)
                         .value_name("VOLUME")
                         .multiple(true))
                    )
        .get_matches();

    match matches.subcommand() {
        ("run", Some(sub_matches)) => {
            let rootless: bool = sub_matches.is_present("rootless");
            let image = sub_matches.value_of("image").unwrap();
            let volumes = sub_matches.values_of("volume").map(|vol_strs| {
                vol_strs.map(|s| {
                    let vol_arr: Vec<&str> = s.split(":").collect();
                    check(vol_arr.len() == 2, "Volume should be origin:destination");
                    let orig = vol_arr[0];
                    let dest = vol_arr[1];
                    (orig, dest)
                }).collect::<Vec<_>>()
            }).unwrap_or(Vec::new());
            cmd_run(image, rootless, &volumes[..]);
        },
        _ => println!("{}", matches.usage()),
    }
}
