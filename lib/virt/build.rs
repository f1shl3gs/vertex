use std::env;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use std::process::Command;

fn compile(source: &str) {
    // compiling remote_protocol.x is a bit more involved
    // first process it with cpp to eval defines
    let cpp = Command::new("/usr/bin/cpp")
        // constants from libvirt-host.h
        .arg("-include")
        .arg("libvirt-defs.h")
        .arg(source)
        .output()
        .unwrap();

    // then write output to temporarily file
    let out_dir = env::var("OUT_DIR").unwrap();
    let mut path = PathBuf::from(out_dir);
    path.push(source);
    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(&path)
        .unwrap();
    file.write_all(&cpp.stdout).unwrap();

    // finally run xdrgen
    let path_str = format!("{}", path.display());
    xdrgen::compile(path_str).unwrap();
}

fn main() {
    compile("virnetprotocol.x");
    compile("remote_protocol.x");

    // xdrgen does not support
    // 1. add Default
    // 2. handle some empty array
    //
    // We can fix those by replacing some lines of the generated files
    let out_dir = env::var("OUT_DIR").unwrap();
    let path = PathBuf::from(out_dir).join("remote_protocol_xdr.rs");

    let content = std::fs::read_to_string(&path).unwrap();
    let content = content.replace(
        r#"#[derive( Copy , Clone , Debug , Eq , PartialEq )] pub struct remote_connect_get_version_ret { pub hv_ver : u64 , }"#,
        r#"#[derive( Copy , Clone , Debug , Default, Eq , PartialEq )] pub struct remote_connect_get_version_ret { pub hv_ver : u64 , }"#
    ).replace(
        r#"#[derive( Copy , Clone , Debug , Eq , PartialEq )] pub struct remote_connect_get_lib_version_ret { pub lib_ver : u64 , }"#,
        r#"#[derive( Copy , Clone , Debug , Default, Eq , PartialEq )] pub struct remote_connect_get_lib_version_ret { pub lib_ver : u64 , }"#
    ).replace(
        r#"#[derive( Clone , Debug , Eq , PartialEq )] pub struct remote_auth_list_ret { pub types : Vec < remote_auth_type > , }"#,
        r#"#[derive( Clone , Debug , Default, Eq , PartialEq )] pub struct remote_auth_list_ret { pub types : Vec < remote_auth_type > , }"#,
    ).replace(
        r#"#[derive( Clone , Debug , PartialEq )] pub struct remote_connect_get_all_domain_stats_ret { pub retStats : Vec < remote_domain_stats_record > , }"#,
        r#"#[derive( Clone , Debug , Default, PartialEq )] pub struct remote_connect_get_all_domain_stats_ret { pub retStats : Vec < remote_domain_stats_record > , }"#,
    ).replace(
        r#"#[derive( Copy , Clone , Debug , Eq , PartialEq )] pub struct remote_domain_get_info_ret { pub state : u8 , pub maxMem : u64 , pub memory : u64 , pub nrVirtCpu : u32 , pub cpuTime : u64 , }"#,
        r#"#[derive( Copy , Clone , Debug , Default, Eq , PartialEq )] pub struct remote_domain_get_info_ret { pub state : u8 , pub maxMem : u64 , pub memory : u64 , pub nrVirtCpu : u32 , pub cpuTime : u64 , }"#,
    ).replace(
        r#"#[derive( Clone , Debug , PartialEq )] pub struct remote_domain_get_block_io_tune_ret { pub params : Vec < remote_typed_param > , pub nparams : i32 , }"#,
        r#"#[derive( Clone , Debug , Default, PartialEq )] pub struct remote_domain_get_block_io_tune_ret { pub params : Vec < remote_typed_param > , pub nparams : i32 , }"#,
    ).replace(
        r#"#[derive( Clone , Debug , Eq , PartialEq )] pub struct remote_domain_get_vcpus_args { pub dom : remote_nonnull_domain , pub maxinfo : i32 , pub maplen : i32 , }"#,
        r#"#[derive( Clone , Debug , Default, Eq , PartialEq )] pub struct remote_domain_get_vcpus_args { pub dom : remote_nonnull_domain , pub maxinfo : i32 , pub maplen : i32 , }"#,
    ).replace(
        r#"#[derive( Clone , Debug , Eq , PartialEq )] pub struct remote_domain_get_vcpus_ret { pub info : Vec < remote_vcpu_info > , pub cpumaps : Vec < u8 > , }"#,
        r#"#[derive( Clone , Debug , Default, Eq , PartialEq )] pub struct remote_domain_get_vcpus_ret { pub info : Vec < remote_vcpu_info > , pub cpumaps : Vec < u8 > , }"#,
    ).replace(
        r#"#[derive( Clone , Debug , Eq , PartialEq )] pub struct remote_domain_memory_stats_args { pub dom : remote_nonnull_domain , pub maxStats : u32 , pub flags : u32 , }"#,
        r#"#[derive( Clone , Debug , Default, Eq , PartialEq )] pub struct remote_domain_memory_stats_args { pub dom : remote_nonnull_domain , pub maxStats : u32 , pub flags : u32 , }"#,
    ).replace(
        r#"#[derive( Clone , Debug , Eq , PartialEq )] pub struct remote_domain_memory_stats_ret { pub stats : Vec < remote_domain_memory_stat > , }"#,
        r#"#[derive( Clone , Debug , Default, Eq , PartialEq )] pub struct remote_domain_memory_stats_ret { pub stats : Vec < remote_domain_memory_stat > , }"#,
    ).replace(
        r#"#[derive( Copy , Clone , Debug , Eq , PartialEq )] pub struct remote_connect_list_all_storage_pools_args { pub need_results : i32 , pub flags : u32 , }"#,
        r#"#[derive( Copy , Clone , Debug , Default, Eq , PartialEq )] pub struct remote_connect_list_all_storage_pools_args { pub need_results : i32 , pub flags : u32 , }"#,
    ).replace(
        r#"#[derive( Clone , Debug , Eq , PartialEq )] pub struct remote_connect_list_all_storage_pools_ret { pub pools : Vec < remote_nonnull_storage_pool > , pub ret : u32 , }"#,
        r#"#[derive( Clone , Debug , Default, Eq , PartialEq )] pub struct remote_connect_list_all_storage_pools_ret { pub pools : Vec < remote_nonnull_storage_pool > , pub ret : u32 , }"#,
    ).replace(
        r#"#[derive( Clone , Debug , Eq , PartialEq )] pub struct remote_storage_pool_get_info_args { pub pool : remote_nonnull_storage_pool , }"#,
        r#"#[derive( Clone , Debug , Default, Eq , PartialEq )] pub struct remote_storage_pool_get_info_args { pub pool : remote_nonnull_storage_pool , }"#,
    ).replace(
        r#"#[derive( Copy , Clone , Debug , Eq , PartialEq )] pub struct remote_storage_pool_get_info_ret { pub state : u8 , pub capacity : u64 , pub allocation : u64 , pub available : u64 , }"#,
        r#"#[derive( Copy , Clone , Debug , Default, Eq , PartialEq )] pub struct remote_storage_pool_get_info_ret { pub state : u8 , pub capacity : u64 , pub allocation : u64 , pub available : u64 , }"#,
    ).replace(
        r#"#[derive( Clone , Debug , Eq , PartialEq )] pub struct remote_domain_get_xml_desc_args { pub dom : remote_nonnull_domain , pub flags : u32 , }"#,
        r#"#[derive( Clone , Debug , Default, Eq , PartialEq )] pub struct remote_domain_get_xml_desc_args { pub dom : remote_nonnull_domain , pub flags : u32 , }"#,
    ).replace(
        r#"#[derive( Clone , Debug , Eq , PartialEq )] pub struct remote_domain_get_xml_desc_ret { pub xml : remote_nonnull_string , }"#,
        r#"#[derive( Clone , Debug , Default, Eq , PartialEq )] pub struct remote_domain_get_xml_desc_ret { pub xml : remote_nonnull_string , }"#,
    ).replace(
        r#"#[derive( Clone , Debug , Eq , PartialEq )] pub struct remote_nonnull_domain { pub name : remote_nonnull_string , pub uuid : remote_uuid , pub id : i32 , }"#,
        r#"#[derive( Clone , Debug , Default, Eq , PartialEq )] pub struct remote_nonnull_domain { pub name : remote_nonnull_string , pub uuid : remote_uuid , pub id : i32 , }"#,
    ).replace(
        r#"#[derive( Clone , Debug , Eq , PartialEq )] pub struct remote_nonnull_string ( pub String ) ;"#,
        r#"#[derive( Clone , Debug , Default, Eq , PartialEq )] pub struct remote_nonnull_string ( pub String ) ;"#,
    ).replace(
        r#"#[derive( Clone , Debug , Eq , PartialEq )] pub struct remote_nonnull_storage_pool { pub name : remote_nonnull_string , pub uuid : remote_uuid , }"#,
        r#"#[derive( Clone , Debug , Default, Eq , PartialEq )] pub struct remote_nonnull_storage_pool { pub name : remote_nonnull_string , pub uuid : remote_uuid , }"#,
    ).replace(
        r#"#[derive( Copy , Clone , Debug , Eq , PartialEq )] pub struct remote_uuid ( pub [ u8 ; 16i64 as usize ] ) ;"#,
        r#"#[derive( Copy , Clone , Debug , Default, Eq , PartialEq )] pub struct remote_uuid ( pub [ u8 ; 16i64 as usize ] ) ;"#,
    ).replace(
        r#"REMOTE_STORAGE_POOL_LIST_MAX as usize ) ) ? ; sz += fsz ; v } , ret : { let ( v , fsz ) = xdr_codec :: Unpack :: unpack ( input ) ?"#,
        r#"REMOTE_STORAGE_POOL_LIST_MAX as usize ) ) ? ; sz += fsz ; v } , ret : { let ( v , fsz ) = xdr_codec :: Unpack :: unpack ( input ).unwrap_or_default()"#,
    );

    std::fs::write(&path, content).unwrap();

    // Another file
    let out_dir = env::var("OUT_DIR").unwrap();
    let path = PathBuf::from(out_dir).join("virnetprotocol_xdr.rs");
    let content = std::fs::read_to_string(&path).unwrap();
    let content = content.replace(
        r#"unsafe { :: std :: mem :: uninitialized ( ) }"#,
        r#"[0; 16]"#,
    );

    std::fs::write(&path, content).unwrap();
}
