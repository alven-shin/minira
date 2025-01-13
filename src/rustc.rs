extern crate rustc_driver;
extern crate rustc_hir;
extern crate rustc_interface;
extern crate rustc_middle;
extern crate rustc_span;

use cargo::core::compiler::{CompileMode, Executor};
use cargo::core::manifest::Target;
use cargo::core::package_id::PackageId;
use cargo::core::{Package, Workspace};
use cargo::ops::{self, CompileOptions};
use cargo::util::errors::CargoResult;
use cargo::util::GlobalContext;
use cargo_util::ProcessBuilder;
use rustc_driver::{Callbacks, Compilation, RunCompiler};
use rustc_hir::intravisit::{self, Visitor};
use rustc_hir::{HirId, Pat, PatKind};
use rustc_interface::interface::Compiler;
use rustc_middle::hir::nested_filter::OnlyBodies;
use rustc_middle::ty::{Ty, TyCtxt};
use rustc_span::{FileName, RealFileName, SourceFile, Span};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::env;
use std::ffi::OsString;
use std::io::{BufRead as _, Write as _};
use std::path::Path;
use std::sync::mpsc::{self, Sender};
use std::sync::Arc;
use tokio::task::JoinError;
use tower_lsp::lsp_types::{Position, Range, Url};

pub async fn check_workspace(manifest_path: &Path) -> Result<Vec<Symbol>, JoinError> {
    let path = manifest_path.to_owned();
    tokio::task::spawn_blocking(move || check_workspace_aux(&path)).await
}

fn check_workspace_aux(manifest_path: &Path) -> Vec<Symbol> {
    let context = GlobalContext::default().expect("Failed to create a global context");
    let workspace =
        Workspace::new(manifest_path, &context).expect("Failed to create Cargo workspace");
    let compile_opts = CompileOptions::new(&context, CompileMode::Check { test: false })
        .expect("Failed to create compile options");
    let (tx, rx) = mpsc::channel();
    let custom_exec = Arc::new(CustomExecutor {
        members: workspace.members().map(Package::package_id).collect(),
        tx,
    }) as _;

    ops::compile_with_exec(&workspace, &compile_opts, &custom_exec)
        .expect("Failed to compile the project");

    let mut symbols = Vec::new();
    while let Ok(symbol) = rx.try_recv() {
        symbols.push(symbol);
    }
    symbols
}

/// the first argument argument is automatically discarded, do not manually discard it
pub fn compiler(args: &[String]) {
    RunCompiler::new(args, &mut ThirCallback).run();
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Symbol {
    pub name: String,
    pub ty: String,
    pub range: Range,
    pub uri: Url,
}

struct CustomExecutor {
    members: HashSet<PackageId>,
    tx: Sender<Symbol>,
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
            // call this program again but with the rustc flag to use the embedded compiler
            let mut cmd = cmd.clone();
            let mut new_args = Vec::from([OsString::from("rustc")]);
            new_args.extend(cmd.get_args().cloned());
            cmd.args_replace(&new_args);
            cmd.program(env::current_exe()?);

            let output = cmd.exec_with_output()?;
            for line in output.stdout.lines() {
                self.tx.send(serde_json::from_str(&line?)?)?;
            }
            Ok(())
        } else {
            cmd.exec_with_output().map(drop)
        }
    }
}

struct ThirCallback;

impl Callbacks for ThirCallback {
    fn after_analysis(&mut self, _compiler: &Compiler, tcx: TyCtxt<'_>) -> Compilation {
        tcx.hir()
            .visit_all_item_likes_in_crate(&mut TypeVisitor { tcx });

        Compilation::Continue
    }
}

struct TypeVisitor<'tcx> {
    tcx: TyCtxt<'tcx>,
}

impl<'tcx> TypeVisitor<'tcx> {
    fn span_location(&self, span: Span) -> Option<(Url, Range)> {
        let source_map = self.tcx.sess.source_map();

        let (Some(source), lo_line, lo_col, hi_line, hi_col) =
            source_map.span_to_location_info(span)
        else {
            return None;
        };

        // extract file path details and convert to uri
        let SourceFile {
            name: FileName::Real(RealFileName::LocalPath(path)),
            ..
        } = source.as_ref()
        else {
            return None;
        };
        let Ok(current_dir) = std::env::current_dir() else {
            return None;
        };
        let path = current_dir.join(path);
        let uri = Url::from_file_path(path).ok()?;

        // convert span to range
        #[allow(clippy::cast_possible_truncation)]
        let range = Range {
            start: Position {
                line: lo_line as _,
                character: lo_col as _,
            },
            end: Position {
                line: hi_line as _,
                character: hi_col as _,
            },
        };

        Some((uri, range))
    }

    fn get_type(&self, hir_id: HirId) -> Ty<'tcx> {
        let def_id = hir_id.owner.def_id;
        self.tcx.typeck(def_id).node_type(hir_id)
    }
}

impl<'tcx> Visitor<'tcx> for TypeVisitor<'tcx> {
    type NestedFilter = OnlyBodies;

    fn nested_visit_map(&mut self) -> Self::Map {
        self.tcx.hir()
    }

    fn visit_pat(&mut self, p: &'tcx Pat<'tcx>) -> Self::Result {
        intravisit::walk_pat(self, p);
        if p.span.from_expansion() {
            return;
        }
        if let PatKind::Binding(_, _, ident, _) = p.kind {
            let Some((uri, range)) = self.span_location(p.span) else {
                return;
            };
            let symbol = Symbol {
                name: ident.name.to_string(),
                ty: self.get_type(p.hir_id).to_string(),
                range,
                uri,
            };
            let mut stdout = std::io::stdout().lock();
            stdout
                .write_all(&serde_json::to_vec(&symbol).expect("failed to serialize"))
                .expect("failed to write to stdout");
            stdout.write_all(b"\n").expect("failed to write to stdout");
        }
    }
}
