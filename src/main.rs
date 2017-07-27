#[macro_use] extern crate clap;
extern crate tempdir;
extern crate rand;
extern crate base32;

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

fn cmd_run(image: &str, rootless: bool, volumes: &[(&str, &str)]) -> Result<Option<i32>, CmdErr> {
    if volumes.len() > 0 {
        return Err(CmdErr { message: "Volume bind-mounts are not supported yet".to_owned(), secondary: None });
    }

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

    // Support for .tar.gz images: un-tar them in the temporary directory
    if image_metadata.file_type().is_file() {
        image_path.file_name().expect("Filename is expected for a file")
            .to_string_lossy().ends_with(".tar.gz").check("File is not a tarball")?;
        image_dir_path = tmp_dir.path().join("image");
        fs::create_dir(&image_dir_path).check("Internal error")?;
        eprintln!("info: Uncompressing image into {}", image_dir_path.to_string_lossy());
        let tar_output = ::std::process::Command::new("tar")
            .arg("--strip-components")
            .arg("1")
            .arg("-C")
            .arg(&image_dir_path)
            .arg("-xf")
            .arg(image_path)
            .output()
            .check("Cannot execute tar")?;
        if !tar_output.status.success() {
            return Err(CmdErr {
                message: "Failed to un-tar image".to_owned(),
                secondary: Some(String::from_utf8_lossy(&tar_output.stderr).into_owned()),
            });
        }
    }

    let bundle = "bundle";

    let bundle_path = tmp_dir.path().join(bundle);
    eprintln!("info: Unpacking image to {}", bundle_path.to_string_lossy());

    let mut umoci_command = ::std::process::Command::new("umoci");
    umoci_command
        .current_dir(tmp_dir.path())
        .arg("unpack")
        .arg("--image")
        .arg(&image_dir_path.file_name().expect("The image directory must have a last component"));
    if rootless {
        umoci_command.arg("--rootless");
    }
    umoci_command.arg(bundle);

    let umoci_output = umoci_command.output()
        .check("Cannot run umoci")?;

    if !umoci_output.status.success() {
        return Err(CmdErr {
            message: "Failed to unpack image with umoci".to_owned(),
            secondary: Some(String::from_utf8_lossy(&umoci_output.stderr).into_owned()),
        });
    }

    let container_name = {
        use rand::Rng;
        let bytes: [u8; 4] = unsafe { ::std::mem::transmute(rand::thread_rng().gen::<u32>().to_be()) };
        ::base32::encode(::base32::Alphabet::RFC4648 { padding: false }, &bytes[..])
    };

    let mut runc_command = ::std::process::Command::new("runc");
    runc_command
        .current_dir(bundle_path);
    if rootless {
        let root_dir_path = tmp_dir.path().join("root");
        fs::create_dir(&root_dir_path).check("Internal error")?;
        runc_command
            .arg("--root")
            .arg(&root_dir_path);
    }
    runc_command
        .arg("run")
        .arg(&container_name);

    eprintln!("info: Starting container with name {}", &container_name);
    eprintln!("info: If runc is successful, the next line will be inside the container");

    let runc_status = runc_command.status().check("Failed to start the container")?;

    eprintln!("info: Container exited with {}", runc_status);

    Ok(runc_status.code())
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
    let app = app_from_crate!()
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
                         .multiple(true)));
    let matches = app.clone().get_matches();

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
            let mut out = ::std::io::stdout();
            app.write_help(&mut out).ok().expect("error: Failed to print usage to stdout");
            println!();
            ::std::process::exit(-1);
        },
    };
    match result {
        Ok(exit_code) => ::std::process::exit(exit_code.unwrap_or(-1)),
        Err(err) => {
            eprintln!("error: {}", err.message);
            if let Some(secondary) = err.secondary {
                eprintln!("error: {}", secondary);
            }
            eprintln!("error: Run with --help for usage");
        },
    }
}
