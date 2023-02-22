fn main() {
    if supported() {
        println!("cargo:rustc-cfg=linker");
    }
}

fn supported() -> bool {
    use std::process::Command;

    let dir = tempfile::tempdir().unwrap();
    let test_c = dir.path().join("test.c");

    let compiler = cc::Build::new()
        .cargo_metadata(false)
        .get_compiler();
    let compiler_path = compiler.path();
    let target = std::env::var("TARGET").unwrap();

    // It might be better to #include the relevant headers and check that the
    // argument types are as expected.

    if target.contains("linux") {
        std::fs::write(&test_c, b"
            void renameat2();
            void statfs();

            int main() {
                renameat2();
                statfs();
            }"
        ).unwrap();

        let status = Command::new(compiler_path)
            .current_dir(dir.path())
            .arg("test.c")
            .status()
            .unwrap();

        if status.success() {
            return true;
        }

        // musl doesn't expose a wrapper around the renameat2 syscall but it
        // does have the syscall number definition. So we're providing our own
        // wrapper. Although, the syscall might not exist and we'd get an error
        // instead of using the fallback in that case.
        if target.contains("musl") {
            cc::Build::new()
                .file("src/linux-musl.c")
                .compile("linux-musl");
            return true;
        }
    } else if target.contains("apple") {
        std::fs::write(&test_c, b"
            void renamex_np();
            void getattrlist();

            int main() {
                renamex_np();
                getattrlist();
            }
        ").unwrap();

        let status = Command::new(compiler_path)
            .current_dir(dir.path())
            .arg("test.c")
            .status()
            .unwrap();

        if status.success() {
            return true;
        }
    } else if target.contains("windows") {
        std::fs::write(&test_c, b"
            void MoveFileExW();

            int main() {
                MoveFileExW();
            }
        ").unwrap();

        let status = Command::new(compiler_path)
            .current_dir(dir.path())
            .arg("test.c")
            .status()
            .unwrap();

        if status.success() {
            return true;
        }
    }

    false
}
