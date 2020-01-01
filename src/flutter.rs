use crate::cargo::Cargo;
use crate::engine::Build;
use crate::error::Error;
use std::path::{Path, PathBuf};
use std::process::Command;

pub struct Flutter {
    root: PathBuf,
}

impl Flutter {
    pub fn new() -> Result<Self, Error> {
        let root = if let Ok(root) = std::env::var("FLUTTER_ROOT") {
            PathBuf::from(root)
        } else {
            let flutter = which::which("flutter").expect("flutter not found");
            let flutter = std::fs::canonicalize(flutter)?;
            flutter
                .parent()
                .ok_or(Error::FlutterNotFound)?
                .parent()
                .ok_or(Error::FlutterNotFound)?
                .to_owned()
        };
        Ok(Flutter { root })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn engine_version(&self) -> Result<String, Error> {
        let version = if let Ok(v) = std::env::var("FLUTTER_ENGINE_VERSION") {
            v
        } else {
            let path = self
                .root
                .join("bin")
                .join("internal")
                .join("engine.version");
            std::fs::read_to_string(path).map(|v| v.trim().to_owned())?
        };
        Ok(version)
    }

    pub fn bundle(&self, cargo: &Cargo, build: Build) -> Result<(), Error> {
        let flag = match build {
            Build::Debug => "--debug",
            Build::Release => "--release",
            Build::Profile => "--profile",
        };

        let status = Command::new("flutter")
            .current_dir(cargo.workspace().root())
            .arg("build")
            .arg("bundle")
            .arg(flag)
            .arg("--track-widget-creation")
            .arg("--asset-dir")
            .arg(cargo.build_dir().join("flutter_assets"))
            .arg("--depfile")
            .arg(cargo.build_dir().join("snapshot_blob.bin.d"))
            .status()
            .expect("flutter build bundle");
        if status.code() != Some(0) {
            return Err(Error::FlutterError);
        }
        Ok(())
    }

    pub fn attach(&self, cargo: &Cargo, debug_uri: &str) -> Result<(), Error> {
        let debug_uri = format!("--debug-uri={}", debug_uri);
        let status = Command::new("flutter")
            .current_dir(cargo.workspace().root())
            .arg("attach")
            .arg("--device-id=flutter-tester")
            .arg(debug_uri)
            .status()
            .expect("Success");
        if status.code() != Some(0) {
            return Err(Error::FlutterError);
        }
        Ok(())
    }

    pub fn aot(&self, cargo: &Cargo, engine_path: &Path) -> Result<(), Error> {
        let root = cargo.workspace().root();
        let build_dir = cargo.build_dir();
        let engine_dir = engine_path.parent().unwrap();
        let snapshot = build_dir.join("kernel_snapshot.dill");
        let status = Command::new(engine_dir.join("dart"))
            .current_dir(root)
            .arg(engine_dir.join("gen").join("frontend_server.dart.snapshot"))
            .arg("--sdk-root")
            .arg(engine_dir.join("flutter_patched_sdk"))
            .arg("--target=flutter")
            .arg("--aot")
            .arg("--tfa")
            .arg("-Ddart.vm.product=true")
            .arg("--packages")
            .arg(".packages")
            .arg("--output-dill")
            .arg(&snapshot)
            .arg(root.join("lib").join("main.dart"))
            .status()
            .expect("Success");
        if status.code() != Some(0) {
            return Err(Error::FlutterError);
        }
        let status = Command::new(engine_dir.join("gen_snapshot"))
            .current_dir(root)
            .arg("--causal_async_stacks")
            .arg("--deterministic")
            .arg("--snapshot_kind=app-aot-elf")
            .arg("--strip")
            .arg(format!("--elf={}", build_dir.join("app.so").display()))
            .arg(&snapshot)
            .status()
            .expect("Success");
        if status.code() != Some(0) {
            return Err(Error::FlutterError);
        }
        Ok(())
    }
}
