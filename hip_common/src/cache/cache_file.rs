use itertools::Itertools;
use std::ffi::{CStr, CString};
use std::fmt::Display;
use std::marker::PhantomData;
use std::path::PathBuf;
use std::time::{self, SystemTime};
use sha2::{Digest, Sha256};

pub struct KernelRepository {
    cache_path: PathBuf,
}

// We do all that string concatenation during run time, because there is no
// good alternative. const_format crate implements const-time format!(),
// but it does not work with generic types. Maybe it works with &'static str
// const generics, but &'static str const generics are currently illegal
impl KernelRepository {
    pub fn new(cache_path: PathBuf) -> Self {
        Self {
            cache_path,
        }
    }

    pub fn save_program(
        &self,
        hash: &str,
        compiler_version: &str,
        git_hash: &str,
        device: &CStr,
        binary: &[u8],
        additional_parameters: &[u8],
    ) -> std::io::Result<()> {
        let mut path = self.cache_path.clone();
        let mut hasher = Sha256::new();
        hasher.update(hash);
        hasher.update(b":");
        hasher.update(compiler_version);
        hasher.update(b":");
        hasher.update(git_hash);
        hasher.update(b":");
        hasher.update(device.to_bytes());
        hasher.update(b":");
        hasher.update(additional_parameters);
        let hash = hex::encode(hasher.finalize());
        let hash_prefix = &hash[..2];
        
        path.push(hash_prefix);
        path.push(hash);

        std::fs::create_dir_all(path.parent().unwrap())?;
        std::fs::write(&path, binary)?;

        Ok(())
    }

    pub fn try_load_program(
        &self,
        hash: &str,
        compiler_version: &str,
        git_hash: &str,
        device: &CStr,
        additional_parameters: &[u8],
    ) -> std::io::Result<Option<Vec<u8>>> {
        let mut path = self.cache_path.clone();
        let mut hasher = Sha256::new();
        hasher.update(hash);
        hasher.update(b":");
        hasher.update(compiler_version);
        hasher.update(b":");
        hasher.update(git_hash);
        hasher.update(b":");
        hasher.update(device.to_bytes());
        hasher.update(b":");
        hasher.update(additional_parameters);
        let hash = hex::encode(hasher.finalize());
        let hash_prefix = &hash[..2];

        path.push(hash_prefix);
        path.push(hash);

        match std::fs::read(&path) {
            Ok(binary) => Ok(Some(binary)),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(err) => Err(err),
        }
    }

}
