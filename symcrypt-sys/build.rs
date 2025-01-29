fn main() {
    #[cfg(feature = "static")]
    {
        const SYMCRYPT_SOURCE_CHECKOUT: &'static str = "target/remote-source";

        //=================================================
        // Pull the sources from the SymCrypt repo via gix
        //=================================================
        if std::fs::metadata(SYMCRYPT_SOURCE_CHECKOUT).is_ok() {
            eprintln!("Checkout exists, skipping clone");
        } else {
            std::fs::create_dir_all(SYMCRYPT_SOURCE_CHECKOUT).unwrap();

            // SAFETY: The closure doesn't use mutexes or memory allocation, so it should be safe to call from a signal handler.
            unsafe {
                gix::interrupt::init_handler(1, || {}).unwrap();
            }
            let mut clone = gix::prepare_clone(
                "https://github.com/microsoft/SymCrypt",
                SYMCRYPT_SOURCE_CHECKOUT,
            )
            .unwrap()
            .with_shallow(gix::remote::fetch::Shallow::DepthAtRemote(
                std::num::NonZeroU32::new(1).unwrap(),
            ));
            // .with_ref_name(Some("HEAD"))
            // .unwrap();

            let (mut prepare_checkout, _) = clone
                .fetch_then_checkout(gix::progress::Discard, &gix::interrupt::IS_INTERRUPTED)
                .unwrap();

            let (repo, _) = prepare_checkout
                .main_worktree(gix::progress::Discard, &gix::interrupt::IS_INTERRUPTED)
                .unwrap();

            eprintln!(
                "Repo cloned into {:?}",
                repo.work_dir().expect("directory pre-created")
            );
        }

        //===================
        // Build using cmake
        //===================
        const SYMCRYPT_TARGET: &'static str = "symcrypt_generic";

        let dst = cmake::Config::new(SYMCRYPT_SOURCE_CHECKOUT)
            .define("SYMCRYPT_TARGET_ARCH", "ARM64")
            .define("SYMCRYPT_USE_ASM", "OFF")
            .define("SYMCRYPT_FIPS_BUILD", "OFF")
            .define("CMAKE_BUILD_TYPE", "RelWithDebInfo")
            .build_target(SYMCRYPT_TARGET)
            .build();

        println!(
            "cargo::rustc-link-search=native={}/build/lib",
            dst.display()
        );
        println!("cargo::rustc-link-lib=static:+bundle={}", SYMCRYPT_TARGET);
        // println!("cargo::rustc-link-lib=static:+bundle={}", "symcrypt_common");
    }

    #[cfg(not(feature = "static"))]
    {
        #[cfg(target_os = "windows")]
        {
            // Look for the .lib file during link time. We are searching the Windows/System32 path which is set as a current default to match
            // the long term placement of a Windows shipped symcrypt.dll

            let lib_path = std::env::var("SYMCRYPT_LIB_PATH")
            .unwrap_or_else(|_| panic!("SYMCRYPT_LIB_PATH environment variable not set, for more information please see: https://github.com/microsoft/rust-symcrypt/tree/main/rust-symcrypt#quick-start-guide"));
            println!("cargo:rustc-link-search=native={}", lib_path);

            println!("cargo:rustc-link-lib=dylib=symcrypt");

            // During run time, the OS will handle finding the symcrypt.dll file. The places Windows will look will be:
            // 1. The folder from which the application loaded.
            // 2. The system folder. Use the GetSystemDirectory function to retrieve the path of this folder.
            // 3. The Windows folder. Use the GetWindowsDirectory function to get the path of this folder.
            // 4. The current folder.
            // 5. The directories that are listed in the PATH environment variable.

            // For more info please see: https://learn.microsoft.com/en-us/windows/win32/dlls/dynamic-link-library-search-order

            // For the least invasive usage, we suggest putting the symcrypt.dll inside of same folder as the .exe file.

            // Note: This process is a band-aid. Long-term SymCrypt will be shipped with Windows which will make this process much more
            // streamlined.
        }

        #[cfg(target_os = "linux")]
        {
            // Note: Linux support is based off of the Azure Linux distro.
            // This has been tested on Ubuntu 22.04.03 LTS on WSL and has confirmed working but support for other distros
            // aside from Azure Linux is not guaranteed so YMMV.
            println!("cargo:rustc-link-lib=dylib=symcrypt"); // the "lib" prefix for libsymcrypt is implied on Linux

            // You must put the included symcrypt.so files in your usr/lib/x86_64-linux-gnu/ path.
            // This is where the Linux ld linker will look for the symcrypt.so files.

            // Note: This process is a band-aid. Long-term, our long term solution is to package manage SymCrypt for a subset of
            // Linux distros.
        }

        #[cfg(target_os = "macos")]
        {
            let _ = std::env::var("SYMCRYPT_LIB_PATH").and_then(|lib_path| {
                println!("cargo:rustc-link-search=native={}", lib_path);
                Ok(())
            });
            println!("cargo:rustc-link-lib=dylib=symcrypt");
        }
    }
}
