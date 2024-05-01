use crate::context::ContextData;
use crate::{OptixCell, ProgramData};
use data_encoding::HEXLOWER;
use hip_common::raytracing::VariablesBlock;
use hip_common::unwrap_or_return;
use rustc_hash::FxHashMap;
use sha2::{Digest, Sha512};
use std::collections::BTreeMap;
use std::ffi::{CStr, CString};
use std::path::{Path, PathBuf};
use std::rc::Weak;
use std::time::{self, SystemTime};

pub(crate) struct KernelRepository(hip_common::cache::KernelRepository);

impl KernelRepository {
    pub(crate) fn new(cache_file: PathBuf) -> std::result::Result<Self, ()> {
        Ok(Self(hip_common::cache::KernelRepository::new(
            cache_file,
        )))
    }

    // #[cfg(test)]
    // pub(crate) fn new_in_memory() -> std::result::Result<Self, ()> {
    //     Ok(Self(hip_common::cache::KernelRepository::new(None)))
    // }

    pub(crate) fn save_program(
        &mut self,
        program_name: &CStr,
        hash: &str,
        compiler_version: &str,
        git_hash: &str,
        device: &CStr,
        binary: &[u8],
        input_attributes: &str,
        hiprt_version: &str,
    ) -> std::io::Result<()> {
        
        let program_name_bytes = program_name.to_bytes();
        let input_attributes_bytes = input_attributes.as_bytes();
        let hiprt_version_bytes = hiprt_version.as_bytes();

        let mut data: Vec<u8> = Vec::with_capacity(
            program_name_bytes.len() + 1 + input_attributes_bytes.len() + 1 + hiprt_version_bytes.len()
        );
        data.extend_from_slice(program_name_bytes);
        data.push(b':');
        data.extend_from_slice(input_attributes_bytes);
        data.push(b':');
        data.extend_from_slice(hiprt_version_bytes);

        self.0.save_program(
            hash,
            compiler_version,
            git_hash,
            device,
            binary,
            &data
        )
    }

    fn try_load_program(
        &mut self,
        program_name: &CStr,
        hash: &str,
        compiler_version: &str,
        git_hash: &str,
        device: &CStr,
        input_attributes: &str,
        hiprt_version: &str,
    ) -> std::io::Result<Option<Vec<u8>>> {

        let program_name_bytes = program_name.to_bytes();
        let input_attributes_bytes = input_attributes.as_bytes();
        let hiprt_version_bytes = hiprt_version.as_bytes();

        let mut data: Vec<u8> = Vec::with_capacity(
            program_name_bytes.len() + input_attributes_bytes.len() + hiprt_version_bytes.len() + 2);
        data.extend_from_slice(program_name_bytes);
        data.push(b':');
        data.extend_from_slice(input_attributes_bytes);
        data.push(b':');
        data.extend_from_slice(hiprt_version_bytes);

        self.0.try_load_program(
            hash,
            compiler_version,
            git_hash,
            device,
            &data
        )
    }
}

pub(crate) struct KernelCache(KernelRepository);

impl KernelCache {
    pub(crate) fn new(cache_dir: &Path) -> Option<Self> {
        let mut file = cache_dir.to_path_buf();
        Some(Self(KernelRepository::new(file).ok()?))
    }

    pub(crate) fn save_program(
        &mut self,
        compiler_version: &str,
        hiprt_version: &str,
        isa: &CStr,
        program_name: &CStr,
        ptx: &str,
        prog: &ProgramData,
        input_attributes: &VariablesBlock,
    ) {
        let mut hasher = Sha512::new();
        hasher.update(ptx);
        let hash = hasher.finalize();
        let hash = HEXLOWER.encode(&hash[..]);
        let git_hash = env!("VERGEN_GIT_SHA");
        let attributes = unwrap_or_return!(Self::serialize_input_attributes(
            &input_attributes.variables
        ));
        self.0
            .save_program(
                program_name,
                &hash,
                compiler_version,
                git_hash,
                isa,
                &prog.shared.binary,
                &attributes,
                hiprt_version,
            )
            .ok();
    }

    pub(crate) fn try_load_program(
        &mut self,
        weak_context: Weak<OptixCell<ContextData>>,
        compiler_version: &str,
        hiprt_version: &str,
        isa: &CStr,
        program_name: &CStr,
        ptx: &str,
        input_attributes: &VariablesBlock,
    ) -> Option<(ProgramData, VariablesBlock)> {
        let mut hasher = Sha512::new();
        hasher.update(ptx);
        let hash = hasher.finalize();
        let hash = HEXLOWER.encode(&hash[..]);
        let git_hash = env!("VERGEN_GIT_SHA");
        let attributes = Self::serialize_input_attributes(&input_attributes.variables).ok()?;
        let binary = self
            .0
            .try_load_program(
                program_name,
                &hash,
                compiler_version,
                git_hash,
                isa,
                &attributes,
                hiprt_version,
            )
            .ok()??;
        ProgramData::try_from_binary(weak_context, binary)
    }

    fn serialize_input_attributes(
        attributes: &FxHashMap<CString, hip_common::raytracing::Variable>,
    ) -> serde_json::Result<String> {
        let sorted_attrbutes = attributes.iter().collect::<BTreeMap<_, _>>();
        serde_json::to_string(&serialize::VariablesMapSerialize2 {
            variables: sorted_attrbutes,
        })
    }
}

pub(crate) mod serialize {
    use serde::{Deserialize, Serialize};
    use serde_with::{serde_as, SerializeAs};
    use std::collections::BTreeMap;
    use std::ffi::CString;

    #[serde_as]
    #[derive(serde::Serialize)]
    #[serde(transparent)]
    pub(crate) struct VariablesMapSerialize2<'a> {
        #[serde_as(as = "BTreeMap<AsString, &Variable>")]
        pub(crate) variables: BTreeMap<&'a CString, &'a hip_common::raytracing::Variable>,
    }

    struct AsString;

    impl SerializeAs<&CString> for AsString {
        fn serialize_as<S>(value: &&CString, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer,
        {
            serializer.serialize_str(value.to_str().unwrap())
        }
    }

    #[derive(Serialize, Deserialize)]
    #[serde(remote = "hip_common::raytracing::Variable")]
    pub(crate) struct Variable {
        pub size: u32,
        pub offset: u32,
        pub default_value: Vec<u8>,
    }

    impl SerializeAs<hip_common::raytracing::Variable> for Variable {
        fn serialize_as<S>(
            value: &hip_common::raytracing::Variable,
            serializer: S,
        ) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer,
        {
            Variable::serialize(value, serializer)
        }
    }
}

// #[cfg(test)]
// mod tests {
//     use super::KernelRepository;
//     use std::ffi::CString;

//     #[test]
//     fn kernel_insert_select() {
//         let mut cache = KernelRepository::new_in_memory().unwrap();
//         let input_attributes = "{TEST}";
//         cache
//             .save_program(
//                 1,
//                 &*CString::new("start").unwrap(),
//                 "FFFF",
//                 "Clang 15",
//                 "EEEE",
//                 &*CString::new("gfx1030").unwrap(),
//                 &vec![0x11, 0x12, 0x13, 0x14],
//                 &input_attributes,
//                 "1.2",
//             )
//             .unwrap();
//         assert_eq!(get_time(&mut cache.0.connect().unwrap()), 1);
//         let binary = cache
//             .try_load_program(
//                 2,
//                 CString::new("start").unwrap().as_c_str(),
//                 "FFFF",
//                 "Clang 15",
//                 "EEEE",
//                 &*CString::new("gfx1030").unwrap(),
//                 input_attributes,
//                 "1.2",
//             )
//             .unwrap()
//             .unwrap();
//         assert_eq!(binary, vec![0x11, 0x12, 0x13, 0x14]);
//     }

//     fn get_time(connection: &mut rusqlite::Connection) -> i64 {
//         connection
//             .query_row("SELECT last_used FROM kernels", [], |row| row.get(0))
//             .unwrap()
//     }
// }
