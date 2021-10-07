mod plugin;
mod rpc;

use std::{
    path::{Path, PathBuf},
    process::Command,
};

use anyhow::{bail, Result};
use fs_err as fs;
use proc_macro2::TokenStream;

fn main() -> Result<()> {
    let workspace_root = workspace_root();
    plugin::gen(&workspace_root)?;
    rpc::gen(&workspace_root)?;
    Ok(())
}

fn arci_traits(workspace_root: &Path) -> Result<Vec<syn::ItemTrait>> {
    let path = &workspace_root.join("arci/src/traits");
    let mut files: Vec<_> = fs::read_dir(path)?
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let path = entry.path();
            if !path.is_file() || path.extension().map_or(true, |e| e != "rs") {
                None
            } else {
                Some(path)
            }
        })
        .collect();
    files.sort_unstable();
    let mut traits = vec![];
    for path in &files {
        let s = &fs::read_to_string(path)?;
        let file: syn::File = syn::parse_str(s)?;
        for item in file.items {
            match item {
                syn::Item::Trait(item) if matches!(item.vis, syn::Visibility::Public(_)) => {
                    traits.push(item);
                }
                _ => {}
            }
        }
    }
    Ok(traits)
}

fn is_str(ty: &syn::Type) -> bool {
    if let syn::Type::Reference(ty) = ty {
        if let Some(path) = get_ty_path(&ty.elem) {
            if path.is_ident("str") {
                return true;
            }
        }
    }
    false
}

fn get_ty_path(ty: &syn::Type) -> Option<&syn::Path> {
    if let syn::Type::Path(ty) = ty {
        return Some(&ty.path);
    }
    None
}

fn is_result(ty: &syn::Type) -> Option<&syn::Type> {
    let path = get_ty_path(ty)?;
    if path.segments.len() == 1 && path.segments[0].ident == "Result" {
        if let syn::PathArguments::AngleBracketed(args) = &path.segments.last().unwrap().arguments {
            if let syn::GenericArgument::Type(ty) = &args.args[0] {
                return Some(ty);
            }
        }
    }
    None
}

fn workspace_root() -> PathBuf {
    let mut dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    dir.pop(); // codegen
    dir.pop(); // tools
    dir
}

fn header() -> String {
    concat!(
        "// This file is @generated by ",
        env!("CARGO_BIN_NAME"),
        ".\n",
        "// It is not intended for manual editing.\n",
        "\n",
        "#![allow(unused_variables)]\n",
        "#![allow(clippy::useless_conversion, clippy::unit_arg)]\n",
        "\n",
    )
    .into()
}

fn write(path: &Path, contents: &TokenStream) -> Result<()> {
    let contents = header() + &contents.to_string();

    let tmpdir = tempfile::Builder::new().prefix("codegen").tempdir()?;
    let tmpfile = &tmpdir.path().join("generated");
    fs::write(tmpfile, &contents)?;
    fs::copy(
        workspace_root().join("rustfmt.toml"),
        tmpdir.path().join(".rustfmt.toml"),
    )?;

    let status = Command::new("rustfmt")
        .arg(tmpfile)
        .args(&[
            "--config",
            "normalize_doc_attributes=true,format_macro_matchers=true",
        ])
        .status()?;
    if !status.success() {
        bail!("rustfmt didn't exit successfully");
    }

    let out = fs::read(tmpfile)?;
    if path.is_file() && fs::read(&path)? == out {
        return Ok(());
    }
    fs::write(path, out)?;
    Ok(())
}
