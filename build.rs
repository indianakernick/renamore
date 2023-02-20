fn main() {
    if let Ok(true) = check() {
        println!("cargo:rustc-cfg=linker");
    }
}

fn check() -> Result<bool, Box<dyn std::error::Error>> {
    use std::process::Command;

    let dir = tempfile::tempdir()?;
    let test_c = dir.path().join("test.c");

    let compiler = cc::Build::new()
        .cargo_metadata(false)
        .get_compiler();
    let compiler_path = compiler.path();

    // It might be better to #include the relevant headers and check that the
    // argument types are as expected.

    if cfg!(target_os = "linux") {
        std::fs::write(&test_c, b"
            void renameat2();
            void statfs();

            int main() {
                renameat2();
                statfs();
            }"
        )?;

        let status = Command::new(compiler_path)
            .current_dir(dir.path())
            .arg("test.c")
            .status()?;

        if status.success() {
            return Ok(true);
        }

        // musl doesn't expose a wrapper around the renameat2 syscall but it
        // does have the syscall number definition. So we're providing our own
        // wrapper. Although, the syscall might not exist and we'd get an error
        // instead of using the fallback in that case.
        if cfg!(target_env="musl") {
            cc::Build::new()
                .file("src/linux-musl.c")
                .compile("linux-musl");
            return Ok(true);
        }
    } else if cfg!(target_vendor = "apple") {
        std::fs::write(&test_c, b"
            void renamex_np();
            void getattrlist();

            int main() {
                renamex_np();
                getattrlist();
            }
        ")?;

        let status = Command::new(compiler_path)
            .current_dir(dir.path())
            .arg("test.c")
            .status()?;

        if status.success() {
            return Ok(true);
        }
    } else if cfg!(target_os = "windows") {
        std::fs::write(&test_c, b"
            void MoveFileExW();

            int main() {
                MoveFileExW();
            }
        ")?;

        let status = Command::new(compiler_path)
            .current_dir(dir.path())
            .arg("test.c")
            .status()?;

        if status.success() {
            return Ok(true);
        }
    }

    Ok(false)
}
