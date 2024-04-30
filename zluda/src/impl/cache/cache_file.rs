use hip_common::{
    cache::KernelRepository,
    unwrap_or_return, CompilationMode,
};
use static_assertions::assert_impl_one;
use std::{borrow::Cow, ffi::CStr, path::Path};

pub(crate) struct KernelCache(KernelRepository);
assert_impl_one!(KernelCache: Sync);

impl KernelCache {
    // pub(crate) fn new(cache_dir: &Path) -> Option<Self> {
    //     let mut file = cache_dir.to_path_buf();
    //     file.push(Self::CACHE_FILE);
    //     Some(Self(KernelRepository::new(Some(file)).ok()?))
    // }

    pub(crate) fn new(cache_dir: &Path) -> Option<Self> {
        let file = cache_dir.to_path_buf();
        Some(Self(KernelRepository::new(file)))
    }

    pub(crate) fn save_program(
        &self,
        compiler_version: &str,
        device: &CStr,
        ptx_modules: &[Cow<'_, str>],
        compilation_mode: CompilationMode,
        binary: &[u8],
    ) {
        let mut hasher = blake3::Hasher::new();
        for module in ptx_modules {
            hasher.update(module.as_bytes());
        }
        let hash = hasher.finalize().to_hex();
        let git_hash = env!("VERGEN_GIT_SHA");
        self.0
            .save_program(
                hash.as_str(),
                compiler_version,
                git_hash,
                device,
                binary,
                &[compilation_mode as u8],
            )
            .ok();
    }

    pub(crate) fn try_load_program(
        &self,
        compiler_version: &str,
        device: &CStr,
        ptx_modules: &[Cow<'_, str>],
        compilation_mode: CompilationMode,
    ) -> Option<Vec<u8>> {
        let mut hasher = blake3::Hasher::new();
        for module in ptx_modules {
            hasher.update(module.as_bytes());
        }
        let hash = hasher.finalize().to_hex();
        let git_hash = env!("VERGEN_GIT_SHA");
        Some(
            self.0
                .try_load_program(
                    hash.as_str(),
                    compiler_version,
                    git_hash,
                    device,
                    &[compilation_mode as u8],
                )
                .ok()
                .flatten()?,
        )
    }
}
