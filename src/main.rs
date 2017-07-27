#[macro_use]
extern crate clap;
extern crate tempdir;

use std::fs;

use clap::{Arg, SubCommand};

use tempdir::TempDir;

struct CmdErr {
    message: String,
    secondary: Option<String>,
}

trait ToCmdErr<T> {
    fn check(self, message: &str) -> Result<T, CmdErr>;
}

impl<T, E: ::std::fmt::Display> ToCmdErr<T> for Result<T, E> {
    fn check(self, message: &str) -> Result<T, CmdErr> {
        self.map_err(|e| CmdErr {
            message: message.to_owned(),
            secondary: Some(format!("{}", e)),
        })
    }
}

impl<T> ToCmdErr<T> for Option<T> {
    fn check(self, message: &str) -> Result<T, CmdErr> {
        match self {
            Some(t) => Ok(t),
            None => Err(CmdErr {
                message: message.to_owned(),
                secondary: None,
            }),
        }
    }
}

impl ToCmdErr<()> for bool {
    fn check(self, message: &str) -> Result<(), CmdErr> {
        if self {
            Ok(())
        } else {
            Err(CmdErr {
                message: message.to_owned(),
                secondary: None,
            })
        }
    }
}

fn cmd_run(image: &str, rootless: bool, volumes: &[(&str, &str)]) -> Result<(), CmdErr> {
    let tmp_dir = TempDir::new("shipc").check("Cannot create temporary directory")?;
    eprintln!("info: Temporary directory: {:?}", tmp_dir.path());
    eprint!("info: Running image {}", image);
    if rootless {
        eprint!("info: in rootless mode");
    }
    eprintln!();
    for &(ref orig, ref dest) in volumes {
        eprintln!("info:   with volume {}:{}", orig, dest);
    }
    let image_path = ::std::path::Path::new(image);
    let image_metadata = fs::metadata(image).check("Invalid image path")?;
    let mut image_dir_path = image_path.to_path_buf();
    if image_metadata.file_type().is_file() {
        image_path.file_name().expect("Filename is expected for a file")
            .to_string_lossy().ends_with(".tar.gz").check("File is not a tarball")?;
        image_dir_path = tmp_dir.path().join("image");
        fs::create_dir(&image_dir_path).check("Internal error")?;
        eprintln!("info: Uncompressing image into {}", image_dir_path.to_string_lossy());
        let tar_output = ::std::process::Command::new("tar")
            .arg("-C")
            .arg(image_dir_path)
            .arg("-xf")
            .arg(image_path)
            .output()
            .check("Cannot un-tar the image")?;
    }
    ::std::mem::drop(tmp_dir);
    unimplemented!();
    Ok(())
}

fn fail(message: &str) {
    eprintln!("error: {}", message);
    eprintln!("error: Run with --help for usage");
    ::std::process::exit(-1);
}

fn check(condition: bool, message: &str) {
    if !condition {
        fail(message);
    }
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

    let result = match matches.subcommand() {
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
            cmd_run(image, rootless, &volumes[..])
        },
        _ => {
            println!("{}", matches.usage());
            ::std::process::exit(-1);
        },
    };
    match result {
        Ok(()) => (),
        Err(err) => {
            eprintln!("error: {}", err.message);
            if let Some(secondary) = err.secondary {
                eprintln!("error: {}", secondary);
                eprintln!("error: Run with --help for usage");
            }
        },
    }
}
