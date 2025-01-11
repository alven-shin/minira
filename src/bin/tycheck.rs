#![feature(rustc_private)]

extern crate rustc_driver;
extern crate rustc_interface;
extern crate rustc_middle;

use cargo::core::compiler::{CompileMode, Executor};
use cargo::core::manifest::Target;
use cargo::core::package_id::PackageId;
use cargo::core::{Package, Workspace};
use cargo::ops::{self, CompileOptions};
use cargo::util::errors::CargoResult;
use cargo::util::GlobalContext;
use cargo_util::ProcessBuilder;
use rustc_driver::{Callbacks, Compilation, RunCompiler};
use rustc_interface::interface::Compiler;
use rustc_middle::ty::TyCtxt;
use std::collections::HashSet;
use std::env;
use std::ffi::OsString;
use std::path::PathBuf;
use std::sync::Arc;

fn main() {
    let mut args = std::env::args().skip(1).peekable(); // skip the first arg, which is the executable name

    if let Some("rustc") = args.peek().map(String::as_str) {
        let args = args.collect::<Vec<_>>();
        RunCompiler::new(&args, &mut ThirCallback).run();
    } else {
        let context = GlobalContext::default().expect("Failed to create a global context");
        let manifest_path = PathBuf::from(
            env::var("CARGO_MANIFEST_DIR").expect("failed to get manifest directory"),
        )
        .join("Cargo.toml");
        let workspace =
            Workspace::new(&manifest_path, &context).expect("Failed to create Cargo workspace");

        let compile_opts = CompileOptions::new(&context, CompileMode::Check { test: false })
            .expect("Failed to create compile options");

        let custom_exec = Arc::new(CustomExecutor {
            members: workspace.members().map(Package::package_id).collect(),
        }) as _;
        ops::compile_with_exec(&workspace, &compile_opts, &custom_exec)
            .expect("Failed to compile the project");
    }
}

struct CustomExecutor {
    members: HashSet<PackageId>,
}

impl Executor for CustomExecutor {
    fn exec(
        &self,
        cmd: &ProcessBuilder,
        id: PackageId,
        _target: &Target,
        _mode: CompileMode,
        _on_stdout_line: &mut dyn FnMut(&str) -> CargoResult<()>,
        _on_stderr_line: &mut dyn FnMut(&str) -> CargoResult<()>,
    ) -> CargoResult<()> {
        if self.members.contains(&id) {
            let mut cmd = cmd.clone();
            let mut new_args = Vec::from([OsString::from("rustc")]);
            new_args.extend(cmd.get_args().cloned());
            cmd.args_replace(&new_args);
            cmd.program(env::current_exe().expect("failed to get current executable path"));
            cmd.exec()
        } else {
            cmd.exec()
        }
    }
}

struct ThirCallback;

impl Callbacks for ThirCallback {
    fn after_analysis(&mut self, _compiler: &Compiler, tcx: TyCtxt<'_>) -> Compilation {
        for item in tcx.hir().items() {
            dbg!(item);
        }
        Compilation::Continue
    }
}
