use ferdinand::ast::Program;
use ferdinand::codegen::Codegen;
use ferdinand::parser;
use ferdinand::resolve::Resolver;
use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

const RUNTIME_H: &str = include_str!("../../runtime/hapsburg_runtime.h");
const RUNTIME_C: &str = include_str!("../../runtime/hapsburg_runtime.c");

struct Options {
    inputs: Vec<PathBuf>,
    output: PathBuf,
    emit_c: bool,
    keep_build_dir: bool,
    show_pedigree: bool,
}

fn parse_args() -> Result<Options, String> {
    let mut inputs = Vec::new();
    let mut output = PathBuf::from("a.out");
    let mut emit_c = false;
    let mut keep_build_dir = false;
    let mut show_pedigree = false;
    let mut args = env::args().skip(1).peekable();
    while let Some(a) = args.next() {
        match a.as_str() {
            "-o" => {
                let v = args.next().ok_or("error: -o requires a filename")?;
                output = PathBuf::from(v);
            }
            "--emit-c" => emit_c = true,
            "--keep-build-dir" => keep_build_dir = true,
            "--show-pedigree" => show_pedigree = true,
            other if other.starts_with('-') => {
                return Err(format!("error: unknown flag '{}'", other))
            }
            other => inputs.push(PathBuf::from(other)),
        }
    }
    if inputs.is_empty() {
        return Err("usage: ferdinand <file1.hb> [file2.hb ...] -o <output>".to_string());
    }
    Ok(Options {
        inputs,
        output,
        emit_c,
        keep_build_dir,
        show_pedigree,
    })
}

fn main() {
    if let Err(e) = run() {
        eprintln!("{}", e);
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let opts = parse_args()?;

    let mut dynasties = Vec::new();
    let mut main_stmts = Vec::new();
    for path in &opts.inputs {
        let src = fs::read_to_string(path)
            .map_err(|e| format!("error: cannot read '{}': {}", path.display(), e))?;
        let prog: Program =
            parser::parse(&src).map_err(|e| format!("{}: {}", path.display(), e))?;
        dynasties.extend(prog.dynasties);
        main_stmts.extend(prog.main_stmts);
    }
    let program = Program {
        dynasties,
        main_stmts,
    };

    let mut resolver = Resolver::new(&program).map_err(|e| e.0)?;
    resolver.check_all().map_err(|e| e.0)?;
    for w in resolver.warnings() {
        eprintln!("ferdinand: {}", w);
    }

    let mut cg = Codegen::new(&resolver);
    let birthed = cg.discover_birthed(&program);
    for class in &birthed {
        if opts.show_pedigree {
            let resolved = resolver.resolve(class).map_err(|e| e.0)?;
            eprintln!(
                "ferdinand: pedigree of '{}': {}",
                class,
                resolved.linearization.join(" -> ")
            );
        }
        cg.gen_class(class)
            .map_err(|e| format!("ferdinand: {}", e))?;
    }
    let main_fn = cg
        .gen_main(&program)
        .map_err(|e| format!("ferdinand: {}", e))?;

    let mut c_src = String::new();
    c_src.push_str("#include \"hapsburg_runtime.h\"\n\n");
    c_src.push_str("/* ---- flattened dynasty structs ---- */\n");
    c_src.push_str(&cg.structs);
    c_src.push_str(
        "\n/* ---- method table (one direct function per resolved method per class) ---- */\n",
    );
    c_src.push_str(&cg.func_decls);
    c_src.push('\n');
    c_src.push_str(&cg.func_impls);
    c_src.push_str("\n/* ---- entry point ---- */\n");
    c_src.push_str(&main_fn);

    let build_dir = env::temp_dir().join(format!("ferdinand-build-{}", std::process::id()));
    fs::create_dir_all(&build_dir).map_err(|e| format!("error: cannot create build dir: {}", e))?;
    let c_path = build_dir.join("program.c");
    let h_path = build_dir.join("hapsburg_runtime.h");
    let rc_path = build_dir.join("hapsburg_runtime.c");
    fs::write(&c_path, &c_src).map_err(|e| e.to_string())?;
    fs::write(&h_path, RUNTIME_H).map_err(|e| e.to_string())?;
    fs::write(&rc_path, RUNTIME_C).map_err(|e| e.to_string())?;

    if opts.emit_c {
        println!("{}", c_src);
    }

    let status = Command::new("cc")
        .arg("-O2")
        .arg("-o")
        .arg(&opts.output)
        .arg(&c_path)
        .arg(&rc_path)
        .arg("-I")
        .arg(&build_dir)
        .status()
        .map_err(|e| format!("error: failed to invoke cc: {}", e))?;

    if !opts.keep_build_dir {
        let _ = fs::remove_dir_all(&build_dir);
    }

    if !status.success() {
        return Err("error: cc failed to compile the generated program".to_string());
    }

    eprintln!(
        "ferdinand: compiled {} dynast{} into '{}'",
        birthed.len(),
        if birthed.len() == 1 { "y" } else { "ies" },
        opts.output.display()
    );
    Ok(())
}
