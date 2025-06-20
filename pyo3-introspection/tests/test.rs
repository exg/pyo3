use anyhow::{ensure, Result};
use pyo3_introspection::{introspect_cdylib, module_stub_files};
use std::collections::HashMap;
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::{env, fs};
use tempfile::NamedTempFile;

#[test]
fn pytests_stubs() -> Result<()> {
    // We run the introspection
    let binary = env::var_os("PYO3_PYTEST_LIB_PATH")
        .expect("The PYO3_PYTEST_LIB_PATH constant must be set and target the pyo3-pytests cdylib");
    let module = introspect_cdylib(binary, "pyo3_pytests")?;
    let actual_stubs = module_stub_files(&module);

    // We read the expected stubs
    let expected_subs_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("pytests")
        .join("stubs");
    let mut expected_subs = HashMap::new();
    add_dir_files(
        &expected_subs_dir,
        &expected_subs_dir.canonicalize()?,
        &mut expected_subs,
    )?;

    // We ensure we do not have extra generated files
    for file_name in actual_stubs.keys() {
        assert!(
            expected_subs.contains_key(file_name),
            "The generated file {} is not in the expected stubs directory pytests/stubs",
            file_name.display()
        );
    }

    // We ensure the expected files are generated properly
    for (file_name, expected_file_content) in &expected_subs {
        let actual_file_content = actual_stubs.get(file_name).unwrap_or_else(|| {
            panic!(
                "The expected stub file {} has not been generated",
                file_name.display()
            )
        });

        let actual_file_content = format_with_ruff(actual_file_content)?;

        // We normalize line jumps for compatibility with Windows
        assert_eq!(
            expected_file_content.replace('\r', ""),
            actual_file_content.replace('\r', ""),
            "The content of file {} is different",
            file_name.display()
        )
    }

    Ok(())
}

fn add_dir_files(
    dir_path: &Path,
    base_dir_path: &Path,
    output: &mut HashMap<PathBuf, String>,
) -> Result<()> {
    for entry in fs::read_dir(dir_path)? {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            add_dir_files(&entry.path(), base_dir_path, output)?;
        } else {
            output.insert(
                entry
                    .path()
                    .canonicalize()?
                    .strip_prefix(base_dir_path)?
                    .into(),
                fs::read_to_string(entry.path())?,
            );
        }
    }
    Ok(())
}

fn format_with_ruff(code: &str) -> Result<String> {
    let temp_file = NamedTempFile::with_suffix(".pyi")?;
    // Write to file
    {
        let mut file = temp_file.as_file();
        file.write_all(code.as_bytes())?;
        file.flush()?;
        file.seek(SeekFrom::Start(0))?;
    }
    ensure!(
        Command::new("ruff")
            .arg("format")
            .arg(temp_file.path())
            .status()?
            .success(),
        "Failed to run ruff"
    );
    let mut content = String::new();
    temp_file.as_file().read_to_string(&mut content)?;
    Ok(content)
}
