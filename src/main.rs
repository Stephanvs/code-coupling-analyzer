use clap::Parser;
use colored::Colorize;
use std::{
    fs,
    io::Read,
    path::{Path, PathBuf},
};
use tree_sitter::Node;

#[derive(Parser)]
struct Args {
    #[arg(short, long)]
    source_folder: Option<PathBuf>,
}

fn main() {
    let args = Args::parse();
    let source_folder = args.source_folder.unwrap_or_else(|| PathBuf::from("."));

    if !Path::exists(&source_folder) {
        panic!("Ensure that the source folder exists.");
    }

    println!(
        "Scanning sources in path: {:?}",
        source_folder.canonicalize().unwrap()
    );

    visit_dirs(&source_folder).expect("Failed processing source files");
}

fn visit_dirs(path: &Path) -> std::io::Result<()> {
    if path.is_dir() {
        for entry in fs::read_dir(path)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_dir() {
                visit_dirs(&path)?;
            } else {
                analyze_file(&path)?;
            }

            // if path.is_dir() {
            //     visit_dirs(&path)?;
            // } else if path.extension().and_then(|ext| ext.to_str()) == Some("rs") {
            //     analyze_source_file(path)?;
            // }
        }
    } else {
        analyze_file(&path)?;
    }

    Ok(())
}

fn analyze_file(path: &Path) -> std::io::Result<()> {
    let mut parser = tree_sitter::Parser::new();
    let ext = path.extension().and_then(|ext| ext.to_str());

    match ext {
        Some("rs") => {
            let _file_type = ext;

            parser
                .set_language(&tree_sitter_rust::LANGUAGE.into())
                .expect("Failed to set language");

            analyze_source_file(path.to_path_buf(), &mut parser)?;
            Ok(())
        }
        // Some(_) => {
        //     // Skip non-Rust files
        //     println!("Skipping non-Rust file: {:?}", path);
        //     Ok(())
        // }
        _ => {
            // Skip files with no extension
            println!("Skipping unsupported file: {:?}", path);
            Ok(())
        }
    }
}

fn analyze_source_file(
    file_path: PathBuf,
    parser: &mut tree_sitter::Parser,
) -> std::io::Result<()> {
    let mut file = fs::File::open(&file_path)?;
    let mut source_code = String::new();
    file.read_to_string(&mut source_code)?;

    let tree = parser.parse(&source_code, None).unwrap();
    let root_node = tree.root_node();

    println!("File: {:?}", file_path);

    visit_node(&source_code, root_node, 0);
    Ok(())
}

fn visit_node(source: &str, node: Node<'_>, depth: usize) {
    if !node.is_named() {
        return;
    }

    print!("{}{}", "  ".repeat(depth), node.kind().to_string().yellow());

    if node.kind() == "identifier" || node.kind() == "type_identifier" {
        let start_byte = node.start_byte();
        let end_byte = node.end_byte();
        let node_text = &source[start_byte..end_byte];
        println!("({})", node_text.bright_cyan());
    } else {
        println!("");
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        visit_node(source, child, depth + 1);
    }
}
