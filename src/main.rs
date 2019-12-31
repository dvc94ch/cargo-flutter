use cargo::util::Config;
use cargo_flutter::package::appimage::AppImage;
use cargo_flutter::{Build, Cargo, Engine, Error, Flutter, Package, TomlConfig};
use clap::{App, AppSettings, Arg, SubCommand};
use std::{env, str};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let app_matches = App::new("cargo-flutter")
        .bin_name("cargo")
        .subcommand(
            SubCommand::with_name("flutter")
                .setting(AppSettings::TrailingVarArg)
                .version(env!("CARGO_PKG_VERSION"))
                .author("flutter-rs")
                .about("Provides a smooth experience for developing flutter-rs apps.")
                .arg(
                    Arg::with_name("format")
                        .short("f")
                        .long("format")
                        .value_name("FORMAT")
                        .takes_value(true)
                        .help("Packaging format"),
                )
                .arg(
                    Arg::with_name("no-bundle")
                        .long("no-bundle")
                        .help("Skips running flutter bundle"),
                )
                .arg(
                    Arg::with_name("cargo-args")
                        .value_name("CARGO_ARGS")
                        .takes_value(true)
                        .required(true)
                        .multiple(true),
                ),
        )
        .get_matches();

    let matches = if let Some(matches) = app_matches.subcommand_matches("flutter") {
        matches
    } else {
        return Err(Error::NotCalledWithCargo.into());
    };

    let cargo_args: Vec<&str> = matches
        .values_of("cargo-args")
        .expect("cargo-args to not be null")
        .collect();
    let cargo_config = Config::default()?;
    let cargo = Cargo::new(&cargo_config, cargo_args)?;
    let build = if cargo.release() {
        Build::Release
    } else {
        Build::Debug
    };
    let config = TomlConfig::load(&cargo)?;
    let metadata = config.metadata();
    let flutter = Flutter::new()?;
    let engine_version = metadata
        .engine_version()
        .unwrap_or_else(|| flutter.engine_version().unwrap());
    let engine = Engine::new(engine_version, cargo.triple()?, build);
    engine.download();

    if !matches.is_present("no-bundle") {
        println!("flutter build bundle");
        flutter.bundle(&cargo, build)?;
    }

    let flutter_asset_dir = cargo.build_dir().join("flutter_assets");
    let engine_path = engine.engine_path();

    match cargo.cmd() {
        "build" => {
            cargo.build(&engine_path)?;

            if let Some(format) = matches.value_of("format") {
                let mut package = Package::new(&config.package.name);
                package.add_bin(cargo.build_dir().join(&config.package.name));
                package.add_lib(engine_path);
                package.add_asset(flutter_asset_dir);
                match format {
                    "appimage" => {
                        let builder = AppImage::new(metadata.appimage.unwrap_or_default());
                        builder.build(&cargo, &package)?;
                    }
                    _ => Err(Error::FormatNotSupported)?,
                }
            }
        }
        "run" => {
            std::env::set_var("FLUTTER_ASSET_DIR", &flutter_asset_dir);
            let debug_uri = cargo.run(&engine.engine_path())?;
            log::info!("Observatory at {}", debug_uri);
            flutter.attach(cargo.workspace(), &debug_uri)?;
        }
        _ => cargo.build(&engine_path)?,
    }

    Ok(())
}
