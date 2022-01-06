// GENERATED CODE
//
// Generated from /home/f1shl3gs/Workspaces/clion/vertex/target/debug/build/virt-84ca5d2b339de854/out/remote_protocol.x by xdrgen.
//
// DO NOT EDIT

pub const REMOTE_AUTH_SASL_DATA_MAX: i64 = 65536i64;
pub const REMOTE_AUTH_TYPE_LIST_MAX: i64 = 20i64;
pub const REMOTE_CONNECT_CPU_MODELS_MAX: i64 = 8192i64;
pub const REMOTE_CONNECT_GET_ALL_DOMAIN_STATS_MAX: i64 = 4096i64;
pub const REMOTE_CPUMAPS_MAX: i64 = 8388608i64;
pub const REMOTE_CPUMAP_MAX: i64 = 2048i64;
pub const REMOTE_CPU_BASELINE_MAX: i64 = 256i64;
pub const REMOTE_DOMAIN_BLKIO_PARAMETERS_MAX: i64 = 16i64;
pub const REMOTE_DOMAIN_BLOCK_COPY_PARAMETERS_MAX: i64 = 16i64;
pub const REMOTE_DOMAIN_BLOCK_IO_TUNE_PARAMETERS_MAX: i64 = 32i64;
pub const REMOTE_DOMAIN_BLOCK_PEEK_BUFFER_MAX: i64 = 4194304i64;
pub const REMOTE_DOMAIN_BLOCK_STATS_PARAMETERS_MAX: i64 = 16i64;
pub const REMOTE_DOMAIN_DISK_ERRORS_MAX: i64 = 256i64;
pub const REMOTE_DOMAIN_EVENT_GRAPHICS_IDENTITY_MAX: i64 = 20i64;
pub const REMOTE_DOMAIN_EVENT_TUNABLE_MAX: i64 = 2048i64;
pub const REMOTE_DOMAIN_FSFREEZE_MOUNTPOINTS_MAX: i64 = 256i64;
pub const REMOTE_DOMAIN_FSINFO_DISKS_MAX: i64 = 256i64;
pub const REMOTE_DOMAIN_FSINFO_MAX: i64 = 256i64;
pub const REMOTE_DOMAIN_GET_CPU_STATS_MAX: i64 = 2048i64;
pub const REMOTE_DOMAIN_GET_CPU_STATS_NCPUS_MAX: i64 = 128i64;
pub const REMOTE_DOMAIN_GUEST_VCPU_PARAMS_MAX: i64 = 64i64;
pub const REMOTE_DOMAIN_INTERFACE_MAX: i64 = 2048i64;
pub const REMOTE_DOMAIN_INTERFACE_PARAMETERS_MAX: i64 = 16i64;
pub const REMOTE_DOMAIN_IP_ADDR_MAX: i64 = 2048i64;
pub const REMOTE_DOMAIN_JOB_STATS_MAX: i64 = 64i64;
pub const REMOTE_DOMAIN_LIST_MAX: i64 = 16384i64;
pub const REMOTE_DOMAIN_MEMORY_PARAMETERS_MAX: i64 = 16i64;
pub const REMOTE_DOMAIN_MEMORY_PEEK_BUFFER_MAX: i64 = 4194304i64;
pub const REMOTE_DOMAIN_MEMORY_STATS_MAX: i64 = 1024i64;
pub const REMOTE_DOMAIN_MIGRATE_PARAM_LIST_MAX: i64 = 64i64;
pub const REMOTE_DOMAIN_NUMA_PARAMETERS_MAX: i64 = 16i64;
pub const REMOTE_DOMAIN_PERF_EVENTS_MAX: i64 = 64i64;
pub const REMOTE_DOMAIN_SCHEDULER_PARAMETERS_MAX: i64 = 16i64;
pub const REMOTE_DOMAIN_SEND_KEY_MAX: i64 = 16i64;
pub const REMOTE_DOMAIN_SNAPSHOT_LIST_MAX: i64 = 1024i64;
pub const REMOTE_INTERFACE_LIST_MAX: i64 = 16384i64;
pub const REMOTE_IOTHREAD_INFO_MAX: i64 = 16384i64;
pub const REMOTE_MIGRATE_COOKIE_MAX: i64 = 4194304i64;

pub const REMOTE_NETWORK_DHCP_LEASES_MAX: i64 = 65536i64;

pub const REMOTE_NETWORK_LIST_MAX: i64 = 16384i64;

pub const REMOTE_NODE_CPU_STATS_MAX: i64 = 16i64;

pub const REMOTE_NODE_DEVICE_CAPS_LIST_MAX: i64 = 65536i64;

pub const REMOTE_NODE_DEVICE_LIST_MAX: i64 = 65536i64;

pub const REMOTE_NODE_MAX_CELLS: i64 = 1024i64;

pub const REMOTE_NODE_MEMORY_PARAMETERS_MAX: i64 = 64i64;

pub const REMOTE_NODE_MEMORY_STATS_MAX: i64 = 16i64;

pub const REMOTE_NWFILTER_LIST_MAX: i64 = 1024i64;

pub const REMOTE_PROGRAM: i64 = 536903814i64;

pub const REMOTE_PROTOCOL_VERSION: i64 = 1i64;

pub const REMOTE_SECRET_LIST_MAX: i64 = 16384i64;

pub const REMOTE_SECRET_VALUE_MAX: i64 = 65536i64;

pub const REMOTE_SECURITY_DOI_MAX: i64 = 257i64;

pub const REMOTE_SECURITY_LABEL_LIST_MAX: i64 = 64i64;

pub const REMOTE_SECURITY_LABEL_MAX: i64 = 4097i64;

pub const REMOTE_SECURITY_MODEL_MAX: i64 = 257i64;

pub const REMOTE_STORAGE_POOL_LIST_MAX: i64 = 4096i64;

pub const REMOTE_STORAGE_VOL_LIST_MAX: i64 = 16384i64;

pub const REMOTE_STRING_MAX: i64 = 4194304i64;

pub const REMOTE_VCPUINFO_MAX: i64 = 16384i64;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct remote_auth_list_ret {
    pub types: Vec<remote_auth_type>,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct remote_auth_polkit_ret {
    pub complete: i32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_auth_sasl_init_ret {
    pub mechlist: remote_nonnull_string,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_auth_sasl_start_args {
    pub mech: remote_nonnull_string,
    pub nil: i32,
    pub data: Vec<i8>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_auth_sasl_start_ret {
    pub complete: i32,
    pub nil: i32,
    pub data: Vec<i8>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_auth_sasl_step_args {
    pub nil: i32,
    pub data: Vec<i8>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_auth_sasl_step_ret {
    pub complete: i32,
    pub nil: i32,
    pub data: Vec<i8>,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum remote_auth_type {
    REMOTE_AUTH_NONE = 0isize,
    REMOTE_AUTH_SASL = 1isize,
    REMOTE_AUTH_POLKIT = 2isize,
}

impl Default for remote_auth_type {
    fn default() -> Self {
        Self::REMOTE_AUTH_NONE
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_connect_baseline_cpu_args {
    pub xmlCPUs: Vec<remote_nonnull_string>,
    pub flags: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_connect_baseline_cpu_ret {
    pub cpu: remote_nonnull_string,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_connect_compare_cpu_args {
    pub xml: remote_nonnull_string,
    pub flags: u32,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct remote_connect_compare_cpu_ret {
    pub result: i32,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct remote_connect_domain_event_callback_deregister_any_args {
    pub callbackID: i32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_connect_domain_event_callback_register_any_args {
    pub eventID: i32,
    pub dom: remote_domain,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct remote_connect_domain_event_callback_register_any_ret {
    pub callbackID: i32,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct remote_connect_domain_event_deregister_any_args {
    pub eventID: i32,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct remote_connect_domain_event_deregister_ret {
    pub cb_registered: i32,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct remote_connect_domain_event_register_any_args {
    pub eventID: i32,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct remote_connect_domain_event_register_ret {
    pub cb_registered: i32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_connect_domain_xml_from_native_args {
    pub nativeFormat: remote_nonnull_string,
    pub nativeConfig: remote_nonnull_string,
    pub flags: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_connect_domain_xml_from_native_ret {
    pub domainXml: remote_nonnull_string,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_connect_domain_xml_to_native_args {
    pub nativeFormat: remote_nonnull_string,
    pub domainXml: remote_nonnull_string,
    pub flags: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_connect_domain_xml_to_native_ret {
    pub nativeConfig: remote_nonnull_string,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct remote_connect_event_connection_closed_msg {
    pub reason: i32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_connect_find_storage_pool_sources_args {
    pub type_: remote_nonnull_string,
    pub srcSpec: remote_string,
    pub flags: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_connect_find_storage_pool_sources_ret {
    pub xml: remote_nonnull_string,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_connect_get_all_domain_stats_args {
    pub doms: Vec<remote_nonnull_domain>,
    pub stats: u32,
    pub flags: u32,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct remote_connect_get_all_domain_stats_ret {
    pub retStats: Vec<remote_domain_stats_record>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_connect_get_capabilities_ret {
    pub capabilities: remote_nonnull_string,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_connect_get_cpu_model_names_args {
    pub arch: remote_nonnull_string,
    pub need_results: i32,
    pub flags: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_connect_get_cpu_model_names_ret {
    pub models: Vec<remote_nonnull_string>,
    pub ret: i32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_connect_get_domain_capabilities_args {
    pub emulatorbin: remote_string,
    pub arch: remote_string,
    pub machine: remote_string,
    pub virttype: remote_string,
    pub flags: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_connect_get_domain_capabilities_ret {
    pub capabilities: remote_nonnull_string,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_connect_get_hostname_ret {
    pub hostname: remote_nonnull_string,
}

#[derive(Copy, Clone, Debug, Default, Eq, PartialEq)]
pub struct remote_connect_get_lib_version_ret {
    pub lib_ver: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_connect_get_max_vcpus_args {
    pub type_: remote_string,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct remote_connect_get_max_vcpus_ret {
    pub max_vcpus: i32,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct remote_connect_get_sysinfo_args {
    pub flags: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_connect_get_sysinfo_ret {
    pub sysinfo: remote_nonnull_string,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_connect_get_type_ret {
    pub type_: remote_nonnull_string,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_connect_get_uri_ret {
    pub uri: remote_nonnull_string,
}

#[derive(Copy, Clone, Debug, Default, Eq, PartialEq)]
pub struct remote_connect_get_version_ret {
    pub hv_ver: u64,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct remote_connect_is_secure_ret {
    pub secure: i32,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct remote_connect_list_all_domains_args {
    pub need_results: i32,
    pub flags: u32,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct remote_connect_list_all_domains_ret {
    pub domains: Vec<remote_nonnull_domain>,
    pub ret: u32,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct remote_connect_list_all_interfaces_args {
    pub need_results: i32,
    pub flags: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_connect_list_all_interfaces_ret {
    pub ifaces: Vec<remote_nonnull_interface>,
    pub ret: u32,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct remote_connect_list_all_networks_args {
    pub need_results: i32,
    pub flags: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_connect_list_all_networks_ret {
    pub nets: Vec<remote_nonnull_network>,
    pub ret: u32,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct remote_connect_list_all_node_devices_args {
    pub need_results: i32,
    pub flags: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_connect_list_all_node_devices_ret {
    pub devices: Vec<remote_nonnull_node_device>,
    pub ret: u32,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct remote_connect_list_all_nwfilters_args {
    pub need_results: i32,
    pub flags: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_connect_list_all_nwfilters_ret {
    pub filters: Vec<remote_nonnull_nwfilter>,
    pub ret: u32,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct remote_connect_list_all_secrets_args {
    pub need_results: i32,
    pub flags: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_connect_list_all_secrets_ret {
    pub secrets: Vec<remote_nonnull_secret>,
    pub ret: u32,
}

#[derive(Copy, Clone, Debug, Default, Eq, PartialEq)]
pub struct remote_connect_list_all_storage_pools_args {
    pub need_results: i32,
    pub flags: u32,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct remote_connect_list_all_storage_pools_ret {
    pub pools: Vec<remote_nonnull_storage_pool>,
    pub ret: u32,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct remote_connect_list_defined_domains_args {
    pub maxnames: i32,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct remote_connect_list_defined_domains_ret {
    pub names: Vec<remote_nonnull_string>,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct remote_connect_list_defined_interfaces_args {
    pub maxnames: i32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_connect_list_defined_interfaces_ret {
    pub names: Vec<remote_nonnull_string>,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct remote_connect_list_defined_networks_args {
    pub maxnames: i32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_connect_list_defined_networks_ret {
    pub names: Vec<remote_nonnull_string>,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct remote_connect_list_defined_storage_pools_args {
    pub maxnames: i32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_connect_list_defined_storage_pools_ret {
    pub names: Vec<remote_nonnull_string>,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct remote_connect_list_domains_args {
    pub maxids: i32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_connect_list_domains_ret {
    pub ids: Vec<i32>,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct remote_connect_list_interfaces_args {
    pub maxnames: i32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_connect_list_interfaces_ret {
    pub names: Vec<remote_nonnull_string>,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct remote_connect_list_networks_args {
    pub maxnames: i32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_connect_list_networks_ret {
    pub names: Vec<remote_nonnull_string>,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct remote_connect_list_nwfilters_args {
    pub maxnames: i32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_connect_list_nwfilters_ret {
    pub names: Vec<remote_nonnull_string>,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct remote_connect_list_secrets_args {
    pub maxuuids: i32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_connect_list_secrets_ret {
    pub uuids: Vec<remote_nonnull_string>,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct remote_connect_list_storage_pools_args {
    pub maxnames: i32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_connect_list_storage_pools_ret {
    pub names: Vec<remote_nonnull_string>,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct remote_connect_network_event_deregister_any_args {
    pub callbackID: i32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_connect_network_event_register_any_args {
    pub eventID: i32,
    pub net: remote_network,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct remote_connect_network_event_register_any_ret {
    pub callbackID: i32,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct remote_connect_node_device_event_deregister_any_args {
    pub callbackID: i32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_connect_node_device_event_register_any_args {
    pub eventID: i32,
    pub dev: remote_node_device,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct remote_connect_node_device_event_register_any_ret {
    pub callbackID: i32,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct remote_connect_num_of_defined_domains_ret {
    pub num: i32,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct remote_connect_num_of_defined_interfaces_ret {
    pub num: i32,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct remote_connect_num_of_defined_networks_ret {
    pub num: i32,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct remote_connect_num_of_defined_storage_pools_ret {
    pub num: i32,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct remote_connect_num_of_domains_ret {
    pub num: i32,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct remote_connect_num_of_interfaces_ret {
    pub num: i32,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct remote_connect_num_of_networks_ret {
    pub num: i32,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct remote_connect_num_of_nwfilters_ret {
    pub num: i32,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct remote_connect_num_of_secrets_ret {
    pub num: i32,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct remote_connect_num_of_storage_pools_ret {
    pub num: i32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_connect_open_args {
    pub name: remote_string,
    pub flags: u32,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct remote_connect_secret_event_deregister_any_args {
    pub callbackID: i32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_connect_secret_event_register_any_args {
    pub eventID: i32,
    pub secret: remote_secret,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct remote_connect_secret_event_register_any_ret {
    pub callbackID: i32,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct remote_connect_storage_pool_event_deregister_any_args {
    pub callbackID: i32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_connect_storage_pool_event_register_any_args {
    pub eventID: i32,
    pub pool: remote_storage_pool,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct remote_connect_storage_pool_event_register_any_ret {
    pub callbackID: i32,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct remote_connect_supports_feature_args {
    pub feature: i32,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct remote_connect_supports_feature_ret {
    pub supported: i32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_abort_job_args {
    pub dom: remote_nonnull_domain,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_add_iothread_args {
    pub dom: remote_nonnull_domain,
    pub iothread_id: u32,
    pub flags: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_attach_device_args {
    pub dom: remote_nonnull_domain,
    pub xml: remote_nonnull_string,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_attach_device_flags_args {
    pub dom: remote_nonnull_domain,
    pub xml: remote_nonnull_string,
    pub flags: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_block_commit_args {
    pub dom: remote_nonnull_domain,
    pub disk: remote_nonnull_string,
    pub base: remote_string,
    pub top: remote_string,
    pub bandwidth: u64,
    pub flags: u32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct remote_domain_block_copy_args {
    pub dom: remote_nonnull_domain,
    pub path: remote_nonnull_string,
    pub destxml: remote_nonnull_string,
    pub params: Vec<remote_typed_param>,
    pub flags: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_block_job_abort_args {
    pub dom: remote_nonnull_domain,
    pub path: remote_nonnull_string,
    pub flags: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_block_job_set_speed_args {
    pub dom: remote_nonnull_domain,
    pub path: remote_nonnull_string,
    pub bandwidth: u64,
    pub flags: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_block_peek_args {
    pub dom: remote_nonnull_domain,
    pub path: remote_nonnull_string,
    pub offset: u64,
    pub size: u32,
    pub flags: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_block_peek_ret {
    pub buffer: Vec<u8>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_block_pull_args {
    pub dom: remote_nonnull_domain,
    pub path: remote_nonnull_string,
    pub bandwidth: u64,
    pub flags: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_block_rebase_args {
    pub dom: remote_nonnull_domain,
    pub path: remote_nonnull_string,
    pub base: remote_string,
    pub bandwidth: u64,
    pub flags: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_block_resize_args {
    pub dom: remote_nonnull_domain,
    pub disk: remote_nonnull_string,
    pub size: u64,
    pub flags: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_block_stats_args {
    pub dom: remote_nonnull_domain,
    pub path: remote_nonnull_string,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_block_stats_flags_args {
    pub dom: remote_nonnull_domain,
    pub path: remote_nonnull_string,
    pub nparams: i32,
    pub flags: u32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct remote_domain_block_stats_flags_ret {
    pub params: Vec<remote_typed_param>,
    pub nparams: i32,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_block_stats_ret {
    pub rd_req: i64,
    pub rd_bytes: i64,
    pub wr_req: i64,
    pub wr_bytes: i64,
    pub errs: i64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_core_dump_args {
    pub dom: remote_nonnull_domain,
    pub to: remote_nonnull_string,
    pub flags: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_core_dump_with_format_args {
    pub dom: remote_nonnull_domain,
    pub to: remote_nonnull_string,
    pub dumpformat: u32,
    pub flags: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_create_args {
    pub dom: remote_nonnull_domain,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_create_with_files_args {
    pub dom: remote_nonnull_domain,
    pub flags: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_create_with_files_ret {
    pub dom: remote_nonnull_domain,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_create_with_flags_args {
    pub dom: remote_nonnull_domain,
    pub flags: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_create_with_flags_ret {
    pub dom: remote_nonnull_domain,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_create_xml_args {
    pub xml_desc: remote_nonnull_string,
    pub flags: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_create_xml_ret {
    pub dom: remote_nonnull_domain,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_create_xml_with_files_args {
    pub xml_desc: remote_nonnull_string,
    pub flags: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_create_xml_with_files_ret {
    pub dom: remote_nonnull_domain,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_define_xml_args {
    pub xml: remote_nonnull_string,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_define_xml_flags_args {
    pub xml: remote_nonnull_string,
    pub flags: u32,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct remote_domain_define_xml_flags_ret {
    pub dom: remote_nonnull_domain,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_define_xml_ret {
    pub dom: remote_nonnull_domain,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_del_iothread_args {
    pub dom: remote_nonnull_domain,
    pub iothread_id: u32,
    pub flags: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_destroy_args {
    pub dom: remote_nonnull_domain,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_destroy_flags_args {
    pub dom: remote_nonnull_domain,
    pub flags: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_detach_device_args {
    pub dom: remote_nonnull_domain,
    pub xml: remote_nonnull_string,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_detach_device_flags_args {
    pub dom: remote_nonnull_domain,
    pub xml: remote_nonnull_string,
    pub flags: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_disk_error {
    pub disk: remote_nonnull_string,
    pub error: i32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_event_balloon_change_msg {
    pub dom: remote_nonnull_domain,
    pub actual: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_event_block_job_2_msg {
    pub callbackID: i32,
    pub dom: remote_nonnull_domain,
    pub dst: remote_nonnull_string,
    pub type_: i32,
    pub status: i32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_event_block_job_msg {
    pub dom: remote_nonnull_domain,
    pub path: remote_nonnull_string,
    pub type_: i32,
    pub status: i32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_event_block_threshold_msg {
    pub callbackID: i32,
    pub dom: remote_nonnull_domain,
    pub dev: remote_nonnull_string,
    pub path: remote_string,
    pub threshold: u64,
    pub excess: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_event_callback_agent_lifecycle_msg {
    pub callbackID: i32,
    pub dom: remote_nonnull_domain,
    pub state: i32,
    pub reason: i32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_event_callback_balloon_change_msg {
    pub callbackID: i32,
    pub msg: remote_domain_event_balloon_change_msg,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_event_callback_block_job_msg {
    pub callbackID: i32,
    pub msg: remote_domain_event_block_job_msg,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_event_callback_control_error_msg {
    pub callbackID: i32,
    pub msg: remote_domain_event_control_error_msg,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_event_callback_device_added_msg {
    pub callbackID: i32,
    pub dom: remote_nonnull_domain,
    pub devAlias: remote_nonnull_string,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_event_callback_device_removal_failed_msg {
    pub callbackID: i32,
    pub dom: remote_nonnull_domain,
    pub devAlias: remote_nonnull_string,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_event_callback_device_removed_msg {
    pub callbackID: i32,
    pub msg: remote_domain_event_device_removed_msg,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_event_callback_disk_change_msg {
    pub callbackID: i32,
    pub msg: remote_domain_event_disk_change_msg,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_event_callback_graphics_msg {
    pub callbackID: i32,
    pub msg: remote_domain_event_graphics_msg,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_event_callback_io_error_msg {
    pub callbackID: i32,
    pub msg: remote_domain_event_io_error_msg,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_event_callback_io_error_reason_msg {
    pub callbackID: i32,
    pub msg: remote_domain_event_io_error_reason_msg,
}

#[derive(Clone, Debug, PartialEq)]
pub struct remote_domain_event_callback_job_completed_msg {
    pub callbackID: i32,
    pub dom: remote_nonnull_domain,
    pub params: Vec<remote_typed_param>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_event_callback_lifecycle_msg {
    pub callbackID: i32,
    pub msg: remote_domain_event_lifecycle_msg,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_event_callback_metadata_change_msg {
    pub callbackID: i32,
    pub dom: remote_nonnull_domain,
    pub type_: i32,
    pub nsuri: remote_string,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_event_callback_migration_iteration_msg {
    pub callbackID: i32,
    pub dom: remote_nonnull_domain,
    pub iteration: i32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_event_callback_pmsuspend_disk_msg {
    pub callbackID: i32,
    pub reason: i32,
    pub msg: remote_domain_event_pmsuspend_disk_msg,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_event_callback_pmsuspend_msg {
    pub callbackID: i32,
    pub reason: i32,
    pub msg: remote_domain_event_pmsuspend_msg,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_event_callback_pmwakeup_msg {
    pub callbackID: i32,
    pub reason: i32,
    pub msg: remote_domain_event_pmwakeup_msg,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_event_callback_reboot_msg {
    pub callbackID: i32,
    pub msg: remote_domain_event_reboot_msg,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_event_callback_rtc_change_msg {
    pub callbackID: i32,
    pub msg: remote_domain_event_rtc_change_msg,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_event_callback_tray_change_msg {
    pub callbackID: i32,
    pub msg: remote_domain_event_tray_change_msg,
}

#[derive(Clone, Debug, PartialEq)]
pub struct remote_domain_event_callback_tunable_msg {
    pub callbackID: i32,
    pub dom: remote_nonnull_domain,
    pub params: Vec<remote_typed_param>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_event_callback_watchdog_msg {
    pub callbackID: i32,
    pub msg: remote_domain_event_watchdog_msg,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_event_control_error_msg {
    pub dom: remote_nonnull_domain,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_event_device_removed_msg {
    pub dom: remote_nonnull_domain,
    pub devAlias: remote_nonnull_string,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_event_disk_change_msg {
    pub dom: remote_nonnull_domain,
    pub oldSrcPath: remote_string,
    pub newSrcPath: remote_string,
    pub devAlias: remote_nonnull_string,
    pub reason: i32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_event_graphics_address {
    pub family: i32,
    pub node: remote_nonnull_string,
    pub service: remote_nonnull_string,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_event_graphics_identity {
    pub type_: remote_nonnull_string,
    pub name: remote_nonnull_string,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_event_graphics_msg {
    pub dom: remote_nonnull_domain,
    pub phase: i32,
    pub local: remote_domain_event_graphics_address,
    pub remote: remote_domain_event_graphics_address,
    pub authScheme: remote_nonnull_string,
    pub subject: Vec<remote_domain_event_graphics_identity>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_event_io_error_msg {
    pub dom: remote_nonnull_domain,
    pub srcPath: remote_nonnull_string,
    pub devAlias: remote_nonnull_string,
    pub action: i32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_event_io_error_reason_msg {
    pub dom: remote_nonnull_domain,
    pub srcPath: remote_nonnull_string,
    pub devAlias: remote_nonnull_string,
    pub action: i32,
    pub reason: remote_nonnull_string,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_event_lifecycle_msg {
    pub dom: remote_nonnull_domain,
    pub event: i32,
    pub detail: i32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_event_pmsuspend_disk_msg {
    pub dom: remote_nonnull_domain,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_event_pmsuspend_msg {
    pub dom: remote_nonnull_domain,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_event_pmwakeup_msg {
    pub dom: remote_nonnull_domain,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_event_reboot_msg {
    pub dom: remote_nonnull_domain,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_event_rtc_change_msg {
    pub dom: remote_nonnull_domain,
    pub offset: i64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_event_tray_change_msg {
    pub dom: remote_nonnull_domain,
    pub devAlias: remote_nonnull_string,
    pub reason: i32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_event_watchdog_msg {
    pub dom: remote_nonnull_domain,
    pub action: i32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_fsfreeze_args {
    pub dom: remote_nonnull_domain,
    pub mountpoints: Vec<remote_nonnull_string>,
    pub flags: u32,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_fsfreeze_ret {
    pub filesystems: i32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_fsinfo {
    pub mountpoint: remote_nonnull_string,
    pub name: remote_nonnull_string,
    pub fstype: remote_nonnull_string,
    pub dev_aliases: Vec<remote_nonnull_string>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_fsthaw_args {
    pub dom: remote_nonnull_domain,
    pub mountpoints: Vec<remote_nonnull_string>,
    pub flags: u32,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_fsthaw_ret {
    pub filesystems: i32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_fstrim_args {
    pub dom: remote_nonnull_domain,
    pub mountPoint: remote_string,
    pub minimum: u64,
    pub flags: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_get_autostart_args {
    pub dom: remote_nonnull_domain,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_get_autostart_ret {
    pub autostart: i32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_get_blkio_parameters_args {
    pub dom: remote_nonnull_domain,
    pub nparams: i32,
    pub flags: u32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct remote_domain_get_blkio_parameters_ret {
    pub params: Vec<remote_typed_param>,
    pub nparams: i32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_get_block_info_args {
    pub dom: remote_nonnull_domain,
    pub path: remote_nonnull_string,
    pub flags: u32,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_get_block_info_ret {
    pub allocation: u64,
    pub capacity: u64,
    pub physical: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_get_block_io_tune_args {
    pub dom: remote_nonnull_domain,
    pub disk: remote_string,
    pub nparams: i32,
    pub flags: u32,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct remote_domain_get_block_io_tune_ret {
    pub params: Vec<remote_typed_param>,
    pub nparams: i32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_get_block_job_info_args {
    pub dom: remote_nonnull_domain,
    pub path: remote_nonnull_string,
    pub flags: u32,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_get_block_job_info_ret {
    pub found: i32,
    pub type_: i32,
    pub bandwidth: u64,
    pub cur: u64,
    pub end: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_get_control_info_args {
    pub dom: remote_nonnull_domain,
    pub flags: u32,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_get_control_info_ret {
    pub state: u32,
    pub details: u32,
    pub stateTime: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_get_cpu_stats_args {
    pub dom: remote_nonnull_domain,
    pub nparams: u32,
    pub start_cpu: i32,
    pub ncpus: u32,
    pub flags: u32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct remote_domain_get_cpu_stats_ret {
    pub params: Vec<remote_typed_param>,
    pub nparams: i32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_get_disk_errors_args {
    pub dom: remote_nonnull_domain,
    pub maxerrors: u32,
    pub flags: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_get_disk_errors_ret {
    pub errors: Vec<remote_domain_disk_error>,
    pub nerrors: i32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_get_emulator_pin_info_args {
    pub dom: remote_nonnull_domain,
    pub maplen: i32,
    pub flags: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_get_emulator_pin_info_ret {
    pub cpumaps: Vec<u8>,
    pub ret: i32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_get_fsinfo_args {
    pub dom: remote_nonnull_domain,
    pub flags: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_get_fsinfo_ret {
    pub info: Vec<remote_domain_fsinfo>,
    pub ret: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_get_guest_vcpus_args {
    pub dom: remote_nonnull_domain,
    pub flags: u32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct remote_domain_get_guest_vcpus_ret {
    pub params: Vec<remote_typed_param>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_get_hostname_args {
    pub dom: remote_nonnull_domain,
    pub flags: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_get_hostname_ret {
    pub hostname: remote_nonnull_string,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_get_info_args {
    pub dom: remote_nonnull_domain,
}

#[derive(Copy, Clone, Debug, Default, Eq, PartialEq)]
pub struct remote_domain_get_info_ret {
    pub state: u8,
    pub maxMem: u64,
    pub memory: u64,
    pub nrVirtCpu: u32,
    pub cpuTime: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_get_interface_parameters_args {
    pub dom: remote_nonnull_domain,
    pub device: remote_nonnull_string,
    pub nparams: i32,
    pub flags: u32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct remote_domain_get_interface_parameters_ret {
    pub params: Vec<remote_typed_param>,
    pub nparams: i32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_get_iothread_info_args {
    pub dom: remote_nonnull_domain,
    pub flags: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_get_iothread_info_ret {
    pub info: Vec<remote_domain_iothread_info>,
    pub ret: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_get_job_info_args {
    pub dom: remote_nonnull_domain,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_get_job_info_ret {
    pub type_: i32,
    pub timeElapsed: u64,
    pub timeRemaining: u64,
    pub dataTotal: u64,
    pub dataProcessed: u64,
    pub dataRemaining: u64,
    pub memTotal: u64,
    pub memProcessed: u64,
    pub memRemaining: u64,
    pub fileTotal: u64,
    pub fileProcessed: u64,
    pub fileRemaining: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_get_job_stats_args {
    pub dom: remote_nonnull_domain,
    pub flags: u32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct remote_domain_get_job_stats_ret {
    pub type_: i32,
    pub params: Vec<remote_typed_param>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_get_max_memory_args {
    pub dom: remote_nonnull_domain,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_get_max_memory_ret {
    pub memory: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_get_max_vcpus_args {
    pub dom: remote_nonnull_domain,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_get_max_vcpus_ret {
    pub num: i32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_get_memory_parameters_args {
    pub dom: remote_nonnull_domain,
    pub nparams: i32,
    pub flags: u32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct remote_domain_get_memory_parameters_ret {
    pub params: Vec<remote_typed_param>,
    pub nparams: i32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_get_metadata_args {
    pub dom: remote_nonnull_domain,
    pub type_: i32,
    pub uri: remote_string,
    pub flags: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_get_metadata_ret {
    pub metadata: remote_nonnull_string,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_get_numa_parameters_args {
    pub dom: remote_nonnull_domain,
    pub nparams: i32,
    pub flags: u32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct remote_domain_get_numa_parameters_ret {
    pub params: Vec<remote_typed_param>,
    pub nparams: i32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_get_os_type_args {
    pub dom: remote_nonnull_domain,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_get_os_type_ret {
    pub type_: remote_nonnull_string,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_get_perf_events_args {
    pub dom: remote_nonnull_domain,
    pub flags: u32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct remote_domain_get_perf_events_ret {
    pub params: Vec<remote_typed_param>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_get_scheduler_parameters_args {
    pub dom: remote_nonnull_domain,
    pub nparams: i32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_get_scheduler_parameters_flags_args {
    pub dom: remote_nonnull_domain,
    pub nparams: i32,
    pub flags: u32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct remote_domain_get_scheduler_parameters_flags_ret {
    pub params: Vec<remote_typed_param>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct remote_domain_get_scheduler_parameters_ret {
    pub params: Vec<remote_typed_param>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_get_scheduler_type_args {
    pub dom: remote_nonnull_domain,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_get_scheduler_type_ret {
    pub type_: remote_nonnull_string,
    pub nparams: i32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_get_security_label_args {
    pub dom: remote_nonnull_domain,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_get_security_label_list_args {
    pub dom: remote_nonnull_domain,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_get_security_label_list_ret {
    pub labels: Vec<remote_domain_get_security_label_ret>,
    pub ret: i32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_get_security_label_ret {
    pub label: Vec<i8>,
    pub enforcing: i32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_get_state_args {
    pub dom: remote_nonnull_domain,
    pub flags: u32,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_get_state_ret {
    pub state: i32,
    pub reason: i32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_get_time_args {
    pub dom: remote_nonnull_domain,
    pub flags: u32,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_get_time_ret {
    pub seconds: i64,
    pub nseconds: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_get_vcpu_pin_info_args {
    pub dom: remote_nonnull_domain,
    pub ncpumaps: i32,
    pub maplen: i32,
    pub flags: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_get_vcpu_pin_info_ret {
    pub cpumaps: Vec<u8>,
    pub num: i32,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct remote_domain_get_vcpus_args {
    pub dom: remote_nonnull_domain,
    pub maxinfo: i32,
    pub maplen: i32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_get_vcpus_flags_args {
    pub dom: remote_nonnull_domain,
    pub flags: u32,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_get_vcpus_flags_ret {
    pub num: i32,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct remote_domain_get_vcpus_ret {
    pub info: Vec<remote_vcpu_info>,
    pub cpumaps: Vec<u8>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct remote_domain_get_xml_desc_args {
    pub dom: remote_nonnull_domain,
    pub flags: u32,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct remote_domain_get_xml_desc_ret {
    pub xml: remote_nonnull_string,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_has_current_snapshot_args {
    pub dom: remote_nonnull_domain,
    pub flags: u32,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_has_current_snapshot_ret {
    pub result: i32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_has_managed_save_image_args {
    pub dom: remote_nonnull_domain,
    pub flags: u32,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_has_managed_save_image_ret {
    pub result: i32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_inject_nmi_args {
    pub dom: remote_nonnull_domain,
    pub flags: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_interface {
    pub name: remote_nonnull_string,
    pub hwaddr: remote_string,
    pub addrs: Vec<remote_domain_ip_addr>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_interface_addresses_args {
    pub dom: remote_nonnull_domain,
    pub source: u32,
    pub flags: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_interface_addresses_ret {
    pub ifaces: Vec<remote_domain_interface>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_interface_stats_args {
    pub dom: remote_nonnull_domain,
    pub path: remote_nonnull_string,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_interface_stats_ret {
    pub rx_bytes: i64,
    pub rx_packets: i64,
    pub rx_errs: i64,
    pub rx_drop: i64,
    pub tx_bytes: i64,
    pub tx_packets: i64,
    pub tx_errs: i64,
    pub tx_drop: i64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_iothread_info {
    pub iothread_id: u32,
    pub cpumap: Vec<u8>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_ip_addr {
    pub type_: i32,
    pub addr: remote_nonnull_string,
    pub prefix: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_is_active_args {
    pub dom: remote_nonnull_domain,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_is_active_ret {
    pub active: i32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_is_persistent_args {
    pub dom: remote_nonnull_domain,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_is_persistent_ret {
    pub persistent: i32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_is_updated_args {
    pub dom: remote_nonnull_domain,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_is_updated_ret {
    pub updated: i32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_list_all_snapshots_args {
    pub dom: remote_nonnull_domain,
    pub need_results: i32,
    pub flags: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_list_all_snapshots_ret {
    pub snapshots: Vec<remote_nonnull_domain_snapshot>,
    pub ret: i32,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_lookup_by_id_args {
    pub id: i32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_lookup_by_id_ret {
    pub dom: remote_nonnull_domain,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_lookup_by_name_args {
    pub name: remote_nonnull_string,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_lookup_by_name_ret {
    pub dom: remote_nonnull_domain,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_lookup_by_uuid_args {
    pub uuid: remote_uuid,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_lookup_by_uuid_ret {
    pub dom: remote_nonnull_domain,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_managed_save_args {
    pub dom: remote_nonnull_domain,
    pub flags: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_managed_save_remove_args {
    pub dom: remote_nonnull_domain,
    pub flags: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_memory_peek_args {
    pub dom: remote_nonnull_domain,
    pub offset: u64,
    pub size: u32,
    pub flags: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_memory_peek_ret {
    pub buffer: Vec<u8>,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_memory_stat {
    pub tag: i32,
    pub val: u64,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct remote_domain_memory_stats_args {
    pub dom: remote_nonnull_domain,
    pub maxStats: u32,
    pub flags: u32,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct remote_domain_memory_stats_ret {
    pub stats: Vec<remote_domain_memory_stat>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_migrate_begin3_args {
    pub dom: remote_nonnull_domain,
    pub xmlin: remote_string,
    pub flags: u64,
    pub dname: remote_string,
    pub resource: u64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct remote_domain_migrate_begin3_params_args {
    pub dom: remote_nonnull_domain,
    pub params: Vec<remote_typed_param>,
    pub flags: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_migrate_begin3_params_ret {
    pub cookie_out: Vec<u8>,
    pub xml: remote_nonnull_string,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_migrate_begin3_ret {
    pub cookie_out: Vec<u8>,
    pub xml: remote_nonnull_string,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_migrate_confirm3_args {
    pub dom: remote_nonnull_domain,
    pub cookie_in: Vec<u8>,
    pub flags: u64,
    pub cancelled: i32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct remote_domain_migrate_confirm3_params_args {
    pub dom: remote_nonnull_domain,
    pub params: Vec<remote_typed_param>,
    pub cookie_in: Vec<u8>,
    pub flags: u32,
    pub cancelled: i32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_migrate_finish2_args {
    pub dname: remote_nonnull_string,
    pub cookie: Vec<u8>,
    pub uri: remote_nonnull_string,
    pub flags: u64,
    pub retcode: i32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_migrate_finish2_ret {
    pub ddom: remote_nonnull_domain,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_migrate_finish3_args {
    pub dname: remote_nonnull_string,
    pub cookie_in: Vec<u8>,
    pub dconnuri: remote_string,
    pub uri: remote_string,
    pub flags: u64,
    pub cancelled: i32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct remote_domain_migrate_finish3_params_args {
    pub params: Vec<remote_typed_param>,
    pub cookie_in: Vec<u8>,
    pub flags: u32,
    pub cancelled: i32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_migrate_finish3_params_ret {
    pub dom: remote_nonnull_domain,
    pub cookie_out: Vec<u8>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_migrate_finish3_ret {
    pub dom: remote_nonnull_domain,
    pub cookie_out: Vec<u8>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_migrate_finish_args {
    pub dname: remote_nonnull_string,
    pub cookie: Vec<u8>,
    pub uri: remote_nonnull_string,
    pub flags: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_migrate_finish_ret {
    pub ddom: remote_nonnull_domain,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_migrate_get_compression_cache_args {
    pub dom: remote_nonnull_domain,
    pub flags: u32,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_migrate_get_compression_cache_ret {
    pub cacheSize: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_migrate_get_max_speed_args {
    pub dom: remote_nonnull_domain,
    pub flags: u32,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_migrate_get_max_speed_ret {
    pub bandwidth: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_migrate_perform3_args {
    pub dom: remote_nonnull_domain,
    pub xmlin: remote_string,
    pub cookie_in: Vec<u8>,
    pub dconnuri: remote_string,
    pub uri: remote_string,
    pub flags: u64,
    pub dname: remote_string,
    pub resource: u64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct remote_domain_migrate_perform3_params_args {
    pub dom: remote_nonnull_domain,
    pub dconnuri: remote_string,
    pub params: Vec<remote_typed_param>,
    pub cookie_in: Vec<u8>,
    pub flags: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_migrate_perform3_params_ret {
    pub cookie_out: Vec<u8>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_migrate_perform3_ret {
    pub cookie_out: Vec<u8>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_migrate_perform_args {
    pub dom: remote_nonnull_domain,
    pub cookie: Vec<u8>,
    pub uri: remote_nonnull_string,
    pub flags: u64,
    pub dname: remote_string,
    pub resource: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_migrate_prepare2_args {
    pub uri_in: remote_string,
    pub flags: u64,
    pub dname: remote_string,
    pub resource: u64,
    pub dom_xml: remote_nonnull_string,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_migrate_prepare2_ret {
    pub cookie: Vec<u8>,
    pub uri_out: remote_string,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_migrate_prepare3_args {
    pub cookie_in: Vec<u8>,
    pub uri_in: remote_string,
    pub flags: u64,
    pub dname: remote_string,
    pub resource: u64,
    pub dom_xml: remote_nonnull_string,
}

#[derive(Clone, Debug, PartialEq)]
pub struct remote_domain_migrate_prepare3_params_args {
    pub params: Vec<remote_typed_param>,
    pub cookie_in: Vec<u8>,
    pub flags: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_migrate_prepare3_params_ret {
    pub cookie_out: Vec<u8>,
    pub uri_out: remote_string,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_migrate_prepare3_ret {
    pub cookie_out: Vec<u8>,
    pub uri_out: remote_string,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_migrate_prepare_args {
    pub uri_in: remote_string,
    pub flags: u64,
    pub dname: remote_string,
    pub resource: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_migrate_prepare_ret {
    pub cookie: Vec<u8>,
    pub uri_out: remote_string,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_migrate_prepare_tunnel3_args {
    pub cookie_in: Vec<u8>,
    pub flags: u64,
    pub dname: remote_string,
    pub resource: u64,
    pub dom_xml: remote_nonnull_string,
}

#[derive(Clone, Debug, PartialEq)]
pub struct remote_domain_migrate_prepare_tunnel3_params_args {
    pub params: Vec<remote_typed_param>,
    pub cookie_in: Vec<u8>,
    pub flags: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_migrate_prepare_tunnel3_params_ret {
    pub cookie_out: Vec<u8>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_migrate_prepare_tunnel3_ret {
    pub cookie_out: Vec<u8>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_migrate_prepare_tunnel_args {
    pub flags: u64,
    pub dname: remote_string,
    pub resource: u64,
    pub dom_xml: remote_nonnull_string,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_migrate_set_compression_cache_args {
    pub dom: remote_nonnull_domain,
    pub cacheSize: u64,
    pub flags: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_migrate_set_max_downtime_args {
    pub dom: remote_nonnull_domain,
    pub downtime: u64,
    pub flags: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_migrate_set_max_speed_args {
    pub dom: remote_nonnull_domain,
    pub bandwidth: u64,
    pub flags: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_migrate_start_post_copy_args {
    pub dom: remote_nonnull_domain,
    pub flags: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_open_channel_args {
    pub dom: remote_nonnull_domain,
    pub name: remote_string,
    pub flags: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_open_console_args {
    pub dom: remote_nonnull_domain,
    pub dev_name: remote_string,
    pub flags: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_open_graphics_args {
    pub dom: remote_nonnull_domain,
    pub idx: u32,
    pub flags: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_open_graphics_fd_args {
    pub dom: remote_nonnull_domain,
    pub idx: u32,
    pub flags: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_pin_emulator_args {
    pub dom: remote_nonnull_domain,
    pub cpumap: Vec<u8>,
    pub flags: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_pin_iothread_args {
    pub dom: remote_nonnull_domain,
    pub iothreads_id: u32,
    pub cpumap: Vec<u8>,
    pub flags: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_pin_vcpu_args {
    pub dom: remote_nonnull_domain,
    pub vcpu: u32,
    pub cpumap: Vec<u8>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_pin_vcpu_flags_args {
    pub dom: remote_nonnull_domain,
    pub vcpu: u32,
    pub cpumap: Vec<u8>,
    pub flags: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_pm_suspend_for_duration_args {
    pub dom: remote_nonnull_domain,
    pub target: u32,
    pub duration: u64,
    pub flags: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_pm_wakeup_args {
    pub dom: remote_nonnull_domain,
    pub flags: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_reboot_args {
    pub dom: remote_nonnull_domain,
    pub flags: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_rename_args {
    pub dom: remote_nonnull_domain,
    pub new_name: remote_string,
    pub flags: u32,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_rename_ret {
    pub retcode: i32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_reset_args {
    pub dom: remote_nonnull_domain,
    pub flags: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_restore_args {
    pub from: remote_nonnull_string,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_restore_flags_args {
    pub from: remote_nonnull_string,
    pub dxml: remote_string,
    pub flags: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_resume_args {
    pub dom: remote_nonnull_domain,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_revert_to_snapshot_args {
    pub snap: remote_nonnull_domain_snapshot,
    pub flags: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_save_args {
    pub dom: remote_nonnull_domain,
    pub to: remote_nonnull_string,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_save_flags_args {
    pub dom: remote_nonnull_domain,
    pub to: remote_nonnull_string,
    pub dxml: remote_string,
    pub flags: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_save_image_define_xml_args {
    pub file: remote_nonnull_string,
    pub dxml: remote_nonnull_string,
    pub flags: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_save_image_get_xml_desc_args {
    pub file: remote_nonnull_string,
    pub flags: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_save_image_get_xml_desc_ret {
    pub xml: remote_nonnull_string,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_screenshot_args {
    pub dom: remote_nonnull_domain,
    pub screen: u32,
    pub flags: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_screenshot_ret {
    pub mime: remote_string,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_send_key_args {
    pub dom: remote_nonnull_domain,
    pub codeset: u32,
    pub holdtime: u32,
    pub keycodes: Vec<u32>,
    pub flags: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_send_process_signal_args {
    pub dom: remote_nonnull_domain,
    pub pid_value: i64,
    pub signum: u32,
    pub flags: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_set_autostart_args {
    pub dom: remote_nonnull_domain,
    pub autostart: i32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct remote_domain_set_blkio_parameters_args {
    pub dom: remote_nonnull_domain,
    pub params: Vec<remote_typed_param>,
    pub flags: u32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct remote_domain_set_block_io_tune_args {
    pub dom: remote_nonnull_domain,
    pub disk: remote_nonnull_string,
    pub params: Vec<remote_typed_param>,
    pub flags: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_set_block_threshold_args {
    pub dom: remote_nonnull_domain,
    pub dev: remote_nonnull_string,
    pub threshold: u64,
    pub flags: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_set_guest_vcpus_args {
    pub dom: remote_nonnull_domain,
    pub cpumap: remote_nonnull_string,
    pub state: i32,
    pub flags: u32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct remote_domain_set_interface_parameters_args {
    pub dom: remote_nonnull_domain,
    pub device: remote_nonnull_string,
    pub params: Vec<remote_typed_param>,
    pub flags: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_set_max_memory_args {
    pub dom: remote_nonnull_domain,
    pub memory: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_set_memory_args {
    pub dom: remote_nonnull_domain,
    pub memory: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_set_memory_flags_args {
    pub dom: remote_nonnull_domain,
    pub memory: u64,
    pub flags: u32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct remote_domain_set_memory_parameters_args {
    pub dom: remote_nonnull_domain,
    pub params: Vec<remote_typed_param>,
    pub flags: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_set_memory_stats_period_args {
    pub dom: remote_nonnull_domain,
    pub period: i32,
    pub flags: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_set_metadata_args {
    pub dom: remote_nonnull_domain,
    pub type_: i32,
    pub metadata: remote_string,
    pub key: remote_string,
    pub uri: remote_string,
    pub flags: u32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct remote_domain_set_numa_parameters_args {
    pub dom: remote_nonnull_domain,
    pub params: Vec<remote_typed_param>,
    pub flags: u32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct remote_domain_set_perf_events_args {
    pub dom: remote_nonnull_domain,
    pub params: Vec<remote_typed_param>,
    pub flags: u32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct remote_domain_set_scheduler_parameters_args {
    pub dom: remote_nonnull_domain,
    pub params: Vec<remote_typed_param>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct remote_domain_set_scheduler_parameters_flags_args {
    pub dom: remote_nonnull_domain,
    pub params: Vec<remote_typed_param>,
    pub flags: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_set_time_args {
    pub dom: remote_nonnull_domain,
    pub seconds: i64,
    pub nseconds: u32,
    pub flags: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_set_user_password_args {
    pub dom: remote_nonnull_domain,
    pub user: remote_string,
    pub password: remote_string,
    pub flags: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_set_vcpu_args {
    pub dom: remote_nonnull_domain,
    pub cpumap: remote_nonnull_string,
    pub state: i32,
    pub flags: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_set_vcpus_args {
    pub dom: remote_nonnull_domain,
    pub nvcpus: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_set_vcpus_flags_args {
    pub dom: remote_nonnull_domain,
    pub nvcpus: u32,
    pub flags: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_shutdown_args {
    pub dom: remote_nonnull_domain,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_shutdown_flags_args {
    pub dom: remote_nonnull_domain,
    pub flags: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_snapshot_create_xml_args {
    pub dom: remote_nonnull_domain,
    pub xml_desc: remote_nonnull_string,
    pub flags: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_snapshot_create_xml_ret {
    pub snap: remote_nonnull_domain_snapshot,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_snapshot_current_args {
    pub dom: remote_nonnull_domain,
    pub flags: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_snapshot_current_ret {
    pub snap: remote_nonnull_domain_snapshot,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_snapshot_delete_args {
    pub snap: remote_nonnull_domain_snapshot,
    pub flags: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_snapshot_get_parent_args {
    pub snap: remote_nonnull_domain_snapshot,
    pub flags: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_snapshot_get_parent_ret {
    pub snap: remote_nonnull_domain_snapshot,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_snapshot_get_xml_desc_args {
    pub snap: remote_nonnull_domain_snapshot,
    pub flags: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_snapshot_get_xml_desc_ret {
    pub xml: remote_nonnull_string,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_snapshot_has_metadata_args {
    pub snap: remote_nonnull_domain_snapshot,
    pub flags: u32,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_snapshot_has_metadata_ret {
    pub metadata: i32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_snapshot_is_current_args {
    pub snap: remote_nonnull_domain_snapshot,
    pub flags: u32,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_snapshot_is_current_ret {
    pub current: i32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_snapshot_list_all_children_args {
    pub snapshot: remote_nonnull_domain_snapshot,
    pub need_results: i32,
    pub flags: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_snapshot_list_all_children_ret {
    pub snapshots: Vec<remote_nonnull_domain_snapshot>,
    pub ret: i32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_snapshot_list_children_names_args {
    pub snap: remote_nonnull_domain_snapshot,
    pub maxnames: i32,
    pub flags: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_snapshot_list_children_names_ret {
    pub names: Vec<remote_nonnull_string>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_snapshot_list_names_args {
    pub dom: remote_nonnull_domain,
    pub maxnames: i32,
    pub flags: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_snapshot_list_names_ret {
    pub names: Vec<remote_nonnull_string>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_snapshot_lookup_by_name_args {
    pub dom: remote_nonnull_domain,
    pub name: remote_nonnull_string,
    pub flags: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_snapshot_lookup_by_name_ret {
    pub snap: remote_nonnull_domain_snapshot,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_snapshot_num_args {
    pub dom: remote_nonnull_domain,
    pub flags: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_snapshot_num_children_args {
    pub snap: remote_nonnull_domain_snapshot,
    pub flags: u32,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_snapshot_num_children_ret {
    pub num: i32,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_snapshot_num_ret {
    pub num: i32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct remote_domain_stats_record {
    pub dom: remote_nonnull_domain,
    pub params: Vec<remote_typed_param>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_suspend_args {
    pub dom: remote_nonnull_domain,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_undefine_args {
    pub dom: remote_nonnull_domain,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_undefine_flags_args {
    pub dom: remote_nonnull_domain,
    pub flags: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_domain_update_device_flags_args {
    pub dom: remote_nonnull_domain,
    pub xml: remote_nonnull_string,
    pub flags: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_error {
    pub code: i32,
    pub domain: i32,
    pub message: remote_string,
    pub level: i32,
    pub dom: remote_domain,
    pub str1: remote_string,
    pub str2: remote_string,
    pub str3: remote_string,
    pub int1: i32,
    pub int2: i32,
    pub net: remote_network,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct remote_interface_change_begin_args {
    pub flags: u32,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct remote_interface_change_commit_args {
    pub flags: u32,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct remote_interface_change_rollback_args {
    pub flags: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_interface_create_args {
    pub iface: remote_nonnull_interface,
    pub flags: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_interface_define_xml_args {
    pub xml: remote_nonnull_string,
    pub flags: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_interface_define_xml_ret {
    pub iface: remote_nonnull_interface,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_interface_destroy_args {
    pub iface: remote_nonnull_interface,
    pub flags: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_interface_get_xml_desc_args {
    pub iface: remote_nonnull_interface,
    pub flags: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_interface_get_xml_desc_ret {
    pub xml: remote_nonnull_string,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_interface_is_active_args {
    pub iface: remote_nonnull_interface,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct remote_interface_is_active_ret {
    pub active: i32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_interface_lookup_by_mac_string_args {
    pub mac: remote_nonnull_string,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_interface_lookup_by_mac_string_ret {
    pub iface: remote_nonnull_interface,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_interface_lookup_by_name_args {
    pub name: remote_nonnull_string,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_interface_lookup_by_name_ret {
    pub iface: remote_nonnull_interface,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_interface_undefine_args {
    pub iface: remote_nonnull_interface,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_network_create_args {
    pub net: remote_nonnull_network,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_network_create_xml_args {
    pub xml: remote_nonnull_string,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_network_create_xml_ret {
    pub net: remote_nonnull_network,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_network_define_xml_args {
    pub xml: remote_nonnull_string,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_network_define_xml_ret {
    pub net: remote_nonnull_network,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_network_destroy_args {
    pub net: remote_nonnull_network,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_network_dhcp_lease {
    pub iface: remote_nonnull_string,
    pub expirytime: i64,
    pub type_: i32,
    pub mac: remote_string,
    pub iaid: remote_string,
    pub ipaddr: remote_nonnull_string,
    pub prefix: u32,
    pub hostname: remote_string,
    pub clientid: remote_string,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_network_event_lifecycle_msg {
    pub callbackID: i32,
    pub net: remote_nonnull_network,
    pub event: i32,
    pub detail: i32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_network_get_autostart_args {
    pub net: remote_nonnull_network,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct remote_network_get_autostart_ret {
    pub autostart: i32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_network_get_bridge_name_args {
    pub net: remote_nonnull_network,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_network_get_bridge_name_ret {
    pub name: remote_nonnull_string,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_network_get_dhcp_leases_args {
    pub net: remote_nonnull_network,
    pub mac: remote_string,
    pub need_results: i32,
    pub flags: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_network_get_dhcp_leases_ret {
    pub leases: Vec<remote_network_dhcp_lease>,
    pub ret: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_network_get_xml_desc_args {
    pub net: remote_nonnull_network,
    pub flags: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_network_get_xml_desc_ret {
    pub xml: remote_nonnull_string,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_network_is_active_args {
    pub net: remote_nonnull_network,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct remote_network_is_active_ret {
    pub active: i32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_network_is_persistent_args {
    pub net: remote_nonnull_network,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct remote_network_is_persistent_ret {
    pub persistent: i32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_network_lookup_by_name_args {
    pub name: remote_nonnull_string,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_network_lookup_by_name_ret {
    pub net: remote_nonnull_network,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct remote_network_lookup_by_uuid_args {
    pub uuid: remote_uuid,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_network_lookup_by_uuid_ret {
    pub net: remote_nonnull_network,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_network_set_autostart_args {
    pub net: remote_nonnull_network,
    pub autostart: i32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_network_undefine_args {
    pub net: remote_nonnull_network,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_network_update_args {
    pub net: remote_nonnull_network,
    pub command: u32,
    pub section: u32,
    pub parentIndex: i32,
    pub xml: remote_nonnull_string,
    pub flags: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_node_alloc_pages_args {
    pub pageSizes: Vec<u32>,
    pub pageCounts: Vec<u64>,
    pub startCell: i32,
    pub cellCount: u32,
    pub flags: u32,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct remote_node_alloc_pages_ret {
    pub ret: i32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_node_device_create_xml_args {
    pub xml_desc: remote_nonnull_string,
    pub flags: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_node_device_create_xml_ret {
    pub dev: remote_nonnull_node_device,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_node_device_destroy_args {
    pub name: remote_nonnull_string,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_node_device_detach_flags_args {
    pub name: remote_nonnull_string,
    pub driverName: remote_string,
    pub flags: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_node_device_dettach_args {
    pub name: remote_nonnull_string,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_node_device_event_lifecycle_msg {
    pub callbackID: i32,
    pub dev: remote_nonnull_node_device,
    pub event: i32,
    pub detail: i32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_node_device_event_update_msg {
    pub callbackID: i32,
    pub dev: remote_nonnull_node_device,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_node_device_get_parent_args {
    pub name: remote_nonnull_string,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_node_device_get_parent_ret {
    pub parent: remote_string,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_node_device_get_xml_desc_args {
    pub name: remote_nonnull_string,
    pub flags: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_node_device_get_xml_desc_ret {
    pub xml: remote_nonnull_string,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_node_device_list_caps_args {
    pub name: remote_nonnull_string,
    pub maxnames: i32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_node_device_list_caps_ret {
    pub names: Vec<remote_nonnull_string>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_node_device_lookup_by_name_args {
    pub name: remote_nonnull_string,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_node_device_lookup_by_name_ret {
    pub dev: remote_nonnull_node_device,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_node_device_lookup_scsi_host_by_wwn_args {
    pub wwnn: remote_nonnull_string,
    pub wwpn: remote_nonnull_string,
    pub flags: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_node_device_lookup_scsi_host_by_wwn_ret {
    pub dev: remote_nonnull_node_device,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_node_device_num_of_caps_args {
    pub name: remote_nonnull_string,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct remote_node_device_num_of_caps_ret {
    pub num: i32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_node_device_re_attach_args {
    pub name: remote_nonnull_string,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_node_device_reset_args {
    pub name: remote_nonnull_string,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct remote_node_get_cells_free_memory_args {
    pub startCell: i32,
    pub maxcells: i32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_node_get_cells_free_memory_ret {
    pub cells: Vec<u64>,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct remote_node_get_cpu_map_args {
    pub need_map: i32,
    pub need_online: i32,
    pub flags: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_node_get_cpu_map_ret {
    pub cpumap: Vec<u8>,
    pub online: u32,
    pub ret: i32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_node_get_cpu_stats {
    pub field: remote_nonnull_string,
    pub value: u64,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct remote_node_get_cpu_stats_args {
    pub cpuNum: i32,
    pub nparams: i32,
    pub flags: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_node_get_cpu_stats_ret {
    pub params: Vec<remote_node_get_cpu_stats>,
    pub nparams: i32,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct remote_node_get_free_memory_ret {
    pub freeMem: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_node_get_free_pages_args {
    pub pages: Vec<u32>,
    pub startCell: i32,
    pub cellCount: u32,
    pub flags: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_node_get_free_pages_ret {
    pub counts: Vec<u64>,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct remote_node_get_info_ret {
    pub model: [i8; 32i64 as usize],
    pub memory: u64,
    pub cpus: i32,
    pub mhz: i32,
    pub nodes: i32,
    pub sockets: i32,
    pub cores: i32,
    pub threads: i32,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct remote_node_get_memory_parameters_args {
    pub nparams: i32,
    pub flags: u32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct remote_node_get_memory_parameters_ret {
    pub params: Vec<remote_typed_param>,
    pub nparams: i32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_node_get_memory_stats {
    pub field: remote_nonnull_string,
    pub value: u64,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct remote_node_get_memory_stats_args {
    pub nparams: i32,
    pub cellNum: i32,
    pub flags: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_node_get_memory_stats_ret {
    pub params: Vec<remote_node_get_memory_stats>,
    pub nparams: i32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_node_get_security_model_ret {
    pub model: Vec<i8>,
    pub doi: Vec<i8>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_node_list_devices_args {
    pub cap: remote_string,
    pub maxnames: i32,
    pub flags: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_node_list_devices_ret {
    pub names: Vec<remote_nonnull_string>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_node_num_of_devices_args {
    pub cap: remote_string,
    pub flags: u32,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct remote_node_num_of_devices_ret {
    pub num: i32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct remote_node_set_memory_parameters_args {
    pub params: Vec<remote_typed_param>,
    pub flags: u32,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct remote_node_suspend_for_duration_args {
    pub target: u32,
    pub duration: u64,
    pub flags: u32,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct remote_nonnull_domain {
    pub name: remote_nonnull_string,
    pub uuid: remote_uuid,
    pub id: i32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_nonnull_domain_snapshot {
    pub name: remote_nonnull_string,
    pub dom: remote_nonnull_domain,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_nonnull_interface {
    pub name: remote_nonnull_string,
    pub mac: remote_nonnull_string,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_nonnull_network {
    pub name: remote_nonnull_string,
    pub uuid: remote_uuid,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_nonnull_node_device {
    pub name: remote_nonnull_string,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_nonnull_nwfilter {
    pub name: remote_nonnull_string,
    pub uuid: remote_uuid,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_nonnull_secret {
    pub uuid: remote_uuid,
    pub usageType: i32,
    pub usageID: remote_nonnull_string,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct remote_nonnull_storage_pool {
    pub name: remote_nonnull_string,
    pub uuid: remote_uuid,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_nonnull_storage_vol {
    pub pool: remote_nonnull_string,
    pub name: remote_nonnull_string,
    pub key: remote_nonnull_string,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct remote_nonnull_string(pub String);

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_nwfilter_define_xml_args {
    pub xml: remote_nonnull_string,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_nwfilter_define_xml_ret {
    pub nwfilter: remote_nonnull_nwfilter,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_nwfilter_get_xml_desc_args {
    pub nwfilter: remote_nonnull_nwfilter,
    pub flags: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_nwfilter_get_xml_desc_ret {
    pub xml: remote_nonnull_string,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_nwfilter_lookup_by_name_args {
    pub name: remote_nonnull_string,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_nwfilter_lookup_by_name_ret {
    pub nwfilter: remote_nonnull_nwfilter,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct remote_nwfilter_lookup_by_uuid_args {
    pub uuid: remote_uuid,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_nwfilter_lookup_by_uuid_ret {
    pub nwfilter: remote_nonnull_nwfilter,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_nwfilter_undefine_args {
    pub nwfilter: remote_nonnull_nwfilter,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum remote_procedure {
    REMOTE_PROC_CONNECT_OPEN = 1isize,
    REMOTE_PROC_CONNECT_CLOSE = 2isize,
    REMOTE_PROC_CONNECT_GET_TYPE = 3isize,
    REMOTE_PROC_CONNECT_GET_VERSION = 4isize,
    REMOTE_PROC_CONNECT_GET_MAX_VCPUS = 5isize,
    REMOTE_PROC_NODE_GET_INFO = 6isize,
    REMOTE_PROC_CONNECT_GET_CAPABILITIES = 7isize,
    REMOTE_PROC_DOMAIN_ATTACH_DEVICE = 8isize,
    REMOTE_PROC_DOMAIN_CREATE = 9isize,
    REMOTE_PROC_DOMAIN_CREATE_XML = 10isize,
    REMOTE_PROC_DOMAIN_DEFINE_XML = 11isize,
    REMOTE_PROC_DOMAIN_DESTROY = 12isize,
    REMOTE_PROC_DOMAIN_DETACH_DEVICE = 13isize,
    REMOTE_PROC_DOMAIN_GET_XML_DESC = 14isize,
    REMOTE_PROC_DOMAIN_GET_AUTOSTART = 15isize,
    REMOTE_PROC_DOMAIN_GET_INFO = 16isize,
    REMOTE_PROC_DOMAIN_GET_MAX_MEMORY = 17isize,
    REMOTE_PROC_DOMAIN_GET_MAX_VCPUS = 18isize,
    REMOTE_PROC_DOMAIN_GET_OS_TYPE = 19isize,
    REMOTE_PROC_DOMAIN_GET_VCPUS = 20isize,
    REMOTE_PROC_CONNECT_LIST_DEFINED_DOMAINS = 21isize,
    REMOTE_PROC_DOMAIN_LOOKUP_BY_ID = 22isize,
    REMOTE_PROC_DOMAIN_LOOKUP_BY_NAME = 23isize,
    REMOTE_PROC_DOMAIN_LOOKUP_BY_UUID = 24isize,
    REMOTE_PROC_CONNECT_NUM_OF_DEFINED_DOMAINS = 25isize,
    REMOTE_PROC_DOMAIN_PIN_VCPU = 26isize,
    REMOTE_PROC_DOMAIN_REBOOT = 27isize,
    REMOTE_PROC_DOMAIN_RESUME = 28isize,
    REMOTE_PROC_DOMAIN_SET_AUTOSTART = 29isize,
    REMOTE_PROC_DOMAIN_SET_MAX_MEMORY = 30isize,
    REMOTE_PROC_DOMAIN_SET_MEMORY = 31isize,
    REMOTE_PROC_DOMAIN_SET_VCPUS = 32isize,
    REMOTE_PROC_DOMAIN_SHUTDOWN = 33isize,
    REMOTE_PROC_DOMAIN_SUSPEND = 34isize,
    REMOTE_PROC_DOMAIN_UNDEFINE = 35isize,
    REMOTE_PROC_CONNECT_LIST_DEFINED_NETWORKS = 36isize,
    REMOTE_PROC_CONNECT_LIST_DOMAINS = 37isize,
    REMOTE_PROC_CONNECT_LIST_NETWORKS = 38isize,
    REMOTE_PROC_NETWORK_CREATE = 39isize,
    REMOTE_PROC_NETWORK_CREATE_XML = 40isize,
    REMOTE_PROC_NETWORK_DEFINE_XML = 41isize,
    REMOTE_PROC_NETWORK_DESTROY = 42isize,
    REMOTE_PROC_NETWORK_GET_XML_DESC = 43isize,
    REMOTE_PROC_NETWORK_GET_AUTOSTART = 44isize,
    REMOTE_PROC_NETWORK_GET_BRIDGE_NAME = 45isize,
    REMOTE_PROC_NETWORK_LOOKUP_BY_NAME = 46isize,
    REMOTE_PROC_NETWORK_LOOKUP_BY_UUID = 47isize,
    REMOTE_PROC_NETWORK_SET_AUTOSTART = 48isize,
    REMOTE_PROC_NETWORK_UNDEFINE = 49isize,
    REMOTE_PROC_CONNECT_NUM_OF_DEFINED_NETWORKS = 50isize,
    REMOTE_PROC_CONNECT_NUM_OF_DOMAINS = 51isize,
    REMOTE_PROC_CONNECT_NUM_OF_NETWORKS = 52isize,
    REMOTE_PROC_DOMAIN_CORE_DUMP = 53isize,
    REMOTE_PROC_DOMAIN_RESTORE = 54isize,
    REMOTE_PROC_DOMAIN_SAVE = 55isize,
    REMOTE_PROC_DOMAIN_GET_SCHEDULER_TYPE = 56isize,
    REMOTE_PROC_DOMAIN_GET_SCHEDULER_PARAMETERS = 57isize,
    REMOTE_PROC_DOMAIN_SET_SCHEDULER_PARAMETERS = 58isize,
    REMOTE_PROC_CONNECT_GET_HOSTNAME = 59isize,
    REMOTE_PROC_CONNECT_SUPPORTS_FEATURE = 60isize,
    REMOTE_PROC_DOMAIN_MIGRATE_PREPARE = 61isize,
    REMOTE_PROC_DOMAIN_MIGRATE_PERFORM = 62isize,
    REMOTE_PROC_DOMAIN_MIGRATE_FINISH = 63isize,
    REMOTE_PROC_DOMAIN_BLOCK_STATS = 64isize,
    REMOTE_PROC_DOMAIN_INTERFACE_STATS = 65isize,
    REMOTE_PROC_AUTH_LIST = 66isize,
    REMOTE_PROC_AUTH_SASL_INIT = 67isize,
    REMOTE_PROC_AUTH_SASL_START = 68isize,
    REMOTE_PROC_AUTH_SASL_STEP = 69isize,
    REMOTE_PROC_AUTH_POLKIT = 70isize,
    REMOTE_PROC_CONNECT_NUM_OF_STORAGE_POOLS = 71isize,
    REMOTE_PROC_CONNECT_LIST_STORAGE_POOLS = 72isize,
    REMOTE_PROC_CONNECT_NUM_OF_DEFINED_STORAGE_POOLS = 73isize,
    REMOTE_PROC_CONNECT_LIST_DEFINED_STORAGE_POOLS = 74isize,
    REMOTE_PROC_CONNECT_FIND_STORAGE_POOL_SOURCES = 75isize,
    REMOTE_PROC_STORAGE_POOL_CREATE_XML = 76isize,
    REMOTE_PROC_STORAGE_POOL_DEFINE_XML = 77isize,
    REMOTE_PROC_STORAGE_POOL_CREATE = 78isize,
    REMOTE_PROC_STORAGE_POOL_BUILD = 79isize,
    REMOTE_PROC_STORAGE_POOL_DESTROY = 80isize,
    REMOTE_PROC_STORAGE_POOL_DELETE = 81isize,
    REMOTE_PROC_STORAGE_POOL_UNDEFINE = 82isize,
    REMOTE_PROC_STORAGE_POOL_REFRESH = 83isize,
    REMOTE_PROC_STORAGE_POOL_LOOKUP_BY_NAME = 84isize,
    REMOTE_PROC_STORAGE_POOL_LOOKUP_BY_UUID = 85isize,
    REMOTE_PROC_STORAGE_POOL_LOOKUP_BY_VOLUME = 86isize,
    REMOTE_PROC_STORAGE_POOL_GET_INFO = 87isize,
    REMOTE_PROC_STORAGE_POOL_GET_XML_DESC = 88isize,
    REMOTE_PROC_STORAGE_POOL_GET_AUTOSTART = 89isize,
    REMOTE_PROC_STORAGE_POOL_SET_AUTOSTART = 90isize,
    REMOTE_PROC_STORAGE_POOL_NUM_OF_VOLUMES = 91isize,
    REMOTE_PROC_STORAGE_POOL_LIST_VOLUMES = 92isize,
    REMOTE_PROC_STORAGE_VOL_CREATE_XML = 93isize,
    REMOTE_PROC_STORAGE_VOL_DELETE = 94isize,
    REMOTE_PROC_STORAGE_VOL_LOOKUP_BY_NAME = 95isize,
    REMOTE_PROC_STORAGE_VOL_LOOKUP_BY_KEY = 96isize,
    REMOTE_PROC_STORAGE_VOL_LOOKUP_BY_PATH = 97isize,
    REMOTE_PROC_STORAGE_VOL_GET_INFO = 98isize,
    REMOTE_PROC_STORAGE_VOL_GET_XML_DESC = 99isize,
    REMOTE_PROC_STORAGE_VOL_GET_PATH = 100isize,
    REMOTE_PROC_NODE_GET_CELLS_FREE_MEMORY = 101isize,
    REMOTE_PROC_NODE_GET_FREE_MEMORY = 102isize,
    REMOTE_PROC_DOMAIN_BLOCK_PEEK = 103isize,
    REMOTE_PROC_DOMAIN_MEMORY_PEEK = 104isize,
    REMOTE_PROC_CONNECT_DOMAIN_EVENT_REGISTER = 105isize,
    REMOTE_PROC_CONNECT_DOMAIN_EVENT_DEREGISTER = 106isize,
    REMOTE_PROC_DOMAIN_EVENT_LIFECYCLE = 107isize,
    REMOTE_PROC_DOMAIN_MIGRATE_PREPARE2 = 108isize,
    REMOTE_PROC_DOMAIN_MIGRATE_FINISH2 = 109isize,
    REMOTE_PROC_CONNECT_GET_URI = 110isize,
    REMOTE_PROC_NODE_NUM_OF_DEVICES = 111isize,
    REMOTE_PROC_NODE_LIST_DEVICES = 112isize,
    REMOTE_PROC_NODE_DEVICE_LOOKUP_BY_NAME = 113isize,
    REMOTE_PROC_NODE_DEVICE_GET_XML_DESC = 114isize,
    REMOTE_PROC_NODE_DEVICE_GET_PARENT = 115isize,
    REMOTE_PROC_NODE_DEVICE_NUM_OF_CAPS = 116isize,
    REMOTE_PROC_NODE_DEVICE_LIST_CAPS = 117isize,
    REMOTE_PROC_NODE_DEVICE_DETTACH = 118isize,
    REMOTE_PROC_NODE_DEVICE_RE_ATTACH = 119isize,
    REMOTE_PROC_NODE_DEVICE_RESET = 120isize,
    REMOTE_PROC_DOMAIN_GET_SECURITY_LABEL = 121isize,
    REMOTE_PROC_NODE_GET_SECURITY_MODEL = 122isize,
    REMOTE_PROC_NODE_DEVICE_CREATE_XML = 123isize,
    REMOTE_PROC_NODE_DEVICE_DESTROY = 124isize,
    REMOTE_PROC_STORAGE_VOL_CREATE_XML_FROM = 125isize,
    REMOTE_PROC_CONNECT_NUM_OF_INTERFACES = 126isize,
    REMOTE_PROC_CONNECT_LIST_INTERFACES = 127isize,
    REMOTE_PROC_INTERFACE_LOOKUP_BY_NAME = 128isize,
    REMOTE_PROC_INTERFACE_LOOKUP_BY_MAC_STRING = 129isize,
    REMOTE_PROC_INTERFACE_GET_XML_DESC = 130isize,
    REMOTE_PROC_INTERFACE_DEFINE_XML = 131isize,
    REMOTE_PROC_INTERFACE_UNDEFINE = 132isize,
    REMOTE_PROC_INTERFACE_CREATE = 133isize,
    REMOTE_PROC_INTERFACE_DESTROY = 134isize,
    REMOTE_PROC_CONNECT_DOMAIN_XML_FROM_NATIVE = 135isize,
    REMOTE_PROC_CONNECT_DOMAIN_XML_TO_NATIVE = 136isize,
    REMOTE_PROC_CONNECT_NUM_OF_DEFINED_INTERFACES = 137isize,
    REMOTE_PROC_CONNECT_LIST_DEFINED_INTERFACES = 138isize,
    REMOTE_PROC_CONNECT_NUM_OF_SECRETS = 139isize,
    REMOTE_PROC_CONNECT_LIST_SECRETS = 140isize,
    REMOTE_PROC_SECRET_LOOKUP_BY_UUID = 141isize,
    REMOTE_PROC_SECRET_DEFINE_XML = 142isize,
    REMOTE_PROC_SECRET_GET_XML_DESC = 143isize,
    REMOTE_PROC_SECRET_SET_VALUE = 144isize,
    REMOTE_PROC_SECRET_GET_VALUE = 145isize,
    REMOTE_PROC_SECRET_UNDEFINE = 146isize,
    REMOTE_PROC_SECRET_LOOKUP_BY_USAGE = 147isize,
    REMOTE_PROC_DOMAIN_MIGRATE_PREPARE_TUNNEL = 148isize,
    REMOTE_PROC_CONNECT_IS_SECURE = 149isize,
    REMOTE_PROC_DOMAIN_IS_ACTIVE = 150isize,
    REMOTE_PROC_DOMAIN_IS_PERSISTENT = 151isize,
    REMOTE_PROC_NETWORK_IS_ACTIVE = 152isize,
    REMOTE_PROC_NETWORK_IS_PERSISTENT = 153isize,
    REMOTE_PROC_STORAGE_POOL_IS_ACTIVE = 154isize,
    REMOTE_PROC_STORAGE_POOL_IS_PERSISTENT = 155isize,
    REMOTE_PROC_INTERFACE_IS_ACTIVE = 156isize,
    REMOTE_PROC_CONNECT_GET_LIB_VERSION = 157isize,
    REMOTE_PROC_CONNECT_COMPARE_CPU = 158isize,
    REMOTE_PROC_DOMAIN_MEMORY_STATS = 159isize,
    REMOTE_PROC_DOMAIN_ATTACH_DEVICE_FLAGS = 160isize,
    REMOTE_PROC_DOMAIN_DETACH_DEVICE_FLAGS = 161isize,
    REMOTE_PROC_CONNECT_BASELINE_CPU = 162isize,
    REMOTE_PROC_DOMAIN_GET_JOB_INFO = 163isize,
    REMOTE_PROC_DOMAIN_ABORT_JOB = 164isize,
    REMOTE_PROC_STORAGE_VOL_WIPE = 165isize,
    REMOTE_PROC_DOMAIN_MIGRATE_SET_MAX_DOWNTIME = 166isize,
    REMOTE_PROC_CONNECT_DOMAIN_EVENT_REGISTER_ANY = 167isize,
    REMOTE_PROC_CONNECT_DOMAIN_EVENT_DEREGISTER_ANY = 168isize,
    REMOTE_PROC_DOMAIN_EVENT_REBOOT = 169isize,
    REMOTE_PROC_DOMAIN_EVENT_RTC_CHANGE = 170isize,
    REMOTE_PROC_DOMAIN_EVENT_WATCHDOG = 171isize,
    REMOTE_PROC_DOMAIN_EVENT_IO_ERROR = 172isize,
    REMOTE_PROC_DOMAIN_EVENT_GRAPHICS = 173isize,
    REMOTE_PROC_DOMAIN_UPDATE_DEVICE_FLAGS = 174isize,
    REMOTE_PROC_NWFILTER_LOOKUP_BY_NAME = 175isize,
    REMOTE_PROC_NWFILTER_LOOKUP_BY_UUID = 176isize,
    REMOTE_PROC_NWFILTER_GET_XML_DESC = 177isize,
    REMOTE_PROC_CONNECT_NUM_OF_NWFILTERS = 178isize,
    REMOTE_PROC_CONNECT_LIST_NWFILTERS = 179isize,
    REMOTE_PROC_NWFILTER_DEFINE_XML = 180isize,
    REMOTE_PROC_NWFILTER_UNDEFINE = 181isize,
    REMOTE_PROC_DOMAIN_MANAGED_SAVE = 182isize,
    REMOTE_PROC_DOMAIN_HAS_MANAGED_SAVE_IMAGE = 183isize,
    REMOTE_PROC_DOMAIN_MANAGED_SAVE_REMOVE = 184isize,
    REMOTE_PROC_DOMAIN_SNAPSHOT_CREATE_XML = 185isize,
    REMOTE_PROC_DOMAIN_SNAPSHOT_GET_XML_DESC = 186isize,
    REMOTE_PROC_DOMAIN_SNAPSHOT_NUM = 187isize,
    REMOTE_PROC_DOMAIN_SNAPSHOT_LIST_NAMES = 188isize,
    REMOTE_PROC_DOMAIN_SNAPSHOT_LOOKUP_BY_NAME = 189isize,
    REMOTE_PROC_DOMAIN_HAS_CURRENT_SNAPSHOT = 190isize,
    REMOTE_PROC_DOMAIN_SNAPSHOT_CURRENT = 191isize,
    REMOTE_PROC_DOMAIN_REVERT_TO_SNAPSHOT = 192isize,
    REMOTE_PROC_DOMAIN_SNAPSHOT_DELETE = 193isize,
    REMOTE_PROC_DOMAIN_GET_BLOCK_INFO = 194isize,
    REMOTE_PROC_DOMAIN_EVENT_IO_ERROR_REASON = 195isize,
    REMOTE_PROC_DOMAIN_CREATE_WITH_FLAGS = 196isize,
    REMOTE_PROC_DOMAIN_SET_MEMORY_PARAMETERS = 197isize,
    REMOTE_PROC_DOMAIN_GET_MEMORY_PARAMETERS = 198isize,
    REMOTE_PROC_DOMAIN_SET_VCPUS_FLAGS = 199isize,
    REMOTE_PROC_DOMAIN_GET_VCPUS_FLAGS = 200isize,
    REMOTE_PROC_DOMAIN_OPEN_CONSOLE = 201isize,
    REMOTE_PROC_DOMAIN_IS_UPDATED = 202isize,
    REMOTE_PROC_CONNECT_GET_SYSINFO = 203isize,
    REMOTE_PROC_DOMAIN_SET_MEMORY_FLAGS = 204isize,
    REMOTE_PROC_DOMAIN_SET_BLKIO_PARAMETERS = 205isize,
    REMOTE_PROC_DOMAIN_GET_BLKIO_PARAMETERS = 206isize,
    REMOTE_PROC_DOMAIN_MIGRATE_SET_MAX_SPEED = 207isize,
    REMOTE_PROC_STORAGE_VOL_UPLOAD = 208isize,
    REMOTE_PROC_STORAGE_VOL_DOWNLOAD = 209isize,
    REMOTE_PROC_DOMAIN_INJECT_NMI = 210isize,
    REMOTE_PROC_DOMAIN_SCREENSHOT = 211isize,
    REMOTE_PROC_DOMAIN_GET_STATE = 212isize,
    REMOTE_PROC_DOMAIN_MIGRATE_BEGIN3 = 213isize,
    REMOTE_PROC_DOMAIN_MIGRATE_PREPARE3 = 214isize,
    REMOTE_PROC_DOMAIN_MIGRATE_PREPARE_TUNNEL3 = 215isize,
    REMOTE_PROC_DOMAIN_MIGRATE_PERFORM3 = 216isize,
    REMOTE_PROC_DOMAIN_MIGRATE_FINISH3 = 217isize,
    REMOTE_PROC_DOMAIN_MIGRATE_CONFIRM3 = 218isize,
    REMOTE_PROC_DOMAIN_SET_SCHEDULER_PARAMETERS_FLAGS = 219isize,
    REMOTE_PROC_INTERFACE_CHANGE_BEGIN = 220isize,
    REMOTE_PROC_INTERFACE_CHANGE_COMMIT = 221isize,
    REMOTE_PROC_INTERFACE_CHANGE_ROLLBACK = 222isize,
    REMOTE_PROC_DOMAIN_GET_SCHEDULER_PARAMETERS_FLAGS = 223isize,
    REMOTE_PROC_DOMAIN_EVENT_CONTROL_ERROR = 224isize,
    REMOTE_PROC_DOMAIN_PIN_VCPU_FLAGS = 225isize,
    REMOTE_PROC_DOMAIN_SEND_KEY = 226isize,
    REMOTE_PROC_NODE_GET_CPU_STATS = 227isize,
    REMOTE_PROC_NODE_GET_MEMORY_STATS = 228isize,
    REMOTE_PROC_DOMAIN_GET_CONTROL_INFO = 229isize,
    REMOTE_PROC_DOMAIN_GET_VCPU_PIN_INFO = 230isize,
    REMOTE_PROC_DOMAIN_UNDEFINE_FLAGS = 231isize,
    REMOTE_PROC_DOMAIN_SAVE_FLAGS = 232isize,
    REMOTE_PROC_DOMAIN_RESTORE_FLAGS = 233isize,
    REMOTE_PROC_DOMAIN_DESTROY_FLAGS = 234isize,
    REMOTE_PROC_DOMAIN_SAVE_IMAGE_GET_XML_DESC = 235isize,
    REMOTE_PROC_DOMAIN_SAVE_IMAGE_DEFINE_XML = 236isize,
    REMOTE_PROC_DOMAIN_BLOCK_JOB_ABORT = 237isize,
    REMOTE_PROC_DOMAIN_GET_BLOCK_JOB_INFO = 238isize,
    REMOTE_PROC_DOMAIN_BLOCK_JOB_SET_SPEED = 239isize,
    REMOTE_PROC_DOMAIN_BLOCK_PULL = 240isize,
    REMOTE_PROC_DOMAIN_EVENT_BLOCK_JOB = 241isize,
    REMOTE_PROC_DOMAIN_MIGRATE_GET_MAX_SPEED = 242isize,
    REMOTE_PROC_DOMAIN_BLOCK_STATS_FLAGS = 243isize,
    REMOTE_PROC_DOMAIN_SNAPSHOT_GET_PARENT = 244isize,
    REMOTE_PROC_DOMAIN_RESET = 245isize,
    REMOTE_PROC_DOMAIN_SNAPSHOT_NUM_CHILDREN = 246isize,
    REMOTE_PROC_DOMAIN_SNAPSHOT_LIST_CHILDREN_NAMES = 247isize,
    REMOTE_PROC_DOMAIN_EVENT_DISK_CHANGE = 248isize,
    REMOTE_PROC_DOMAIN_OPEN_GRAPHICS = 249isize,
    REMOTE_PROC_NODE_SUSPEND_FOR_DURATION = 250isize,
    REMOTE_PROC_DOMAIN_BLOCK_RESIZE = 251isize,
    REMOTE_PROC_DOMAIN_SET_BLOCK_IO_TUNE = 252isize,
    REMOTE_PROC_DOMAIN_GET_BLOCK_IO_TUNE = 253isize,
    REMOTE_PROC_DOMAIN_SET_NUMA_PARAMETERS = 254isize,
    REMOTE_PROC_DOMAIN_GET_NUMA_PARAMETERS = 255isize,
    REMOTE_PROC_DOMAIN_SET_INTERFACE_PARAMETERS = 256isize,
    REMOTE_PROC_DOMAIN_GET_INTERFACE_PARAMETERS = 257isize,
    REMOTE_PROC_DOMAIN_SHUTDOWN_FLAGS = 258isize,
    REMOTE_PROC_STORAGE_VOL_WIPE_PATTERN = 259isize,
    REMOTE_PROC_STORAGE_VOL_RESIZE = 260isize,
    REMOTE_PROC_DOMAIN_PM_SUSPEND_FOR_DURATION = 261isize,
    REMOTE_PROC_DOMAIN_GET_CPU_STATS = 262isize,
    REMOTE_PROC_DOMAIN_GET_DISK_ERRORS = 263isize,
    REMOTE_PROC_DOMAIN_SET_METADATA = 264isize,
    REMOTE_PROC_DOMAIN_GET_METADATA = 265isize,
    REMOTE_PROC_DOMAIN_BLOCK_REBASE = 266isize,
    REMOTE_PROC_DOMAIN_PM_WAKEUP = 267isize,
    REMOTE_PROC_DOMAIN_EVENT_TRAY_CHANGE = 268isize,
    REMOTE_PROC_DOMAIN_EVENT_PMWAKEUP = 269isize,
    REMOTE_PROC_DOMAIN_EVENT_PMSUSPEND = 270isize,
    REMOTE_PROC_DOMAIN_SNAPSHOT_IS_CURRENT = 271isize,
    REMOTE_PROC_DOMAIN_SNAPSHOT_HAS_METADATA = 272isize,
    REMOTE_PROC_CONNECT_LIST_ALL_DOMAINS = 273isize,
    REMOTE_PROC_DOMAIN_LIST_ALL_SNAPSHOTS = 274isize,
    REMOTE_PROC_DOMAIN_SNAPSHOT_LIST_ALL_CHILDREN = 275isize,
    REMOTE_PROC_DOMAIN_EVENT_BALLOON_CHANGE = 276isize,
    REMOTE_PROC_DOMAIN_GET_HOSTNAME = 277isize,
    REMOTE_PROC_DOMAIN_GET_SECURITY_LABEL_LIST = 278isize,
    REMOTE_PROC_DOMAIN_PIN_EMULATOR = 279isize,
    REMOTE_PROC_DOMAIN_GET_EMULATOR_PIN_INFO = 280isize,
    REMOTE_PROC_CONNECT_LIST_ALL_STORAGE_POOLS = 281isize,
    REMOTE_PROC_STORAGE_POOL_LIST_ALL_VOLUMES = 282isize,
    REMOTE_PROC_CONNECT_LIST_ALL_NETWORKS = 283isize,
    REMOTE_PROC_CONNECT_LIST_ALL_INTERFACES = 284isize,
    REMOTE_PROC_CONNECT_LIST_ALL_NODE_DEVICES = 285isize,
    REMOTE_PROC_CONNECT_LIST_ALL_NWFILTERS = 286isize,
    REMOTE_PROC_CONNECT_LIST_ALL_SECRETS = 287isize,
    REMOTE_PROC_NODE_SET_MEMORY_PARAMETERS = 288isize,
    REMOTE_PROC_NODE_GET_MEMORY_PARAMETERS = 289isize,
    REMOTE_PROC_DOMAIN_BLOCK_COMMIT = 290isize,
    REMOTE_PROC_NETWORK_UPDATE = 291isize,
    REMOTE_PROC_DOMAIN_EVENT_PMSUSPEND_DISK = 292isize,
    REMOTE_PROC_NODE_GET_CPU_MAP = 293isize,
    REMOTE_PROC_DOMAIN_FSTRIM = 294isize,
    REMOTE_PROC_DOMAIN_SEND_PROCESS_SIGNAL = 295isize,
    REMOTE_PROC_DOMAIN_OPEN_CHANNEL = 296isize,
    REMOTE_PROC_NODE_DEVICE_LOOKUP_SCSI_HOST_BY_WWN = 297isize,
    REMOTE_PROC_DOMAIN_GET_JOB_STATS = 298isize,
    REMOTE_PROC_DOMAIN_MIGRATE_GET_COMPRESSION_CACHE = 299isize,
    REMOTE_PROC_DOMAIN_MIGRATE_SET_COMPRESSION_CACHE = 300isize,
    REMOTE_PROC_NODE_DEVICE_DETACH_FLAGS = 301isize,
    REMOTE_PROC_DOMAIN_MIGRATE_BEGIN3_PARAMS = 302isize,
    REMOTE_PROC_DOMAIN_MIGRATE_PREPARE3_PARAMS = 303isize,
    REMOTE_PROC_DOMAIN_MIGRATE_PREPARE_TUNNEL3_PARAMS = 304isize,
    REMOTE_PROC_DOMAIN_MIGRATE_PERFORM3_PARAMS = 305isize,
    REMOTE_PROC_DOMAIN_MIGRATE_FINISH3_PARAMS = 306isize,
    REMOTE_PROC_DOMAIN_MIGRATE_CONFIRM3_PARAMS = 307isize,
    REMOTE_PROC_DOMAIN_SET_MEMORY_STATS_PERIOD = 308isize,
    REMOTE_PROC_DOMAIN_CREATE_XML_WITH_FILES = 309isize,
    REMOTE_PROC_DOMAIN_CREATE_WITH_FILES = 310isize,
    REMOTE_PROC_DOMAIN_EVENT_DEVICE_REMOVED = 311isize,
    REMOTE_PROC_CONNECT_GET_CPU_MODEL_NAMES = 312isize,
    REMOTE_PROC_CONNECT_NETWORK_EVENT_REGISTER_ANY = 313isize,
    REMOTE_PROC_CONNECT_NETWORK_EVENT_DEREGISTER_ANY = 314isize,
    REMOTE_PROC_NETWORK_EVENT_LIFECYCLE = 315isize,
    REMOTE_PROC_CONNECT_DOMAIN_EVENT_CALLBACK_REGISTER_ANY = 316isize,
    REMOTE_PROC_CONNECT_DOMAIN_EVENT_CALLBACK_DEREGISTER_ANY = 317isize,
    REMOTE_PROC_DOMAIN_EVENT_CALLBACK_LIFECYCLE = 318isize,
    REMOTE_PROC_DOMAIN_EVENT_CALLBACK_REBOOT = 319isize,
    REMOTE_PROC_DOMAIN_EVENT_CALLBACK_RTC_CHANGE = 320isize,
    REMOTE_PROC_DOMAIN_EVENT_CALLBACK_WATCHDOG = 321isize,
    REMOTE_PROC_DOMAIN_EVENT_CALLBACK_IO_ERROR = 322isize,
    REMOTE_PROC_DOMAIN_EVENT_CALLBACK_GRAPHICS = 323isize,
    REMOTE_PROC_DOMAIN_EVENT_CALLBACK_IO_ERROR_REASON = 324isize,
    REMOTE_PROC_DOMAIN_EVENT_CALLBACK_CONTROL_ERROR = 325isize,
    REMOTE_PROC_DOMAIN_EVENT_CALLBACK_BLOCK_JOB = 326isize,
    REMOTE_PROC_DOMAIN_EVENT_CALLBACK_DISK_CHANGE = 327isize,
    REMOTE_PROC_DOMAIN_EVENT_CALLBACK_TRAY_CHANGE = 328isize,
    REMOTE_PROC_DOMAIN_EVENT_CALLBACK_PMWAKEUP = 329isize,
    REMOTE_PROC_DOMAIN_EVENT_CALLBACK_PMSUSPEND = 330isize,
    REMOTE_PROC_DOMAIN_EVENT_CALLBACK_BALLOON_CHANGE = 331isize,
    REMOTE_PROC_DOMAIN_EVENT_CALLBACK_PMSUSPEND_DISK = 332isize,
    REMOTE_PROC_DOMAIN_EVENT_CALLBACK_DEVICE_REMOVED = 333isize,
    REMOTE_PROC_DOMAIN_CORE_DUMP_WITH_FORMAT = 334isize,
    REMOTE_PROC_DOMAIN_FSFREEZE = 335isize,
    REMOTE_PROC_DOMAIN_FSTHAW = 336isize,
    REMOTE_PROC_DOMAIN_GET_TIME = 337isize,
    REMOTE_PROC_DOMAIN_SET_TIME = 338isize,
    REMOTE_PROC_DOMAIN_EVENT_BLOCK_JOB_2 = 339isize,
    REMOTE_PROC_NODE_GET_FREE_PAGES = 340isize,
    REMOTE_PROC_NETWORK_GET_DHCP_LEASES = 341isize,
    REMOTE_PROC_CONNECT_GET_DOMAIN_CAPABILITIES = 342isize,
    REMOTE_PROC_DOMAIN_OPEN_GRAPHICS_FD = 343isize,
    REMOTE_PROC_CONNECT_GET_ALL_DOMAIN_STATS = 344isize,
    REMOTE_PROC_DOMAIN_BLOCK_COPY = 345isize,
    REMOTE_PROC_DOMAIN_EVENT_CALLBACK_TUNABLE = 346isize,
    REMOTE_PROC_NODE_ALLOC_PAGES = 347isize,
    REMOTE_PROC_DOMAIN_EVENT_CALLBACK_AGENT_LIFECYCLE = 348isize,
    REMOTE_PROC_DOMAIN_GET_FSINFO = 349isize,
    REMOTE_PROC_DOMAIN_DEFINE_XML_FLAGS = 350isize,
    REMOTE_PROC_DOMAIN_GET_IOTHREAD_INFO = 351isize,
    REMOTE_PROC_DOMAIN_PIN_IOTHREAD = 352isize,
    REMOTE_PROC_DOMAIN_INTERFACE_ADDRESSES = 353isize,
    REMOTE_PROC_DOMAIN_EVENT_CALLBACK_DEVICE_ADDED = 354isize,
    REMOTE_PROC_DOMAIN_ADD_IOTHREAD = 355isize,
    REMOTE_PROC_DOMAIN_DEL_IOTHREAD = 356isize,
    REMOTE_PROC_DOMAIN_SET_USER_PASSWORD = 357isize,
    REMOTE_PROC_DOMAIN_RENAME = 358isize,
    REMOTE_PROC_DOMAIN_EVENT_CALLBACK_MIGRATION_ITERATION = 359isize,
    REMOTE_PROC_CONNECT_REGISTER_CLOSE_CALLBACK = 360isize,
    REMOTE_PROC_CONNECT_UNREGISTER_CLOSE_CALLBACK = 361isize,
    REMOTE_PROC_CONNECT_EVENT_CONNECTION_CLOSED = 362isize,
    REMOTE_PROC_DOMAIN_EVENT_CALLBACK_JOB_COMPLETED = 363isize,
    REMOTE_PROC_DOMAIN_MIGRATE_START_POST_COPY = 364isize,
    REMOTE_PROC_DOMAIN_GET_PERF_EVENTS = 365isize,
    REMOTE_PROC_DOMAIN_SET_PERF_EVENTS = 366isize,
    REMOTE_PROC_DOMAIN_EVENT_CALLBACK_DEVICE_REMOVAL_FAILED = 367isize,
    REMOTE_PROC_CONNECT_STORAGE_POOL_EVENT_REGISTER_ANY = 368isize,
    REMOTE_PROC_CONNECT_STORAGE_POOL_EVENT_DEREGISTER_ANY = 369isize,
    REMOTE_PROC_STORAGE_POOL_EVENT_LIFECYCLE = 370isize,
    REMOTE_PROC_DOMAIN_GET_GUEST_VCPUS = 371isize,
    REMOTE_PROC_DOMAIN_SET_GUEST_VCPUS = 372isize,
    REMOTE_PROC_STORAGE_POOL_EVENT_REFRESH = 373isize,
    REMOTE_PROC_CONNECT_NODE_DEVICE_EVENT_REGISTER_ANY = 374isize,
    REMOTE_PROC_CONNECT_NODE_DEVICE_EVENT_DEREGISTER_ANY = 375isize,
    REMOTE_PROC_NODE_DEVICE_EVENT_LIFECYCLE = 376isize,
    REMOTE_PROC_NODE_DEVICE_EVENT_UPDATE = 377isize,
    REMOTE_PROC_STORAGE_VOL_GET_INFO_FLAGS = 378isize,
    REMOTE_PROC_DOMAIN_EVENT_CALLBACK_METADATA_CHANGE = 379isize,
    REMOTE_PROC_CONNECT_SECRET_EVENT_REGISTER_ANY = 380isize,
    REMOTE_PROC_CONNECT_SECRET_EVENT_DEREGISTER_ANY = 381isize,
    REMOTE_PROC_SECRET_EVENT_LIFECYCLE = 382isize,
    REMOTE_PROC_SECRET_EVENT_VALUE_CHANGED = 383isize,
    REMOTE_PROC_DOMAIN_SET_VCPU = 384isize,
    REMOTE_PROC_DOMAIN_EVENT_BLOCK_THRESHOLD = 385isize,
    REMOTE_PROC_DOMAIN_SET_BLOCK_THRESHOLD = 386isize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_secret_define_xml_args {
    pub xml: remote_nonnull_string,
    pub flags: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_secret_define_xml_ret {
    pub secret: remote_nonnull_secret,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_secret_event_lifecycle_msg {
    pub callbackID: i32,
    pub secret: remote_nonnull_secret,
    pub event: i32,
    pub detail: i32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_secret_event_value_changed_msg {
    pub callbackID: i32,
    pub secret: remote_nonnull_secret,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_secret_get_value_args {
    pub secret: remote_nonnull_secret,
    pub flags: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_secret_get_value_ret {
    pub value: Vec<u8>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_secret_get_xml_desc_args {
    pub secret: remote_nonnull_secret,
    pub flags: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_secret_get_xml_desc_ret {
    pub xml: remote_nonnull_string,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_secret_lookup_by_usage_args {
    pub usageType: i32,
    pub usageID: remote_nonnull_string,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_secret_lookup_by_usage_ret {
    pub secret: remote_nonnull_secret,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct remote_secret_lookup_by_uuid_args {
    pub uuid: remote_uuid,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_secret_lookup_by_uuid_ret {
    pub secret: remote_nonnull_secret,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_secret_set_value_args {
    pub secret: remote_nonnull_secret,
    pub value: Vec<u8>,
    pub flags: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_secret_undefine_args {
    pub secret: remote_nonnull_secret,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_storage_pool_build_args {
    pub pool: remote_nonnull_storage_pool,
    pub flags: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_storage_pool_create_args {
    pub pool: remote_nonnull_storage_pool,
    pub flags: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_storage_pool_create_xml_args {
    pub xml: remote_nonnull_string,
    pub flags: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_storage_pool_create_xml_ret {
    pub pool: remote_nonnull_storage_pool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_storage_pool_define_xml_args {
    pub xml: remote_nonnull_string,
    pub flags: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_storage_pool_define_xml_ret {
    pub pool: remote_nonnull_storage_pool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_storage_pool_delete_args {
    pub pool: remote_nonnull_storage_pool,
    pub flags: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_storage_pool_destroy_args {
    pub pool: remote_nonnull_storage_pool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_storage_pool_event_lifecycle_msg {
    pub callbackID: i32,
    pub pool: remote_nonnull_storage_pool,
    pub event: i32,
    pub detail: i32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_storage_pool_event_refresh_msg {
    pub callbackID: i32,
    pub pool: remote_nonnull_storage_pool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_storage_pool_get_autostart_args {
    pub pool: remote_nonnull_storage_pool,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct remote_storage_pool_get_autostart_ret {
    pub autostart: i32,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct remote_storage_pool_get_info_args {
    pub pool: remote_nonnull_storage_pool,
}

#[derive(Copy, Clone, Debug, Default, Eq, PartialEq)]
pub struct remote_storage_pool_get_info_ret {
    pub state: u8,
    pub capacity: u64,
    pub allocation: u64,
    pub available: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_storage_pool_get_xml_desc_args {
    pub pool: remote_nonnull_storage_pool,
    pub flags: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_storage_pool_get_xml_desc_ret {
    pub xml: remote_nonnull_string,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_storage_pool_is_active_args {
    pub pool: remote_nonnull_storage_pool,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct remote_storage_pool_is_active_ret {
    pub active: i32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_storage_pool_is_persistent_args {
    pub pool: remote_nonnull_storage_pool,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct remote_storage_pool_is_persistent_ret {
    pub persistent: i32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_storage_pool_list_all_volumes_args {
    pub pool: remote_nonnull_storage_pool,
    pub need_results: i32,
    pub flags: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_storage_pool_list_all_volumes_ret {
    pub vols: Vec<remote_nonnull_storage_vol>,
    pub ret: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_storage_pool_list_volumes_args {
    pub pool: remote_nonnull_storage_pool,
    pub maxnames: i32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_storage_pool_list_volumes_ret {
    pub names: Vec<remote_nonnull_string>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_storage_pool_lookup_by_name_args {
    pub name: remote_nonnull_string,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_storage_pool_lookup_by_name_ret {
    pub pool: remote_nonnull_storage_pool,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct remote_storage_pool_lookup_by_uuid_args {
    pub uuid: remote_uuid,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_storage_pool_lookup_by_uuid_ret {
    pub pool: remote_nonnull_storage_pool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_storage_pool_lookup_by_volume_args {
    pub vol: remote_nonnull_storage_vol,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_storage_pool_lookup_by_volume_ret {
    pub pool: remote_nonnull_storage_pool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_storage_pool_num_of_volumes_args {
    pub pool: remote_nonnull_storage_pool,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct remote_storage_pool_num_of_volumes_ret {
    pub num: i32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_storage_pool_refresh_args {
    pub pool: remote_nonnull_storage_pool,
    pub flags: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_storage_pool_set_autostart_args {
    pub pool: remote_nonnull_storage_pool,
    pub autostart: i32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_storage_pool_undefine_args {
    pub pool: remote_nonnull_storage_pool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_storage_vol_create_xml_args {
    pub pool: remote_nonnull_storage_pool,
    pub xml: remote_nonnull_string,
    pub flags: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_storage_vol_create_xml_from_args {
    pub pool: remote_nonnull_storage_pool,
    pub xml: remote_nonnull_string,
    pub clonevol: remote_nonnull_storage_vol,
    pub flags: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_storage_vol_create_xml_from_ret {
    pub vol: remote_nonnull_storage_vol,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_storage_vol_create_xml_ret {
    pub vol: remote_nonnull_storage_vol,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_storage_vol_delete_args {
    pub vol: remote_nonnull_storage_vol,
    pub flags: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_storage_vol_download_args {
    pub vol: remote_nonnull_storage_vol,
    pub offset: u64,
    pub length: u64,
    pub flags: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_storage_vol_get_info_args {
    pub vol: remote_nonnull_storage_vol,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_storage_vol_get_info_flags_args {
    pub vol: remote_nonnull_storage_vol,
    pub flags: u32,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct remote_storage_vol_get_info_flags_ret {
    pub type_: i8,
    pub capacity: u64,
    pub allocation: u64,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct remote_storage_vol_get_info_ret {
    pub type_: i8,
    pub capacity: u64,
    pub allocation: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_storage_vol_get_path_args {
    pub vol: remote_nonnull_storage_vol,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_storage_vol_get_path_ret {
    pub name: remote_nonnull_string,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_storage_vol_get_xml_desc_args {
    pub vol: remote_nonnull_storage_vol,
    pub flags: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_storage_vol_get_xml_desc_ret {
    pub xml: remote_nonnull_string,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_storage_vol_lookup_by_key_args {
    pub key: remote_nonnull_string,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_storage_vol_lookup_by_key_ret {
    pub vol: remote_nonnull_storage_vol,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_storage_vol_lookup_by_name_args {
    pub pool: remote_nonnull_storage_pool,
    pub name: remote_nonnull_string,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_storage_vol_lookup_by_name_ret {
    pub vol: remote_nonnull_storage_vol,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_storage_vol_lookup_by_path_args {
    pub path: remote_nonnull_string,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_storage_vol_lookup_by_path_ret {
    pub vol: remote_nonnull_storage_vol,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_storage_vol_resize_args {
    pub vol: remote_nonnull_storage_vol,
    pub capacity: u64,
    pub flags: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_storage_vol_upload_args {
    pub vol: remote_nonnull_storage_vol,
    pub offset: u64,
    pub length: u64,
    pub flags: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_storage_vol_wipe_args {
    pub vol: remote_nonnull_storage_vol,
    pub flags: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct remote_storage_vol_wipe_pattern_args {
    pub vol: remote_nonnull_storage_vol,
    pub algorithm: u32,
    pub flags: u32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct remote_typed_param {
    pub field: remote_nonnull_string,
    pub value: remote_typed_param_value,
}

#[derive(Clone, Debug, PartialEq)]
pub enum remote_typed_param_value {
    Const1(i32),
    Const2(u32),
    Const3(i64),
    Const4(u64),
    Const5(f64),
    Const6(i32),
    Const7(remote_nonnull_string),
}

#[derive(Copy, Clone, Debug, Default, Eq, PartialEq)]
pub struct remote_uuid(pub [u8; 16i64 as usize]);

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct remote_vcpu_info {
    pub number: u32,
    pub state: i32,
    pub cpu_time: u64,
    pub cpu: i32,
}

pub type remote_domain = Option<Box<remote_nonnull_domain>>;

pub type remote_network = Option<Box<remote_nonnull_network>>;

pub type remote_node_device = Option<Box<remote_nonnull_node_device>>;

pub type remote_nwfilter = Option<Box<remote_nonnull_nwfilter>>;

pub type remote_secret = Option<Box<remote_nonnull_secret>>;

pub type remote_storage_pool = Option<Box<remote_nonnull_storage_pool>>;

pub type remote_storage_vol = Option<Box<remote_nonnull_storage_vol>>;

pub type remote_string = Option<remote_nonnull_string>;

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_auth_list_ret {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(xdr_codec::pack_flex(&self.types, Some(REMOTE_AUTH_TYPE_LIST_MAX as usize), out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_auth_polkit_ret {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.complete.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_auth_sasl_init_ret {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.mechlist.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_auth_sasl_start_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.mech.pack(out)?
            + self.nil.pack(out)?
            + xdr_codec::pack_flex(&self.data, Some(REMOTE_AUTH_SASL_DATA_MAX as usize), out)?
            + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_auth_sasl_start_ret {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.complete.pack(out)?
            + self.nil.pack(out)?
            + xdr_codec::pack_flex(&self.data, Some(REMOTE_AUTH_SASL_DATA_MAX as usize), out)?
            + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_auth_sasl_step_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.nil.pack(out)?
            + xdr_codec::pack_flex(&self.data, Some(REMOTE_AUTH_SASL_DATA_MAX as usize), out)?
            + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_auth_sasl_step_ret {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.complete.pack(out)?
            + self.nil.pack(out)?
            + xdr_codec::pack_flex(&self.data, Some(REMOTE_AUTH_SASL_DATA_MAX as usize), out)?
            + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_auth_type {
    #[inline]
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok((*self as i32).pack(out)?)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_connect_baseline_cpu_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(
            xdr_codec::pack_flex(&self.xmlCPUs, Some(REMOTE_CPU_BASELINE_MAX as usize), out)?
                + self.flags.pack(out)?
                + 0,
        )
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_connect_baseline_cpu_ret {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.cpu.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_connect_compare_cpu_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.xml.pack(out)? + self.flags.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_connect_compare_cpu_ret {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.result.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out>
    for remote_connect_domain_event_callback_deregister_any_args
{
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.callbackID.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out>
    for remote_connect_domain_event_callback_register_any_args
{
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.eventID.pack(out)? + self.dom.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out>
    for remote_connect_domain_event_callback_register_any_ret
{
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.callbackID.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out>
    for remote_connect_domain_event_deregister_any_args
{
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.eventID.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_connect_domain_event_deregister_ret {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.cb_registered.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_connect_domain_event_register_any_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.eventID.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_connect_domain_event_register_ret {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.cb_registered.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_connect_domain_xml_from_native_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.nativeFormat.pack(out)? + self.nativeConfig.pack(out)? + self.flags.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_connect_domain_xml_from_native_ret {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.domainXml.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_connect_domain_xml_to_native_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.nativeFormat.pack(out)? + self.domainXml.pack(out)? + self.flags.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_connect_domain_xml_to_native_ret {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.nativeConfig.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_connect_event_connection_closed_msg {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.reason.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_connect_find_storage_pool_sources_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.type_.pack(out)? + self.srcSpec.pack(out)? + self.flags.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_connect_find_storage_pool_sources_ret {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.xml.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_connect_get_all_domain_stats_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(
            xdr_codec::pack_flex(&self.doms, Some(REMOTE_DOMAIN_LIST_MAX as usize), out)?
                + self.stats.pack(out)?
                + self.flags.pack(out)?
                + 0,
        )
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_connect_get_all_domain_stats_ret {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(xdr_codec::pack_flex(&self.retStats, Some(REMOTE_DOMAIN_LIST_MAX as usize), out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_connect_get_capabilities_ret {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.capabilities.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_connect_get_cpu_model_names_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.arch.pack(out)? + self.need_results.pack(out)? + self.flags.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_connect_get_cpu_model_names_ret {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(xdr_codec::pack_flex(
            &self.models,
            Some(REMOTE_CONNECT_CPU_MODELS_MAX as usize),
            out,
        )? + self.ret.pack(out)?
            + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_connect_get_domain_capabilities_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.emulatorbin.pack(out)?
            + self.arch.pack(out)?
            + self.machine.pack(out)?
            + self.virttype.pack(out)?
            + self.flags.pack(out)?
            + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_connect_get_domain_capabilities_ret {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.capabilities.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_connect_get_hostname_ret {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.hostname.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_connect_get_lib_version_ret {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.lib_ver.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_connect_get_max_vcpus_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.type_.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_connect_get_max_vcpus_ret {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.max_vcpus.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_connect_get_sysinfo_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.flags.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_connect_get_sysinfo_ret {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.sysinfo.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_connect_get_type_ret {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.type_.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_connect_get_uri_ret {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.uri.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_connect_get_version_ret {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.hv_ver.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_connect_is_secure_ret {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.secure.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_connect_list_all_domains_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.need_results.pack(out)? + self.flags.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_connect_list_all_domains_ret {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(
            xdr_codec::pack_flex(&self.domains, Some(REMOTE_DOMAIN_LIST_MAX as usize), out)?
                + self.ret.pack(out)?
                + 0,
        )
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_connect_list_all_interfaces_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.need_results.pack(out)? + self.flags.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_connect_list_all_interfaces_ret {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(
            xdr_codec::pack_flex(&self.ifaces, Some(REMOTE_INTERFACE_LIST_MAX as usize), out)?
                + self.ret.pack(out)?
                + 0,
        )
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_connect_list_all_networks_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.need_results.pack(out)? + self.flags.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_connect_list_all_networks_ret {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(
            xdr_codec::pack_flex(&self.nets, Some(REMOTE_NETWORK_LIST_MAX as usize), out)?
                + self.ret.pack(out)?
                + 0,
        )
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_connect_list_all_node_devices_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.need_results.pack(out)? + self.flags.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_connect_list_all_node_devices_ret {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(xdr_codec::pack_flex(
            &self.devices,
            Some(REMOTE_NODE_DEVICE_LIST_MAX as usize),
            out,
        )? + self.ret.pack(out)?
            + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_connect_list_all_nwfilters_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.need_results.pack(out)? + self.flags.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_connect_list_all_nwfilters_ret {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(
            xdr_codec::pack_flex(&self.filters, Some(REMOTE_NWFILTER_LIST_MAX as usize), out)?
                + self.ret.pack(out)?
                + 0,
        )
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_connect_list_all_secrets_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.need_results.pack(out)? + self.flags.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_connect_list_all_secrets_ret {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(
            xdr_codec::pack_flex(&self.secrets, Some(REMOTE_SECRET_LIST_MAX as usize), out)?
                + self.ret.pack(out)?
                + 0,
        )
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_connect_list_all_storage_pools_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.need_results.pack(out)? + self.flags.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_connect_list_all_storage_pools_ret {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(xdr_codec::pack_flex(
            &self.pools,
            Some(REMOTE_STORAGE_POOL_LIST_MAX as usize),
            out,
        )? + self.ret.pack(out)?
            + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_connect_list_defined_domains_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.maxnames.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_connect_list_defined_domains_ret {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(xdr_codec::pack_flex(&self.names, Some(REMOTE_DOMAIN_LIST_MAX as usize), out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_connect_list_defined_interfaces_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.maxnames.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_connect_list_defined_interfaces_ret {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(xdr_codec::pack_flex(&self.names, Some(REMOTE_INTERFACE_LIST_MAX as usize), out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_connect_list_defined_networks_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.maxnames.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_connect_list_defined_networks_ret {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(xdr_codec::pack_flex(&self.names, Some(REMOTE_NETWORK_LIST_MAX as usize), out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out>
    for remote_connect_list_defined_storage_pools_args
{
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.maxnames.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_connect_list_defined_storage_pools_ret {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(xdr_codec::pack_flex(
            &self.names,
            Some(REMOTE_STORAGE_POOL_LIST_MAX as usize),
            out,
        )? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_connect_list_domains_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.maxids.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_connect_list_domains_ret {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(xdr_codec::pack_flex(&self.ids, Some(REMOTE_DOMAIN_LIST_MAX as usize), out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_connect_list_interfaces_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.maxnames.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_connect_list_interfaces_ret {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(xdr_codec::pack_flex(&self.names, Some(REMOTE_INTERFACE_LIST_MAX as usize), out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_connect_list_networks_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.maxnames.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_connect_list_networks_ret {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(xdr_codec::pack_flex(&self.names, Some(REMOTE_NETWORK_LIST_MAX as usize), out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_connect_list_nwfilters_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.maxnames.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_connect_list_nwfilters_ret {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(xdr_codec::pack_flex(&self.names, Some(REMOTE_NWFILTER_LIST_MAX as usize), out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_connect_list_secrets_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.maxuuids.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_connect_list_secrets_ret {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(xdr_codec::pack_flex(&self.uuids, Some(REMOTE_SECRET_LIST_MAX as usize), out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_connect_list_storage_pools_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.maxnames.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_connect_list_storage_pools_ret {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(xdr_codec::pack_flex(
            &self.names,
            Some(REMOTE_STORAGE_POOL_LIST_MAX as usize),
            out,
        )? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out>
    for remote_connect_network_event_deregister_any_args
{
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.callbackID.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out>
    for remote_connect_network_event_register_any_args
{
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.eventID.pack(out)? + self.net.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_connect_network_event_register_any_ret {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.callbackID.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out>
    for remote_connect_node_device_event_deregister_any_args
{
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.callbackID.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out>
    for remote_connect_node_device_event_register_any_args
{
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.eventID.pack(out)? + self.dev.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out>
    for remote_connect_node_device_event_register_any_ret
{
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.callbackID.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_connect_num_of_defined_domains_ret {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.num.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_connect_num_of_defined_interfaces_ret {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.num.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_connect_num_of_defined_networks_ret {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.num.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out>
    for remote_connect_num_of_defined_storage_pools_ret
{
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.num.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_connect_num_of_domains_ret {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.num.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_connect_num_of_interfaces_ret {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.num.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_connect_num_of_networks_ret {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.num.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_connect_num_of_nwfilters_ret {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.num.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_connect_num_of_secrets_ret {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.num.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_connect_num_of_storage_pools_ret {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.num.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_connect_open_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.name.pack(out)? + self.flags.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out>
    for remote_connect_secret_event_deregister_any_args
{
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.callbackID.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_connect_secret_event_register_any_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.eventID.pack(out)? + self.secret.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_connect_secret_event_register_any_ret {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.callbackID.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out>
    for remote_connect_storage_pool_event_deregister_any_args
{
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.callbackID.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out>
    for remote_connect_storage_pool_event_register_any_args
{
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.eventID.pack(out)? + self.pool.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out>
    for remote_connect_storage_pool_event_register_any_ret
{
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.callbackID.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_connect_supports_feature_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.feature.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_connect_supports_feature_ret {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.supported.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_abort_job_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.dom.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_add_iothread_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.dom.pack(out)? + self.iothread_id.pack(out)? + self.flags.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_attach_device_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.dom.pack(out)? + self.xml.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_attach_device_flags_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.dom.pack(out)? + self.xml.pack(out)? + self.flags.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_block_commit_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.dom.pack(out)?
            + self.disk.pack(out)?
            + self.base.pack(out)?
            + self.top.pack(out)?
            + self.bandwidth.pack(out)?
            + self.flags.pack(out)?
            + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_block_copy_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.dom.pack(out)?
            + self.path.pack(out)?
            + self.destxml.pack(out)?
            + xdr_codec::pack_flex(
                &self.params,
                Some(REMOTE_DOMAIN_BLOCK_COPY_PARAMETERS_MAX as usize),
                out,
            )?
            + self.flags.pack(out)?
            + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_block_job_abort_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.dom.pack(out)? + self.path.pack(out)? + self.flags.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_block_job_set_speed_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.dom.pack(out)?
            + self.path.pack(out)?
            + self.bandwidth.pack(out)?
            + self.flags.pack(out)?
            + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_block_peek_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.dom.pack(out)?
            + self.path.pack(out)?
            + self.offset.pack(out)?
            + self.size.pack(out)?
            + self.flags.pack(out)?
            + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_block_peek_ret {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(xdr_codec::pack_opaque_flex(
            &self.buffer,
            Some(REMOTE_DOMAIN_BLOCK_PEEK_BUFFER_MAX as usize),
            out,
        )? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_block_pull_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.dom.pack(out)?
            + self.path.pack(out)?
            + self.bandwidth.pack(out)?
            + self.flags.pack(out)?
            + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_block_rebase_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.dom.pack(out)?
            + self.path.pack(out)?
            + self.base.pack(out)?
            + self.bandwidth.pack(out)?
            + self.flags.pack(out)?
            + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_block_resize_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.dom.pack(out)?
            + self.disk.pack(out)?
            + self.size.pack(out)?
            + self.flags.pack(out)?
            + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_block_stats_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.dom.pack(out)? + self.path.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_block_stats_flags_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.dom.pack(out)?
            + self.path.pack(out)?
            + self.nparams.pack(out)?
            + self.flags.pack(out)?
            + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_block_stats_flags_ret {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(xdr_codec::pack_flex(
            &self.params,
            Some(REMOTE_DOMAIN_BLOCK_STATS_PARAMETERS_MAX as usize),
            out,
        )? + self.nparams.pack(out)?
            + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_block_stats_ret {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.rd_req.pack(out)?
            + self.rd_bytes.pack(out)?
            + self.wr_req.pack(out)?
            + self.wr_bytes.pack(out)?
            + self.errs.pack(out)?
            + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_core_dump_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.dom.pack(out)? + self.to.pack(out)? + self.flags.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_core_dump_with_format_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.dom.pack(out)?
            + self.to.pack(out)?
            + self.dumpformat.pack(out)?
            + self.flags.pack(out)?
            + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_create_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.dom.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_create_with_files_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.dom.pack(out)? + self.flags.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_create_with_files_ret {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.dom.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_create_with_flags_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.dom.pack(out)? + self.flags.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_create_with_flags_ret {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.dom.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_create_xml_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.xml_desc.pack(out)? + self.flags.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_create_xml_ret {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.dom.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_create_xml_with_files_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.xml_desc.pack(out)? + self.flags.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_create_xml_with_files_ret {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.dom.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_define_xml_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.xml.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_define_xml_flags_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.xml.pack(out)? + self.flags.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_define_xml_flags_ret {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.dom.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_define_xml_ret {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.dom.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_del_iothread_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.dom.pack(out)? + self.iothread_id.pack(out)? + self.flags.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_destroy_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.dom.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_destroy_flags_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.dom.pack(out)? + self.flags.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_detach_device_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.dom.pack(out)? + self.xml.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_detach_device_flags_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.dom.pack(out)? + self.xml.pack(out)? + self.flags.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_disk_error {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.disk.pack(out)? + self.error.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_event_balloon_change_msg {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.dom.pack(out)? + self.actual.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_event_block_job_2_msg {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.callbackID.pack(out)?
            + self.dom.pack(out)?
            + self.dst.pack(out)?
            + self.type_.pack(out)?
            + self.status.pack(out)?
            + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_event_block_job_msg {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.dom.pack(out)?
            + self.path.pack(out)?
            + self.type_.pack(out)?
            + self.status.pack(out)?
            + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_event_block_threshold_msg {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.callbackID.pack(out)?
            + self.dom.pack(out)?
            + self.dev.pack(out)?
            + self.path.pack(out)?
            + self.threshold.pack(out)?
            + self.excess.pack(out)?
            + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out>
    for remote_domain_event_callback_agent_lifecycle_msg
{
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.callbackID.pack(out)?
            + self.dom.pack(out)?
            + self.state.pack(out)?
            + self.reason.pack(out)?
            + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out>
    for remote_domain_event_callback_balloon_change_msg
{
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.callbackID.pack(out)? + self.msg.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_event_callback_block_job_msg {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.callbackID.pack(out)? + self.msg.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out>
    for remote_domain_event_callback_control_error_msg
{
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.callbackID.pack(out)? + self.msg.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_event_callback_device_added_msg {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.callbackID.pack(out)? + self.dom.pack(out)? + self.devAlias.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out>
    for remote_domain_event_callback_device_removal_failed_msg
{
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.callbackID.pack(out)? + self.dom.pack(out)? + self.devAlias.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out>
    for remote_domain_event_callback_device_removed_msg
{
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.callbackID.pack(out)? + self.msg.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_event_callback_disk_change_msg {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.callbackID.pack(out)? + self.msg.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_event_callback_graphics_msg {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.callbackID.pack(out)? + self.msg.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_event_callback_io_error_msg {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.callbackID.pack(out)? + self.msg.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out>
    for remote_domain_event_callback_io_error_reason_msg
{
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.callbackID.pack(out)? + self.msg.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out>
    for remote_domain_event_callback_job_completed_msg
{
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.callbackID.pack(out)?
            + self.dom.pack(out)?
            + xdr_codec::pack_flex(
                &self.params,
                Some(REMOTE_DOMAIN_JOB_STATS_MAX as usize),
                out,
            )?
            + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_event_callback_lifecycle_msg {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.callbackID.pack(out)? + self.msg.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out>
    for remote_domain_event_callback_metadata_change_msg
{
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.callbackID.pack(out)?
            + self.dom.pack(out)?
            + self.type_.pack(out)?
            + self.nsuri.pack(out)?
            + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out>
    for remote_domain_event_callback_migration_iteration_msg
{
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.callbackID.pack(out)? + self.dom.pack(out)? + self.iteration.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out>
    for remote_domain_event_callback_pmsuspend_disk_msg
{
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.callbackID.pack(out)? + self.reason.pack(out)? + self.msg.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_event_callback_pmsuspend_msg {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.callbackID.pack(out)? + self.reason.pack(out)? + self.msg.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_event_callback_pmwakeup_msg {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.callbackID.pack(out)? + self.reason.pack(out)? + self.msg.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_event_callback_reboot_msg {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.callbackID.pack(out)? + self.msg.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_event_callback_rtc_change_msg {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.callbackID.pack(out)? + self.msg.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_event_callback_tray_change_msg {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.callbackID.pack(out)? + self.msg.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_event_callback_tunable_msg {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.callbackID.pack(out)?
            + self.dom.pack(out)?
            + xdr_codec::pack_flex(
                &self.params,
                Some(REMOTE_DOMAIN_EVENT_TUNABLE_MAX as usize),
                out,
            )?
            + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_event_callback_watchdog_msg {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.callbackID.pack(out)? + self.msg.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_event_control_error_msg {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.dom.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_event_device_removed_msg {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.dom.pack(out)? + self.devAlias.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_event_disk_change_msg {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.dom.pack(out)?
            + self.oldSrcPath.pack(out)?
            + self.newSrcPath.pack(out)?
            + self.devAlias.pack(out)?
            + self.reason.pack(out)?
            + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_event_graphics_address {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.family.pack(out)? + self.node.pack(out)? + self.service.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_event_graphics_identity {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.type_.pack(out)? + self.name.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_event_graphics_msg {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.dom.pack(out)?
            + self.phase.pack(out)?
            + self.local.pack(out)?
            + self.remote.pack(out)?
            + self.authScheme.pack(out)?
            + xdr_codec::pack_flex(
                &self.subject,
                Some(REMOTE_DOMAIN_EVENT_GRAPHICS_IDENTITY_MAX as usize),
                out,
            )?
            + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_event_io_error_msg {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.dom.pack(out)?
            + self.srcPath.pack(out)?
            + self.devAlias.pack(out)?
            + self.action.pack(out)?
            + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_event_io_error_reason_msg {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.dom.pack(out)?
            + self.srcPath.pack(out)?
            + self.devAlias.pack(out)?
            + self.action.pack(out)?
            + self.reason.pack(out)?
            + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_event_lifecycle_msg {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.dom.pack(out)? + self.event.pack(out)? + self.detail.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_event_pmsuspend_disk_msg {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.dom.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_event_pmsuspend_msg {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.dom.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_event_pmwakeup_msg {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.dom.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_event_reboot_msg {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.dom.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_event_rtc_change_msg {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.dom.pack(out)? + self.offset.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_event_tray_change_msg {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.dom.pack(out)? + self.devAlias.pack(out)? + self.reason.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_event_watchdog_msg {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.dom.pack(out)? + self.action.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_fsfreeze_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.dom.pack(out)?
            + xdr_codec::pack_flex(
                &self.mountpoints,
                Some(REMOTE_DOMAIN_FSFREEZE_MOUNTPOINTS_MAX as usize),
                out,
            )?
            + self.flags.pack(out)?
            + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_fsfreeze_ret {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.filesystems.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_fsinfo {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.mountpoint.pack(out)?
            + self.name.pack(out)?
            + self.fstype.pack(out)?
            + xdr_codec::pack_flex(
                &self.dev_aliases,
                Some(REMOTE_DOMAIN_FSINFO_DISKS_MAX as usize),
                out,
            )?
            + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_fsthaw_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.dom.pack(out)?
            + xdr_codec::pack_flex(
                &self.mountpoints,
                Some(REMOTE_DOMAIN_FSFREEZE_MOUNTPOINTS_MAX as usize),
                out,
            )?
            + self.flags.pack(out)?
            + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_fsthaw_ret {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.filesystems.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_fstrim_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.dom.pack(out)?
            + self.mountPoint.pack(out)?
            + self.minimum.pack(out)?
            + self.flags.pack(out)?
            + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_get_autostart_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.dom.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_get_autostart_ret {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.autostart.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_get_blkio_parameters_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.dom.pack(out)? + self.nparams.pack(out)? + self.flags.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_get_blkio_parameters_ret {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(xdr_codec::pack_flex(
            &self.params,
            Some(REMOTE_DOMAIN_BLKIO_PARAMETERS_MAX as usize),
            out,
        )? + self.nparams.pack(out)?
            + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_get_block_info_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.dom.pack(out)? + self.path.pack(out)? + self.flags.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_get_block_info_ret {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.allocation.pack(out)? + self.capacity.pack(out)? + self.physical.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_get_block_io_tune_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.dom.pack(out)?
            + self.disk.pack(out)?
            + self.nparams.pack(out)?
            + self.flags.pack(out)?
            + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_get_block_io_tune_ret {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(xdr_codec::pack_flex(
            &self.params,
            Some(REMOTE_DOMAIN_BLOCK_IO_TUNE_PARAMETERS_MAX as usize),
            out,
        )? + self.nparams.pack(out)?
            + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_get_block_job_info_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.dom.pack(out)? + self.path.pack(out)? + self.flags.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_get_block_job_info_ret {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.found.pack(out)?
            + self.type_.pack(out)?
            + self.bandwidth.pack(out)?
            + self.cur.pack(out)?
            + self.end.pack(out)?
            + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_get_control_info_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.dom.pack(out)? + self.flags.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_get_control_info_ret {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.state.pack(out)? + self.details.pack(out)? + self.stateTime.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_get_cpu_stats_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.dom.pack(out)?
            + self.nparams.pack(out)?
            + self.start_cpu.pack(out)?
            + self.ncpus.pack(out)?
            + self.flags.pack(out)?
            + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_get_cpu_stats_ret {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(xdr_codec::pack_flex(
            &self.params,
            Some(REMOTE_DOMAIN_GET_CPU_STATS_MAX as usize),
            out,
        )? + self.nparams.pack(out)?
            + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_get_disk_errors_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.dom.pack(out)? + self.maxerrors.pack(out)? + self.flags.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_get_disk_errors_ret {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(xdr_codec::pack_flex(
            &self.errors,
            Some(REMOTE_DOMAIN_DISK_ERRORS_MAX as usize),
            out,
        )? + self.nerrors.pack(out)?
            + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_get_emulator_pin_info_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.dom.pack(out)? + self.maplen.pack(out)? + self.flags.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_get_emulator_pin_info_ret {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(
            xdr_codec::pack_opaque_flex(&self.cpumaps, Some(REMOTE_CPUMAPS_MAX as usize), out)?
                + self.ret.pack(out)?
                + 0,
        )
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_get_fsinfo_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.dom.pack(out)? + self.flags.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_get_fsinfo_ret {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(
            xdr_codec::pack_flex(&self.info, Some(REMOTE_DOMAIN_FSINFO_MAX as usize), out)?
                + self.ret.pack(out)?
                + 0,
        )
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_get_guest_vcpus_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.dom.pack(out)? + self.flags.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_get_guest_vcpus_ret {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(xdr_codec::pack_flex(
            &self.params,
            Some(REMOTE_DOMAIN_GUEST_VCPU_PARAMS_MAX as usize),
            out,
        )? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_get_hostname_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.dom.pack(out)? + self.flags.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_get_hostname_ret {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.hostname.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_get_info_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.dom.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_get_info_ret {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.state.pack(out)?
            + self.maxMem.pack(out)?
            + self.memory.pack(out)?
            + self.nrVirtCpu.pack(out)?
            + self.cpuTime.pack(out)?
            + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_get_interface_parameters_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.dom.pack(out)?
            + self.device.pack(out)?
            + self.nparams.pack(out)?
            + self.flags.pack(out)?
            + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_get_interface_parameters_ret {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(xdr_codec::pack_flex(
            &self.params,
            Some(REMOTE_DOMAIN_INTERFACE_PARAMETERS_MAX as usize),
            out,
        )? + self.nparams.pack(out)?
            + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_get_iothread_info_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.dom.pack(out)? + self.flags.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_get_iothread_info_ret {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(
            xdr_codec::pack_flex(&self.info, Some(REMOTE_IOTHREAD_INFO_MAX as usize), out)?
                + self.ret.pack(out)?
                + 0,
        )
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_get_job_info_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.dom.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_get_job_info_ret {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.type_.pack(out)?
            + self.timeElapsed.pack(out)?
            + self.timeRemaining.pack(out)?
            + self.dataTotal.pack(out)?
            + self.dataProcessed.pack(out)?
            + self.dataRemaining.pack(out)?
            + self.memTotal.pack(out)?
            + self.memProcessed.pack(out)?
            + self.memRemaining.pack(out)?
            + self.fileTotal.pack(out)?
            + self.fileProcessed.pack(out)?
            + self.fileRemaining.pack(out)?
            + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_get_job_stats_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.dom.pack(out)? + self.flags.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_get_job_stats_ret {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.type_.pack(out)?
            + xdr_codec::pack_flex(
                &self.params,
                Some(REMOTE_DOMAIN_JOB_STATS_MAX as usize),
                out,
            )?
            + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_get_max_memory_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.dom.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_get_max_memory_ret {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.memory.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_get_max_vcpus_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.dom.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_get_max_vcpus_ret {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.num.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_get_memory_parameters_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.dom.pack(out)? + self.nparams.pack(out)? + self.flags.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_get_memory_parameters_ret {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(xdr_codec::pack_flex(
            &self.params,
            Some(REMOTE_DOMAIN_MEMORY_PARAMETERS_MAX as usize),
            out,
        )? + self.nparams.pack(out)?
            + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_get_metadata_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.dom.pack(out)?
            + self.type_.pack(out)?
            + self.uri.pack(out)?
            + self.flags.pack(out)?
            + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_get_metadata_ret {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.metadata.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_get_numa_parameters_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.dom.pack(out)? + self.nparams.pack(out)? + self.flags.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_get_numa_parameters_ret {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(xdr_codec::pack_flex(
            &self.params,
            Some(REMOTE_DOMAIN_NUMA_PARAMETERS_MAX as usize),
            out,
        )? + self.nparams.pack(out)?
            + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_get_os_type_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.dom.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_get_os_type_ret {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.type_.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_get_perf_events_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.dom.pack(out)? + self.flags.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_get_perf_events_ret {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(xdr_codec::pack_flex(
            &self.params,
            Some(REMOTE_DOMAIN_PERF_EVENTS_MAX as usize),
            out,
        )? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_get_scheduler_parameters_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.dom.pack(out)? + self.nparams.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out>
    for remote_domain_get_scheduler_parameters_flags_args
{
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.dom.pack(out)? + self.nparams.pack(out)? + self.flags.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out>
    for remote_domain_get_scheduler_parameters_flags_ret
{
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(xdr_codec::pack_flex(
            &self.params,
            Some(REMOTE_DOMAIN_SCHEDULER_PARAMETERS_MAX as usize),
            out,
        )? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_get_scheduler_parameters_ret {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(xdr_codec::pack_flex(
            &self.params,
            Some(REMOTE_DOMAIN_SCHEDULER_PARAMETERS_MAX as usize),
            out,
        )? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_get_scheduler_type_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.dom.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_get_scheduler_type_ret {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.type_.pack(out)? + self.nparams.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_get_security_label_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.dom.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_get_security_label_list_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.dom.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_get_security_label_list_ret {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(xdr_codec::pack_flex(
            &self.labels,
            Some(REMOTE_SECURITY_LABEL_LIST_MAX as usize),
            out,
        )? + self.ret.pack(out)?
            + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_get_security_label_ret {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(
            xdr_codec::pack_flex(&self.label, Some(REMOTE_SECURITY_LABEL_MAX as usize), out)?
                + self.enforcing.pack(out)?
                + 0,
        )
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_get_state_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.dom.pack(out)? + self.flags.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_get_state_ret {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.state.pack(out)? + self.reason.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_get_time_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.dom.pack(out)? + self.flags.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_get_time_ret {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.seconds.pack(out)? + self.nseconds.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_get_vcpu_pin_info_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.dom.pack(out)?
            + self.ncpumaps.pack(out)?
            + self.maplen.pack(out)?
            + self.flags.pack(out)?
            + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_get_vcpu_pin_info_ret {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(
            xdr_codec::pack_opaque_flex(&self.cpumaps, Some(REMOTE_CPUMAPS_MAX as usize), out)?
                + self.num.pack(out)?
                + 0,
        )
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_get_vcpus_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.dom.pack(out)? + self.maxinfo.pack(out)? + self.maplen.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_get_vcpus_flags_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.dom.pack(out)? + self.flags.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_get_vcpus_flags_ret {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.num.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_get_vcpus_ret {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(
            xdr_codec::pack_flex(&self.info, Some(REMOTE_VCPUINFO_MAX as usize), out)?
                + xdr_codec::pack_opaque_flex(
                    &self.cpumaps,
                    Some(REMOTE_CPUMAPS_MAX as usize),
                    out,
                )?
                + 0,
        )
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_get_xml_desc_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.dom.pack(out)? + self.flags.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_get_xml_desc_ret {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.xml.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_has_current_snapshot_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.dom.pack(out)? + self.flags.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_has_current_snapshot_ret {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.result.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_has_managed_save_image_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.dom.pack(out)? + self.flags.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_has_managed_save_image_ret {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.result.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_inject_nmi_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.dom.pack(out)? + self.flags.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_interface {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.name.pack(out)?
            + self.hwaddr.pack(out)?
            + xdr_codec::pack_flex(&self.addrs, Some(REMOTE_DOMAIN_IP_ADDR_MAX as usize), out)?
            + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_interface_addresses_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.dom.pack(out)? + self.source.pack(out)? + self.flags.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_interface_addresses_ret {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(xdr_codec::pack_flex(
            &self.ifaces,
            Some(REMOTE_DOMAIN_INTERFACE_MAX as usize),
            out,
        )? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_interface_stats_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.dom.pack(out)? + self.path.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_interface_stats_ret {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.rx_bytes.pack(out)?
            + self.rx_packets.pack(out)?
            + self.rx_errs.pack(out)?
            + self.rx_drop.pack(out)?
            + self.tx_bytes.pack(out)?
            + self.tx_packets.pack(out)?
            + self.tx_errs.pack(out)?
            + self.tx_drop.pack(out)?
            + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_iothread_info {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.iothread_id.pack(out)?
            + xdr_codec::pack_opaque_flex(&self.cpumap, Some(REMOTE_CPUMAP_MAX as usize), out)?
            + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_ip_addr {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.type_.pack(out)? + self.addr.pack(out)? + self.prefix.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_is_active_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.dom.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_is_active_ret {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.active.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_is_persistent_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.dom.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_is_persistent_ret {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.persistent.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_is_updated_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.dom.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_is_updated_ret {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.updated.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_list_all_snapshots_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.dom.pack(out)? + self.need_results.pack(out)? + self.flags.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_list_all_snapshots_ret {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(xdr_codec::pack_flex(
            &self.snapshots,
            Some(REMOTE_DOMAIN_SNAPSHOT_LIST_MAX as usize),
            out,
        )? + self.ret.pack(out)?
            + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_lookup_by_id_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.id.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_lookup_by_id_ret {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.dom.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_lookup_by_name_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.name.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_lookup_by_name_ret {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.dom.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_lookup_by_uuid_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.uuid.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_lookup_by_uuid_ret {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.dom.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_managed_save_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.dom.pack(out)? + self.flags.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_managed_save_remove_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.dom.pack(out)? + self.flags.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_memory_peek_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.dom.pack(out)?
            + self.offset.pack(out)?
            + self.size.pack(out)?
            + self.flags.pack(out)?
            + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_memory_peek_ret {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(xdr_codec::pack_opaque_flex(
            &self.buffer,
            Some(REMOTE_DOMAIN_MEMORY_PEEK_BUFFER_MAX as usize),
            out,
        )? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_memory_stat {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.tag.pack(out)? + self.val.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_memory_stats_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.dom.pack(out)? + self.maxStats.pack(out)? + self.flags.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_memory_stats_ret {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(xdr_codec::pack_flex(
            &self.stats,
            Some(REMOTE_DOMAIN_MEMORY_STATS_MAX as usize),
            out,
        )? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_migrate_begin3_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.dom.pack(out)?
            + self.xmlin.pack(out)?
            + self.flags.pack(out)?
            + self.dname.pack(out)?
            + self.resource.pack(out)?
            + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_migrate_begin3_params_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.dom.pack(out)?
            + xdr_codec::pack_flex(
                &self.params,
                Some(REMOTE_DOMAIN_MIGRATE_PARAM_LIST_MAX as usize),
                out,
            )?
            + self.flags.pack(out)?
            + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_migrate_begin3_params_ret {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(xdr_codec::pack_opaque_flex(
            &self.cookie_out,
            Some(REMOTE_MIGRATE_COOKIE_MAX as usize),
            out,
        )? + self.xml.pack(out)?
            + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_migrate_begin3_ret {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(xdr_codec::pack_opaque_flex(
            &self.cookie_out,
            Some(REMOTE_MIGRATE_COOKIE_MAX as usize),
            out,
        )? + self.xml.pack(out)?
            + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_migrate_confirm3_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.dom.pack(out)?
            + xdr_codec::pack_opaque_flex(
                &self.cookie_in,
                Some(REMOTE_MIGRATE_COOKIE_MAX as usize),
                out,
            )?
            + self.flags.pack(out)?
            + self.cancelled.pack(out)?
            + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_migrate_confirm3_params_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.dom.pack(out)?
            + xdr_codec::pack_flex(
                &self.params,
                Some(REMOTE_DOMAIN_MIGRATE_PARAM_LIST_MAX as usize),
                out,
            )?
            + xdr_codec::pack_opaque_flex(
                &self.cookie_in,
                Some(REMOTE_MIGRATE_COOKIE_MAX as usize),
                out,
            )?
            + self.flags.pack(out)?
            + self.cancelled.pack(out)?
            + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_migrate_finish2_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.dname.pack(out)?
            + xdr_codec::pack_opaque_flex(
                &self.cookie,
                Some(REMOTE_MIGRATE_COOKIE_MAX as usize),
                out,
            )?
            + self.uri.pack(out)?
            + self.flags.pack(out)?
            + self.retcode.pack(out)?
            + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_migrate_finish2_ret {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.ddom.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_migrate_finish3_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.dname.pack(out)?
            + xdr_codec::pack_opaque_flex(
                &self.cookie_in,
                Some(REMOTE_MIGRATE_COOKIE_MAX as usize),
                out,
            )?
            + self.dconnuri.pack(out)?
            + self.uri.pack(out)?
            + self.flags.pack(out)?
            + self.cancelled.pack(out)?
            + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_migrate_finish3_params_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(xdr_codec::pack_flex(
            &self.params,
            Some(REMOTE_DOMAIN_MIGRATE_PARAM_LIST_MAX as usize),
            out,
        )? + xdr_codec::pack_opaque_flex(
            &self.cookie_in,
            Some(REMOTE_MIGRATE_COOKIE_MAX as usize),
            out,
        )? + self.flags.pack(out)?
            + self.cancelled.pack(out)?
            + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_migrate_finish3_params_ret {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.dom.pack(out)?
            + xdr_codec::pack_opaque_flex(
                &self.cookie_out,
                Some(REMOTE_MIGRATE_COOKIE_MAX as usize),
                out,
            )?
            + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_migrate_finish3_ret {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.dom.pack(out)?
            + xdr_codec::pack_opaque_flex(
                &self.cookie_out,
                Some(REMOTE_MIGRATE_COOKIE_MAX as usize),
                out,
            )?
            + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_migrate_finish_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.dname.pack(out)?
            + xdr_codec::pack_opaque_flex(
                &self.cookie,
                Some(REMOTE_MIGRATE_COOKIE_MAX as usize),
                out,
            )?
            + self.uri.pack(out)?
            + self.flags.pack(out)?
            + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_migrate_finish_ret {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.ddom.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out>
    for remote_domain_migrate_get_compression_cache_args
{
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.dom.pack(out)? + self.flags.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out>
    for remote_domain_migrate_get_compression_cache_ret
{
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.cacheSize.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_migrate_get_max_speed_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.dom.pack(out)? + self.flags.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_migrate_get_max_speed_ret {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.bandwidth.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_migrate_perform3_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.dom.pack(out)?
            + self.xmlin.pack(out)?
            + xdr_codec::pack_opaque_flex(
                &self.cookie_in,
                Some(REMOTE_MIGRATE_COOKIE_MAX as usize),
                out,
            )?
            + self.dconnuri.pack(out)?
            + self.uri.pack(out)?
            + self.flags.pack(out)?
            + self.dname.pack(out)?
            + self.resource.pack(out)?
            + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_migrate_perform3_params_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.dom.pack(out)?
            + self.dconnuri.pack(out)?
            + xdr_codec::pack_flex(
                &self.params,
                Some(REMOTE_DOMAIN_MIGRATE_PARAM_LIST_MAX as usize),
                out,
            )?
            + xdr_codec::pack_opaque_flex(
                &self.cookie_in,
                Some(REMOTE_MIGRATE_COOKIE_MAX as usize),
                out,
            )?
            + self.flags.pack(out)?
            + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_migrate_perform3_params_ret {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(xdr_codec::pack_opaque_flex(
            &self.cookie_out,
            Some(REMOTE_MIGRATE_COOKIE_MAX as usize),
            out,
        )? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_migrate_perform3_ret {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(xdr_codec::pack_opaque_flex(
            &self.cookie_out,
            Some(REMOTE_MIGRATE_COOKIE_MAX as usize),
            out,
        )? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_migrate_perform_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.dom.pack(out)?
            + xdr_codec::pack_opaque_flex(
                &self.cookie,
                Some(REMOTE_MIGRATE_COOKIE_MAX as usize),
                out,
            )?
            + self.uri.pack(out)?
            + self.flags.pack(out)?
            + self.dname.pack(out)?
            + self.resource.pack(out)?
            + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_migrate_prepare2_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.uri_in.pack(out)?
            + self.flags.pack(out)?
            + self.dname.pack(out)?
            + self.resource.pack(out)?
            + self.dom_xml.pack(out)?
            + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_migrate_prepare2_ret {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(xdr_codec::pack_opaque_flex(
            &self.cookie,
            Some(REMOTE_MIGRATE_COOKIE_MAX as usize),
            out,
        )? + self.uri_out.pack(out)?
            + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_migrate_prepare3_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(xdr_codec::pack_opaque_flex(
            &self.cookie_in,
            Some(REMOTE_MIGRATE_COOKIE_MAX as usize),
            out,
        )? + self.uri_in.pack(out)?
            + self.flags.pack(out)?
            + self.dname.pack(out)?
            + self.resource.pack(out)?
            + self.dom_xml.pack(out)?
            + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_migrate_prepare3_params_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(xdr_codec::pack_flex(
            &self.params,
            Some(REMOTE_DOMAIN_MIGRATE_PARAM_LIST_MAX as usize),
            out,
        )? + xdr_codec::pack_opaque_flex(
            &self.cookie_in,
            Some(REMOTE_MIGRATE_COOKIE_MAX as usize),
            out,
        )? + self.flags.pack(out)?
            + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_migrate_prepare3_params_ret {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(xdr_codec::pack_opaque_flex(
            &self.cookie_out,
            Some(REMOTE_MIGRATE_COOKIE_MAX as usize),
            out,
        )? + self.uri_out.pack(out)?
            + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_migrate_prepare3_ret {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(xdr_codec::pack_opaque_flex(
            &self.cookie_out,
            Some(REMOTE_MIGRATE_COOKIE_MAX as usize),
            out,
        )? + self.uri_out.pack(out)?
            + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_migrate_prepare_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.uri_in.pack(out)?
            + self.flags.pack(out)?
            + self.dname.pack(out)?
            + self.resource.pack(out)?
            + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_migrate_prepare_ret {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(xdr_codec::pack_opaque_flex(
            &self.cookie,
            Some(REMOTE_MIGRATE_COOKIE_MAX as usize),
            out,
        )? + self.uri_out.pack(out)?
            + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_migrate_prepare_tunnel3_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(xdr_codec::pack_opaque_flex(
            &self.cookie_in,
            Some(REMOTE_MIGRATE_COOKIE_MAX as usize),
            out,
        )? + self.flags.pack(out)?
            + self.dname.pack(out)?
            + self.resource.pack(out)?
            + self.dom_xml.pack(out)?
            + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out>
    for remote_domain_migrate_prepare_tunnel3_params_args
{
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(xdr_codec::pack_flex(
            &self.params,
            Some(REMOTE_DOMAIN_MIGRATE_PARAM_LIST_MAX as usize),
            out,
        )? + xdr_codec::pack_opaque_flex(
            &self.cookie_in,
            Some(REMOTE_MIGRATE_COOKIE_MAX as usize),
            out,
        )? + self.flags.pack(out)?
            + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out>
    for remote_domain_migrate_prepare_tunnel3_params_ret
{
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(xdr_codec::pack_opaque_flex(
            &self.cookie_out,
            Some(REMOTE_MIGRATE_COOKIE_MAX as usize),
            out,
        )? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_migrate_prepare_tunnel3_ret {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(xdr_codec::pack_opaque_flex(
            &self.cookie_out,
            Some(REMOTE_MIGRATE_COOKIE_MAX as usize),
            out,
        )? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_migrate_prepare_tunnel_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.flags.pack(out)?
            + self.dname.pack(out)?
            + self.resource.pack(out)?
            + self.dom_xml.pack(out)?
            + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out>
    for remote_domain_migrate_set_compression_cache_args
{
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.dom.pack(out)? + self.cacheSize.pack(out)? + self.flags.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_migrate_set_max_downtime_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.dom.pack(out)? + self.downtime.pack(out)? + self.flags.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_migrate_set_max_speed_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.dom.pack(out)? + self.bandwidth.pack(out)? + self.flags.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_migrate_start_post_copy_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.dom.pack(out)? + self.flags.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_open_channel_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.dom.pack(out)? + self.name.pack(out)? + self.flags.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_open_console_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.dom.pack(out)? + self.dev_name.pack(out)? + self.flags.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_open_graphics_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.dom.pack(out)? + self.idx.pack(out)? + self.flags.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_open_graphics_fd_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.dom.pack(out)? + self.idx.pack(out)? + self.flags.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_pin_emulator_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.dom.pack(out)?
            + xdr_codec::pack_opaque_flex(&self.cpumap, Some(REMOTE_CPUMAP_MAX as usize), out)?
            + self.flags.pack(out)?
            + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_pin_iothread_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.dom.pack(out)?
            + self.iothreads_id.pack(out)?
            + xdr_codec::pack_opaque_flex(&self.cpumap, Some(REMOTE_CPUMAP_MAX as usize), out)?
            + self.flags.pack(out)?
            + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_pin_vcpu_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.dom.pack(out)?
            + self.vcpu.pack(out)?
            + xdr_codec::pack_opaque_flex(&self.cpumap, Some(REMOTE_CPUMAP_MAX as usize), out)?
            + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_pin_vcpu_flags_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.dom.pack(out)?
            + self.vcpu.pack(out)?
            + xdr_codec::pack_opaque_flex(&self.cpumap, Some(REMOTE_CPUMAP_MAX as usize), out)?
            + self.flags.pack(out)?
            + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_pm_suspend_for_duration_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.dom.pack(out)?
            + self.target.pack(out)?
            + self.duration.pack(out)?
            + self.flags.pack(out)?
            + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_pm_wakeup_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.dom.pack(out)? + self.flags.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_reboot_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.dom.pack(out)? + self.flags.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_rename_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.dom.pack(out)? + self.new_name.pack(out)? + self.flags.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_rename_ret {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.retcode.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_reset_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.dom.pack(out)? + self.flags.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_restore_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.from.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_restore_flags_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.from.pack(out)? + self.dxml.pack(out)? + self.flags.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_resume_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.dom.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_revert_to_snapshot_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.snap.pack(out)? + self.flags.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_save_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.dom.pack(out)? + self.to.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_save_flags_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.dom.pack(out)?
            + self.to.pack(out)?
            + self.dxml.pack(out)?
            + self.flags.pack(out)?
            + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_save_image_define_xml_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.file.pack(out)? + self.dxml.pack(out)? + self.flags.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_save_image_get_xml_desc_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.file.pack(out)? + self.flags.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_save_image_get_xml_desc_ret {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.xml.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_screenshot_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.dom.pack(out)? + self.screen.pack(out)? + self.flags.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_screenshot_ret {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.mime.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_send_key_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.dom.pack(out)?
            + self.codeset.pack(out)?
            + self.holdtime.pack(out)?
            + xdr_codec::pack_flex(
                &self.keycodes,
                Some(REMOTE_DOMAIN_SEND_KEY_MAX as usize),
                out,
            )?
            + self.flags.pack(out)?
            + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_send_process_signal_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.dom.pack(out)?
            + self.pid_value.pack(out)?
            + self.signum.pack(out)?
            + self.flags.pack(out)?
            + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_set_autostart_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.dom.pack(out)? + self.autostart.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_set_blkio_parameters_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.dom.pack(out)?
            + xdr_codec::pack_flex(
                &self.params,
                Some(REMOTE_DOMAIN_BLKIO_PARAMETERS_MAX as usize),
                out,
            )?
            + self.flags.pack(out)?
            + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_set_block_io_tune_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.dom.pack(out)?
            + self.disk.pack(out)?
            + xdr_codec::pack_flex(
                &self.params,
                Some(REMOTE_DOMAIN_BLOCK_IO_TUNE_PARAMETERS_MAX as usize),
                out,
            )?
            + self.flags.pack(out)?
            + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_set_block_threshold_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.dom.pack(out)?
            + self.dev.pack(out)?
            + self.threshold.pack(out)?
            + self.flags.pack(out)?
            + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_set_guest_vcpus_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.dom.pack(out)?
            + self.cpumap.pack(out)?
            + self.state.pack(out)?
            + self.flags.pack(out)?
            + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_set_interface_parameters_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.dom.pack(out)?
            + self.device.pack(out)?
            + xdr_codec::pack_flex(
                &self.params,
                Some(REMOTE_DOMAIN_INTERFACE_PARAMETERS_MAX as usize),
                out,
            )?
            + self.flags.pack(out)?
            + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_set_max_memory_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.dom.pack(out)? + self.memory.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_set_memory_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.dom.pack(out)? + self.memory.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_set_memory_flags_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.dom.pack(out)? + self.memory.pack(out)? + self.flags.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_set_memory_parameters_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.dom.pack(out)?
            + xdr_codec::pack_flex(
                &self.params,
                Some(REMOTE_DOMAIN_MEMORY_PARAMETERS_MAX as usize),
                out,
            )?
            + self.flags.pack(out)?
            + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_set_memory_stats_period_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.dom.pack(out)? + self.period.pack(out)? + self.flags.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_set_metadata_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.dom.pack(out)?
            + self.type_.pack(out)?
            + self.metadata.pack(out)?
            + self.key.pack(out)?
            + self.uri.pack(out)?
            + self.flags.pack(out)?
            + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_set_numa_parameters_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.dom.pack(out)?
            + xdr_codec::pack_flex(
                &self.params,
                Some(REMOTE_DOMAIN_NUMA_PARAMETERS_MAX as usize),
                out,
            )?
            + self.flags.pack(out)?
            + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_set_perf_events_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.dom.pack(out)?
            + xdr_codec::pack_flex(
                &self.params,
                Some(REMOTE_DOMAIN_PERF_EVENTS_MAX as usize),
                out,
            )?
            + self.flags.pack(out)?
            + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_set_scheduler_parameters_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.dom.pack(out)?
            + xdr_codec::pack_flex(
                &self.params,
                Some(REMOTE_DOMAIN_SCHEDULER_PARAMETERS_MAX as usize),
                out,
            )?
            + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out>
    for remote_domain_set_scheduler_parameters_flags_args
{
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.dom.pack(out)?
            + xdr_codec::pack_flex(
                &self.params,
                Some(REMOTE_DOMAIN_SCHEDULER_PARAMETERS_MAX as usize),
                out,
            )?
            + self.flags.pack(out)?
            + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_set_time_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.dom.pack(out)?
            + self.seconds.pack(out)?
            + self.nseconds.pack(out)?
            + self.flags.pack(out)?
            + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_set_user_password_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.dom.pack(out)?
            + self.user.pack(out)?
            + self.password.pack(out)?
            + self.flags.pack(out)?
            + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_set_vcpu_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.dom.pack(out)?
            + self.cpumap.pack(out)?
            + self.state.pack(out)?
            + self.flags.pack(out)?
            + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_set_vcpus_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.dom.pack(out)? + self.nvcpus.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_set_vcpus_flags_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.dom.pack(out)? + self.nvcpus.pack(out)? + self.flags.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_shutdown_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.dom.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_shutdown_flags_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.dom.pack(out)? + self.flags.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_snapshot_create_xml_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.dom.pack(out)? + self.xml_desc.pack(out)? + self.flags.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_snapshot_create_xml_ret {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.snap.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_snapshot_current_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.dom.pack(out)? + self.flags.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_snapshot_current_ret {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.snap.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_snapshot_delete_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.snap.pack(out)? + self.flags.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_snapshot_get_parent_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.snap.pack(out)? + self.flags.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_snapshot_get_parent_ret {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.snap.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_snapshot_get_xml_desc_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.snap.pack(out)? + self.flags.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_snapshot_get_xml_desc_ret {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.xml.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_snapshot_has_metadata_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.snap.pack(out)? + self.flags.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_snapshot_has_metadata_ret {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.metadata.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_snapshot_is_current_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.snap.pack(out)? + self.flags.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_snapshot_is_current_ret {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.current.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_snapshot_list_all_children_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.snapshot.pack(out)? + self.need_results.pack(out)? + self.flags.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_snapshot_list_all_children_ret {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(xdr_codec::pack_flex(
            &self.snapshots,
            Some(REMOTE_DOMAIN_SNAPSHOT_LIST_MAX as usize),
            out,
        )? + self.ret.pack(out)?
            + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out>
    for remote_domain_snapshot_list_children_names_args
{
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.snap.pack(out)? + self.maxnames.pack(out)? + self.flags.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out>
    for remote_domain_snapshot_list_children_names_ret
{
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(xdr_codec::pack_flex(
            &self.names,
            Some(REMOTE_DOMAIN_SNAPSHOT_LIST_MAX as usize),
            out,
        )? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_snapshot_list_names_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.dom.pack(out)? + self.maxnames.pack(out)? + self.flags.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_snapshot_list_names_ret {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(xdr_codec::pack_flex(
            &self.names,
            Some(REMOTE_DOMAIN_SNAPSHOT_LIST_MAX as usize),
            out,
        )? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_snapshot_lookup_by_name_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.dom.pack(out)? + self.name.pack(out)? + self.flags.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_snapshot_lookup_by_name_ret {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.snap.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_snapshot_num_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.dom.pack(out)? + self.flags.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_snapshot_num_children_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.snap.pack(out)? + self.flags.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_snapshot_num_children_ret {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.num.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_snapshot_num_ret {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.num.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_stats_record {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.dom.pack(out)?
            + xdr_codec::pack_flex(
                &self.params,
                Some(REMOTE_CONNECT_GET_ALL_DOMAIN_STATS_MAX as usize),
                out,
            )?
            + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_suspend_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.dom.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_undefine_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.dom.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_undefine_flags_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.dom.pack(out)? + self.flags.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_domain_update_device_flags_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.dom.pack(out)? + self.xml.pack(out)? + self.flags.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_error {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.code.pack(out)?
            + self.domain.pack(out)?
            + self.message.pack(out)?
            + self.level.pack(out)?
            + self.dom.pack(out)?
            + self.str1.pack(out)?
            + self.str2.pack(out)?
            + self.str3.pack(out)?
            + self.int1.pack(out)?
            + self.int2.pack(out)?
            + self.net.pack(out)?
            + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_interface_change_begin_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.flags.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_interface_change_commit_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.flags.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_interface_change_rollback_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.flags.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_interface_create_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.iface.pack(out)? + self.flags.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_interface_define_xml_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.xml.pack(out)? + self.flags.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_interface_define_xml_ret {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.iface.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_interface_destroy_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.iface.pack(out)? + self.flags.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_interface_get_xml_desc_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.iface.pack(out)? + self.flags.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_interface_get_xml_desc_ret {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.xml.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_interface_is_active_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.iface.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_interface_is_active_ret {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.active.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_interface_lookup_by_mac_string_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.mac.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_interface_lookup_by_mac_string_ret {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.iface.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_interface_lookup_by_name_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.name.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_interface_lookup_by_name_ret {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.iface.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_interface_undefine_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.iface.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_network_create_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.net.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_network_create_xml_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.xml.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_network_create_xml_ret {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.net.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_network_define_xml_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.xml.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_network_define_xml_ret {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.net.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_network_destroy_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.net.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_network_dhcp_lease {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.iface.pack(out)?
            + self.expirytime.pack(out)?
            + self.type_.pack(out)?
            + self.mac.pack(out)?
            + self.iaid.pack(out)?
            + self.ipaddr.pack(out)?
            + self.prefix.pack(out)?
            + self.hostname.pack(out)?
            + self.clientid.pack(out)?
            + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_network_event_lifecycle_msg {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.callbackID.pack(out)?
            + self.net.pack(out)?
            + self.event.pack(out)?
            + self.detail.pack(out)?
            + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_network_get_autostart_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.net.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_network_get_autostart_ret {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.autostart.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_network_get_bridge_name_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.net.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_network_get_bridge_name_ret {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.name.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_network_get_dhcp_leases_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.net.pack(out)?
            + self.mac.pack(out)?
            + self.need_results.pack(out)?
            + self.flags.pack(out)?
            + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_network_get_dhcp_leases_ret {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(xdr_codec::pack_flex(
            &self.leases,
            Some(REMOTE_NETWORK_DHCP_LEASES_MAX as usize),
            out,
        )? + self.ret.pack(out)?
            + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_network_get_xml_desc_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.net.pack(out)? + self.flags.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_network_get_xml_desc_ret {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.xml.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_network_is_active_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.net.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_network_is_active_ret {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.active.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_network_is_persistent_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.net.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_network_is_persistent_ret {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.persistent.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_network_lookup_by_name_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.name.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_network_lookup_by_name_ret {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.net.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_network_lookup_by_uuid_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.uuid.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_network_lookup_by_uuid_ret {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.net.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_network_set_autostart_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.net.pack(out)? + self.autostart.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_network_undefine_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.net.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_network_update_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.net.pack(out)?
            + self.command.pack(out)?
            + self.section.pack(out)?
            + self.parentIndex.pack(out)?
            + self.xml.pack(out)?
            + self.flags.pack(out)?
            + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_node_alloc_pages_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(
            xdr_codec::pack_flex(&self.pageSizes, Some(REMOTE_NODE_MAX_CELLS as usize), out)?
                + xdr_codec::pack_flex(
                    &self.pageCounts,
                    Some(REMOTE_NODE_MAX_CELLS as usize),
                    out,
                )?
                + self.startCell.pack(out)?
                + self.cellCount.pack(out)?
                + self.flags.pack(out)?
                + 0,
        )
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_node_alloc_pages_ret {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.ret.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_node_device_create_xml_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.xml_desc.pack(out)? + self.flags.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_node_device_create_xml_ret {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.dev.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_node_device_destroy_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.name.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_node_device_detach_flags_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.name.pack(out)? + self.driverName.pack(out)? + self.flags.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_node_device_dettach_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.name.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_node_device_event_lifecycle_msg {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.callbackID.pack(out)?
            + self.dev.pack(out)?
            + self.event.pack(out)?
            + self.detail.pack(out)?
            + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_node_device_event_update_msg {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.callbackID.pack(out)? + self.dev.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_node_device_get_parent_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.name.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_node_device_get_parent_ret {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.parent.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_node_device_get_xml_desc_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.name.pack(out)? + self.flags.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_node_device_get_xml_desc_ret {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.xml.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_node_device_list_caps_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.name.pack(out)? + self.maxnames.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_node_device_list_caps_ret {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(xdr_codec::pack_flex(
            &self.names,
            Some(REMOTE_NODE_DEVICE_CAPS_LIST_MAX as usize),
            out,
        )? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_node_device_lookup_by_name_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.name.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_node_device_lookup_by_name_ret {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.dev.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out>
    for remote_node_device_lookup_scsi_host_by_wwn_args
{
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.wwnn.pack(out)? + self.wwpn.pack(out)? + self.flags.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out>
    for remote_node_device_lookup_scsi_host_by_wwn_ret
{
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.dev.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_node_device_num_of_caps_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.name.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_node_device_num_of_caps_ret {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.num.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_node_device_re_attach_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.name.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_node_device_reset_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.name.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_node_get_cells_free_memory_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.startCell.pack(out)? + self.maxcells.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_node_get_cells_free_memory_ret {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(xdr_codec::pack_flex(&self.cells, Some(REMOTE_NODE_MAX_CELLS as usize), out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_node_get_cpu_map_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.need_map.pack(out)? + self.need_online.pack(out)? + self.flags.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_node_get_cpu_map_ret {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(
            xdr_codec::pack_opaque_flex(&self.cpumap, Some(REMOTE_CPUMAP_MAX as usize), out)?
                + self.online.pack(out)?
                + self.ret.pack(out)?
                + 0,
        )
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_node_get_cpu_stats {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.field.pack(out)? + self.value.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_node_get_cpu_stats_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.cpuNum.pack(out)? + self.nparams.pack(out)? + self.flags.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_node_get_cpu_stats_ret {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(
            xdr_codec::pack_flex(&self.params, Some(REMOTE_NODE_CPU_STATS_MAX as usize), out)?
                + self.nparams.pack(out)?
                + 0,
        )
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_node_get_free_memory_ret {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.freeMem.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_node_get_free_pages_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(
            xdr_codec::pack_flex(&self.pages, Some(REMOTE_NODE_MAX_CELLS as usize), out)?
                + self.startCell.pack(out)?
                + self.cellCount.pack(out)?
                + self.flags.pack(out)?
                + 0,
        )
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_node_get_free_pages_ret {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(xdr_codec::pack_flex(&self.counts, Some(REMOTE_NODE_MAX_CELLS as usize), out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_node_get_info_ret {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(
            xdr_codec::pack_array(&self.model[..], self.model.len(), out, None)?
                + self.memory.pack(out)?
                + self.cpus.pack(out)?
                + self.mhz.pack(out)?
                + self.nodes.pack(out)?
                + self.sockets.pack(out)?
                + self.cores.pack(out)?
                + self.threads.pack(out)?
                + 0,
        )
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_node_get_memory_parameters_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.nparams.pack(out)? + self.flags.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_node_get_memory_parameters_ret {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(xdr_codec::pack_flex(
            &self.params,
            Some(REMOTE_NODE_MEMORY_PARAMETERS_MAX as usize),
            out,
        )? + self.nparams.pack(out)?
            + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_node_get_memory_stats {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.field.pack(out)? + self.value.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_node_get_memory_stats_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.nparams.pack(out)? + self.cellNum.pack(out)? + self.flags.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_node_get_memory_stats_ret {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(xdr_codec::pack_flex(
            &self.params,
            Some(REMOTE_NODE_MEMORY_STATS_MAX as usize),
            out,
        )? + self.nparams.pack(out)?
            + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_node_get_security_model_ret {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(
            xdr_codec::pack_flex(&self.model, Some(REMOTE_SECURITY_MODEL_MAX as usize), out)?
                + xdr_codec::pack_flex(&self.doi, Some(REMOTE_SECURITY_DOI_MAX as usize), out)?
                + 0,
        )
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_node_list_devices_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.cap.pack(out)? + self.maxnames.pack(out)? + self.flags.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_node_list_devices_ret {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(xdr_codec::pack_flex(&self.names, Some(REMOTE_NODE_DEVICE_LIST_MAX as usize), out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_node_num_of_devices_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.cap.pack(out)? + self.flags.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_node_num_of_devices_ret {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.num.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_node_set_memory_parameters_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(xdr_codec::pack_flex(
            &self.params,
            Some(REMOTE_NODE_MEMORY_PARAMETERS_MAX as usize),
            out,
        )? + self.flags.pack(out)?
            + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_node_suspend_for_duration_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.target.pack(out)? + self.duration.pack(out)? + self.flags.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_nonnull_domain {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.name.pack(out)? + self.uuid.pack(out)? + self.id.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_nonnull_domain_snapshot {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.name.pack(out)? + self.dom.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_nonnull_interface {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.name.pack(out)? + self.mac.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_nonnull_network {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.name.pack(out)? + self.uuid.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_nonnull_node_device {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.name.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_nonnull_nwfilter {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.name.pack(out)? + self.uuid.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_nonnull_secret {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.uuid.pack(out)? + self.usageType.pack(out)? + self.usageID.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_nonnull_storage_pool {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.name.pack(out)? + self.uuid.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_nonnull_storage_vol {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.pool.pack(out)? + self.name.pack(out)? + self.key.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_nonnull_string {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(xdr_codec::pack_string(
            &self.0,
            Some(REMOTE_STRING_MAX as usize),
            out,
        )?)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_nwfilter_define_xml_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.xml.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_nwfilter_define_xml_ret {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.nwfilter.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_nwfilter_get_xml_desc_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.nwfilter.pack(out)? + self.flags.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_nwfilter_get_xml_desc_ret {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.xml.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_nwfilter_lookup_by_name_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.name.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_nwfilter_lookup_by_name_ret {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.nwfilter.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_nwfilter_lookup_by_uuid_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.uuid.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_nwfilter_lookup_by_uuid_ret {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.nwfilter.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_nwfilter_undefine_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.nwfilter.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_procedure {
    #[inline]
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok((*self as i32).pack(out)?)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_secret_define_xml_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.xml.pack(out)? + self.flags.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_secret_define_xml_ret {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.secret.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_secret_event_lifecycle_msg {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.callbackID.pack(out)?
            + self.secret.pack(out)?
            + self.event.pack(out)?
            + self.detail.pack(out)?
            + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_secret_event_value_changed_msg {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.callbackID.pack(out)? + self.secret.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_secret_get_value_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.secret.pack(out)? + self.flags.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_secret_get_value_ret {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(
            xdr_codec::pack_opaque_flex(&self.value, Some(REMOTE_SECRET_VALUE_MAX as usize), out)?
                + 0,
        )
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_secret_get_xml_desc_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.secret.pack(out)? + self.flags.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_secret_get_xml_desc_ret {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.xml.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_secret_lookup_by_usage_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.usageType.pack(out)? + self.usageID.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_secret_lookup_by_usage_ret {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.secret.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_secret_lookup_by_uuid_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.uuid.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_secret_lookup_by_uuid_ret {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.secret.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_secret_set_value_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.secret.pack(out)?
            + xdr_codec::pack_opaque_flex(
                &self.value,
                Some(REMOTE_SECRET_VALUE_MAX as usize),
                out,
            )?
            + self.flags.pack(out)?
            + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_secret_undefine_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.secret.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_storage_pool_build_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.pool.pack(out)? + self.flags.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_storage_pool_create_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.pool.pack(out)? + self.flags.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_storage_pool_create_xml_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.xml.pack(out)? + self.flags.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_storage_pool_create_xml_ret {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.pool.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_storage_pool_define_xml_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.xml.pack(out)? + self.flags.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_storage_pool_define_xml_ret {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.pool.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_storage_pool_delete_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.pool.pack(out)? + self.flags.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_storage_pool_destroy_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.pool.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_storage_pool_event_lifecycle_msg {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.callbackID.pack(out)?
            + self.pool.pack(out)?
            + self.event.pack(out)?
            + self.detail.pack(out)?
            + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_storage_pool_event_refresh_msg {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.callbackID.pack(out)? + self.pool.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_storage_pool_get_autostart_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.pool.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_storage_pool_get_autostart_ret {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.autostart.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_storage_pool_get_info_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.pool.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_storage_pool_get_info_ret {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.state.pack(out)?
            + self.capacity.pack(out)?
            + self.allocation.pack(out)?
            + self.available.pack(out)?
            + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_storage_pool_get_xml_desc_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.pool.pack(out)? + self.flags.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_storage_pool_get_xml_desc_ret {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.xml.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_storage_pool_is_active_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.pool.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_storage_pool_is_active_ret {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.active.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_storage_pool_is_persistent_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.pool.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_storage_pool_is_persistent_ret {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.persistent.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_storage_pool_list_all_volumes_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.pool.pack(out)? + self.need_results.pack(out)? + self.flags.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_storage_pool_list_all_volumes_ret {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(
            xdr_codec::pack_flex(&self.vols, Some(REMOTE_STORAGE_VOL_LIST_MAX as usize), out)?
                + self.ret.pack(out)?
                + 0,
        )
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_storage_pool_list_volumes_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.pool.pack(out)? + self.maxnames.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_storage_pool_list_volumes_ret {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(xdr_codec::pack_flex(&self.names, Some(REMOTE_STORAGE_VOL_LIST_MAX as usize), out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_storage_pool_lookup_by_name_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.name.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_storage_pool_lookup_by_name_ret {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.pool.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_storage_pool_lookup_by_uuid_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.uuid.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_storage_pool_lookup_by_uuid_ret {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.pool.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_storage_pool_lookup_by_volume_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.vol.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_storage_pool_lookup_by_volume_ret {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.pool.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_storage_pool_num_of_volumes_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.pool.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_storage_pool_num_of_volumes_ret {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.num.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_storage_pool_refresh_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.pool.pack(out)? + self.flags.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_storage_pool_set_autostart_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.pool.pack(out)? + self.autostart.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_storage_pool_undefine_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.pool.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_storage_vol_create_xml_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.pool.pack(out)? + self.xml.pack(out)? + self.flags.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_storage_vol_create_xml_from_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.pool.pack(out)?
            + self.xml.pack(out)?
            + self.clonevol.pack(out)?
            + self.flags.pack(out)?
            + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_storage_vol_create_xml_from_ret {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.vol.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_storage_vol_create_xml_ret {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.vol.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_storage_vol_delete_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.vol.pack(out)? + self.flags.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_storage_vol_download_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.vol.pack(out)?
            + self.offset.pack(out)?
            + self.length.pack(out)?
            + self.flags.pack(out)?
            + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_storage_vol_get_info_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.vol.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_storage_vol_get_info_flags_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.vol.pack(out)? + self.flags.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_storage_vol_get_info_flags_ret {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.type_.pack(out)? + self.capacity.pack(out)? + self.allocation.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_storage_vol_get_info_ret {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.type_.pack(out)? + self.capacity.pack(out)? + self.allocation.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_storage_vol_get_path_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.vol.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_storage_vol_get_path_ret {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.name.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_storage_vol_get_xml_desc_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.vol.pack(out)? + self.flags.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_storage_vol_get_xml_desc_ret {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.xml.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_storage_vol_lookup_by_key_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.key.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_storage_vol_lookup_by_key_ret {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.vol.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_storage_vol_lookup_by_name_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.pool.pack(out)? + self.name.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_storage_vol_lookup_by_name_ret {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.vol.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_storage_vol_lookup_by_path_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.path.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_storage_vol_lookup_by_path_ret {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.vol.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_storage_vol_resize_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.vol.pack(out)? + self.capacity.pack(out)? + self.flags.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_storage_vol_upload_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.vol.pack(out)?
            + self.offset.pack(out)?
            + self.length.pack(out)?
            + self.flags.pack(out)?
            + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_storage_vol_wipe_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.vol.pack(out)? + self.flags.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_storage_vol_wipe_pattern_args {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.vol.pack(out)? + self.algorithm.pack(out)? + self.flags.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_typed_param {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.field.pack(out)? + self.value.pack(out)? + 0)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_typed_param_value {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(match self {
            &remote_typed_param_value::Const1(ref val) => {
                (1i64 as i32).pack(out)? + val.pack(out)?
            }
            &remote_typed_param_value::Const2(ref val) => {
                (2i64 as i32).pack(out)? + val.pack(out)?
            }
            &remote_typed_param_value::Const3(ref val) => {
                (3i64 as i32).pack(out)? + val.pack(out)?
            }
            &remote_typed_param_value::Const4(ref val) => {
                (4i64 as i32).pack(out)? + val.pack(out)?
            }
            &remote_typed_param_value::Const5(ref val) => {
                (5i64 as i32).pack(out)? + val.pack(out)?
            }
            &remote_typed_param_value::Const6(ref val) => {
                (6i64 as i32).pack(out)? + val.pack(out)?
            }
            &remote_typed_param_value::Const7(ref val) => {
                (7i64 as i32).pack(out)? + val.pack(out)?
            }
        })
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_uuid {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(xdr_codec::pack_opaque_array(
            &self.0[..],
            self.0.len(),
            out,
        )?)
    }
}

impl<Out: xdr_codec::Write> xdr_codec::Pack<Out> for remote_vcpu_info {
    fn pack(&self, out: &mut Out) -> xdr_codec::Result<usize> {
        Ok(self.number.pack(out)?
            + self.state.pack(out)?
            + self.cpu_time.pack(out)?
            + self.cpu.pack(out)?
            + 0)
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_auth_list_ret {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_auth_list_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_auth_list_ret {
                types: {
                    let (v, fsz) =
                        xdr_codec::unpack_flex(input, Some(REMOTE_AUTH_TYPE_LIST_MAX as usize))?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_auth_polkit_ret {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_auth_polkit_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_auth_polkit_ret {
                complete: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_auth_sasl_init_ret {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_auth_sasl_init_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_auth_sasl_init_ret {
                mechlist: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_auth_sasl_start_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_auth_sasl_start_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_auth_sasl_start_args {
                mech: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                nil: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                data: {
                    let (v, fsz) =
                        xdr_codec::unpack_flex(input, Some(REMOTE_AUTH_SASL_DATA_MAX as usize))?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_auth_sasl_start_ret {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_auth_sasl_start_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_auth_sasl_start_ret {
                complete: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                nil: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                data: {
                    let (v, fsz) =
                        xdr_codec::unpack_flex(input, Some(REMOTE_AUTH_SASL_DATA_MAX as usize))?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_auth_sasl_step_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_auth_sasl_step_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_auth_sasl_step_args {
                nil: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                data: {
                    let (v, fsz) =
                        xdr_codec::unpack_flex(input, Some(REMOTE_AUTH_SASL_DATA_MAX as usize))?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_auth_sasl_step_ret {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_auth_sasl_step_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_auth_sasl_step_ret {
                complete: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                nil: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                data: {
                    let (v, fsz) =
                        xdr_codec::unpack_flex(input, Some(REMOTE_AUTH_SASL_DATA_MAX as usize))?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_auth_type {
    #[inline]
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_auth_type, usize)> {
        let mut sz = 0;
        Ok((
            {
                let (e, esz): (i32, _) = xdr_codec::Unpack::unpack(input)?;
                sz += esz;
                match e {
                    x if x == remote_auth_type::REMOTE_AUTH_NONE as i32 => {
                        remote_auth_type::REMOTE_AUTH_NONE
                    }
                    x if x == remote_auth_type::REMOTE_AUTH_SASL as i32 => {
                        remote_auth_type::REMOTE_AUTH_SASL
                    }
                    x if x == remote_auth_type::REMOTE_AUTH_POLKIT as i32 => {
                        remote_auth_type::REMOTE_AUTH_POLKIT
                    }
                    e => return Err(xdr_codec::Error::invalidenum(e)),
                }
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_connect_baseline_cpu_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_connect_baseline_cpu_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_connect_baseline_cpu_args {
                xmlCPUs: {
                    let (v, fsz) =
                        xdr_codec::unpack_flex(input, Some(REMOTE_CPU_BASELINE_MAX as usize))?;
                    sz += fsz;
                    v
                },
                flags: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_connect_baseline_cpu_ret {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_connect_baseline_cpu_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_connect_baseline_cpu_ret {
                cpu: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_connect_compare_cpu_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_connect_compare_cpu_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_connect_compare_cpu_args {
                xml: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                flags: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_connect_compare_cpu_ret {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_connect_compare_cpu_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_connect_compare_cpu_ret {
                result: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In>
    for remote_connect_domain_event_callback_deregister_any_args
{
    fn unpack(
        input: &mut In,
    ) -> xdr_codec::Result<(
        remote_connect_domain_event_callback_deregister_any_args,
        usize,
    )> {
        let mut sz = 0;
        Ok((
            remote_connect_domain_event_callback_deregister_any_args {
                callbackID: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In>
    for remote_connect_domain_event_callback_register_any_args
{
    fn unpack(
        input: &mut In,
    ) -> xdr_codec::Result<(
        remote_connect_domain_event_callback_register_any_args,
        usize,
    )> {
        let mut sz = 0;
        Ok((
            remote_connect_domain_event_callback_register_any_args {
                eventID: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                dom: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In>
    for remote_connect_domain_event_callback_register_any_ret
{
    fn unpack(
        input: &mut In,
    ) -> xdr_codec::Result<(remote_connect_domain_event_callback_register_any_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_connect_domain_event_callback_register_any_ret {
                callbackID: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In>
    for remote_connect_domain_event_deregister_any_args
{
    fn unpack(
        input: &mut In,
    ) -> xdr_codec::Result<(remote_connect_domain_event_deregister_any_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_connect_domain_event_deregister_any_args {
                eventID: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_connect_domain_event_deregister_ret {
    fn unpack(
        input: &mut In,
    ) -> xdr_codec::Result<(remote_connect_domain_event_deregister_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_connect_domain_event_deregister_ret {
                cb_registered: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_connect_domain_event_register_any_args {
    fn unpack(
        input: &mut In,
    ) -> xdr_codec::Result<(remote_connect_domain_event_register_any_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_connect_domain_event_register_any_args {
                eventID: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_connect_domain_event_register_ret {
    fn unpack(
        input: &mut In,
    ) -> xdr_codec::Result<(remote_connect_domain_event_register_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_connect_domain_event_register_ret {
                cb_registered: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_connect_domain_xml_from_native_args {
    fn unpack(
        input: &mut In,
    ) -> xdr_codec::Result<(remote_connect_domain_xml_from_native_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_connect_domain_xml_from_native_args {
                nativeFormat: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                nativeConfig: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                flags: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_connect_domain_xml_from_native_ret {
    fn unpack(
        input: &mut In,
    ) -> xdr_codec::Result<(remote_connect_domain_xml_from_native_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_connect_domain_xml_from_native_ret {
                domainXml: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_connect_domain_xml_to_native_args {
    fn unpack(
        input: &mut In,
    ) -> xdr_codec::Result<(remote_connect_domain_xml_to_native_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_connect_domain_xml_to_native_args {
                nativeFormat: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                domainXml: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                flags: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_connect_domain_xml_to_native_ret {
    fn unpack(
        input: &mut In,
    ) -> xdr_codec::Result<(remote_connect_domain_xml_to_native_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_connect_domain_xml_to_native_ret {
                nativeConfig: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_connect_event_connection_closed_msg {
    fn unpack(
        input: &mut In,
    ) -> xdr_codec::Result<(remote_connect_event_connection_closed_msg, usize)> {
        let mut sz = 0;
        Ok((
            remote_connect_event_connection_closed_msg {
                reason: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_connect_find_storage_pool_sources_args {
    fn unpack(
        input: &mut In,
    ) -> xdr_codec::Result<(remote_connect_find_storage_pool_sources_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_connect_find_storage_pool_sources_args {
                type_: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                srcSpec: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                flags: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_connect_find_storage_pool_sources_ret {
    fn unpack(
        input: &mut In,
    ) -> xdr_codec::Result<(remote_connect_find_storage_pool_sources_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_connect_find_storage_pool_sources_ret {
                xml: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_connect_get_all_domain_stats_args {
    fn unpack(
        input: &mut In,
    ) -> xdr_codec::Result<(remote_connect_get_all_domain_stats_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_connect_get_all_domain_stats_args {
                doms: {
                    let (v, fsz) =
                        xdr_codec::unpack_flex(input, Some(REMOTE_DOMAIN_LIST_MAX as usize))?;
                    sz += fsz;
                    v
                },
                stats: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                flags: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_connect_get_all_domain_stats_ret {
    fn unpack(
        input: &mut In,
    ) -> xdr_codec::Result<(remote_connect_get_all_domain_stats_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_connect_get_all_domain_stats_ret {
                retStats: {
                    let (v, fsz) =
                        xdr_codec::unpack_flex(input, Some(REMOTE_DOMAIN_LIST_MAX as usize))?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_connect_get_capabilities_ret {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_connect_get_capabilities_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_connect_get_capabilities_ret {
                capabilities: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_connect_get_cpu_model_names_args {
    fn unpack(
        input: &mut In,
    ) -> xdr_codec::Result<(remote_connect_get_cpu_model_names_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_connect_get_cpu_model_names_args {
                arch: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                need_results: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                flags: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_connect_get_cpu_model_names_ret {
    fn unpack(
        input: &mut In,
    ) -> xdr_codec::Result<(remote_connect_get_cpu_model_names_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_connect_get_cpu_model_names_ret {
                models: {
                    let (v, fsz) = xdr_codec::unpack_flex(
                        input,
                        Some(REMOTE_CONNECT_CPU_MODELS_MAX as usize),
                    )?;
                    sz += fsz;
                    v
                },
                ret: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_connect_get_domain_capabilities_args {
    fn unpack(
        input: &mut In,
    ) -> xdr_codec::Result<(remote_connect_get_domain_capabilities_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_connect_get_domain_capabilities_args {
                emulatorbin: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                arch: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                machine: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                virttype: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                flags: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_connect_get_domain_capabilities_ret {
    fn unpack(
        input: &mut In,
    ) -> xdr_codec::Result<(remote_connect_get_domain_capabilities_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_connect_get_domain_capabilities_ret {
                capabilities: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_connect_get_hostname_ret {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_connect_get_hostname_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_connect_get_hostname_ret {
                hostname: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_connect_get_lib_version_ret {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_connect_get_lib_version_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_connect_get_lib_version_ret {
                lib_ver: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_connect_get_max_vcpus_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_connect_get_max_vcpus_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_connect_get_max_vcpus_args {
                type_: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_connect_get_max_vcpus_ret {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_connect_get_max_vcpus_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_connect_get_max_vcpus_ret {
                max_vcpus: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_connect_get_sysinfo_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_connect_get_sysinfo_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_connect_get_sysinfo_args {
                flags: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_connect_get_sysinfo_ret {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_connect_get_sysinfo_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_connect_get_sysinfo_ret {
                sysinfo: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_connect_get_type_ret {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_connect_get_type_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_connect_get_type_ret {
                type_: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_connect_get_uri_ret {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_connect_get_uri_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_connect_get_uri_ret {
                uri: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_connect_get_version_ret {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_connect_get_version_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_connect_get_version_ret {
                hv_ver: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_connect_is_secure_ret {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_connect_is_secure_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_connect_is_secure_ret {
                secure: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_connect_list_all_domains_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_connect_list_all_domains_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_connect_list_all_domains_args {
                need_results: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                flags: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_connect_list_all_domains_ret {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_connect_list_all_domains_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_connect_list_all_domains_ret {
                domains: {
                    let (v, fsz) =
                        xdr_codec::unpack_flex(input, Some(REMOTE_DOMAIN_LIST_MAX as usize))?;
                    sz += fsz;
                    v
                },
                ret: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_connect_list_all_interfaces_args {
    fn unpack(
        input: &mut In,
    ) -> xdr_codec::Result<(remote_connect_list_all_interfaces_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_connect_list_all_interfaces_args {
                need_results: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                flags: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_connect_list_all_interfaces_ret {
    fn unpack(
        input: &mut In,
    ) -> xdr_codec::Result<(remote_connect_list_all_interfaces_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_connect_list_all_interfaces_ret {
                ifaces: {
                    let (v, fsz) =
                        xdr_codec::unpack_flex(input, Some(REMOTE_INTERFACE_LIST_MAX as usize))?;
                    sz += fsz;
                    v
                },
                ret: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_connect_list_all_networks_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_connect_list_all_networks_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_connect_list_all_networks_args {
                need_results: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                flags: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_connect_list_all_networks_ret {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_connect_list_all_networks_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_connect_list_all_networks_ret {
                nets: {
                    let (v, fsz) =
                        xdr_codec::unpack_flex(input, Some(REMOTE_NETWORK_LIST_MAX as usize))?;
                    sz += fsz;
                    v
                },
                ret: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_connect_list_all_node_devices_args {
    fn unpack(
        input: &mut In,
    ) -> xdr_codec::Result<(remote_connect_list_all_node_devices_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_connect_list_all_node_devices_args {
                need_results: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                flags: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_connect_list_all_node_devices_ret {
    fn unpack(
        input: &mut In,
    ) -> xdr_codec::Result<(remote_connect_list_all_node_devices_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_connect_list_all_node_devices_ret {
                devices: {
                    let (v, fsz) =
                        xdr_codec::unpack_flex(input, Some(REMOTE_NODE_DEVICE_LIST_MAX as usize))?;
                    sz += fsz;
                    v
                },
                ret: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_connect_list_all_nwfilters_args {
    fn unpack(
        input: &mut In,
    ) -> xdr_codec::Result<(remote_connect_list_all_nwfilters_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_connect_list_all_nwfilters_args {
                need_results: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                flags: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_connect_list_all_nwfilters_ret {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_connect_list_all_nwfilters_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_connect_list_all_nwfilters_ret {
                filters: {
                    let (v, fsz) =
                        xdr_codec::unpack_flex(input, Some(REMOTE_NWFILTER_LIST_MAX as usize))?;
                    sz += fsz;
                    v
                },
                ret: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_connect_list_all_secrets_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_connect_list_all_secrets_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_connect_list_all_secrets_args {
                need_results: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                flags: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_connect_list_all_secrets_ret {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_connect_list_all_secrets_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_connect_list_all_secrets_ret {
                secrets: {
                    let (v, fsz) =
                        xdr_codec::unpack_flex(input, Some(REMOTE_SECRET_LIST_MAX as usize))?;
                    sz += fsz;
                    v
                },
                ret: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_connect_list_all_storage_pools_args {
    fn unpack(
        input: &mut In,
    ) -> xdr_codec::Result<(remote_connect_list_all_storage_pools_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_connect_list_all_storage_pools_args {
                need_results: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                flags: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_connect_list_all_storage_pools_ret {
    fn unpack(
        input: &mut In,
    ) -> xdr_codec::Result<(remote_connect_list_all_storage_pools_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_connect_list_all_storage_pools_ret {
                pools: {
                    let (v, fsz) =
                        xdr_codec::unpack_flex(input, Some(REMOTE_STORAGE_POOL_LIST_MAX as usize))?;
                    sz += fsz;
                    v
                },
                ret: {
                    // Libvirt might not return this field
                    let (v, fsz) = xdr_codec::Unpack::unpack(input).unwrap_or_default();
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_connect_list_defined_domains_args {
    fn unpack(
        input: &mut In,
    ) -> xdr_codec::Result<(remote_connect_list_defined_domains_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_connect_list_defined_domains_args {
                maxnames: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_connect_list_defined_domains_ret {
    fn unpack(
        input: &mut In,
    ) -> xdr_codec::Result<(remote_connect_list_defined_domains_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_connect_list_defined_domains_ret {
                names: {
                    let (v, fsz) =
                        xdr_codec::unpack_flex(input, Some(REMOTE_DOMAIN_LIST_MAX as usize))?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_connect_list_defined_interfaces_args {
    fn unpack(
        input: &mut In,
    ) -> xdr_codec::Result<(remote_connect_list_defined_interfaces_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_connect_list_defined_interfaces_args {
                maxnames: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_connect_list_defined_interfaces_ret {
    fn unpack(
        input: &mut In,
    ) -> xdr_codec::Result<(remote_connect_list_defined_interfaces_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_connect_list_defined_interfaces_ret {
                names: {
                    let (v, fsz) =
                        xdr_codec::unpack_flex(input, Some(REMOTE_INTERFACE_LIST_MAX as usize))?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_connect_list_defined_networks_args {
    fn unpack(
        input: &mut In,
    ) -> xdr_codec::Result<(remote_connect_list_defined_networks_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_connect_list_defined_networks_args {
                maxnames: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_connect_list_defined_networks_ret {
    fn unpack(
        input: &mut In,
    ) -> xdr_codec::Result<(remote_connect_list_defined_networks_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_connect_list_defined_networks_ret {
                names: {
                    let (v, fsz) =
                        xdr_codec::unpack_flex(input, Some(REMOTE_NETWORK_LIST_MAX as usize))?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_connect_list_defined_storage_pools_args {
    fn unpack(
        input: &mut In,
    ) -> xdr_codec::Result<(remote_connect_list_defined_storage_pools_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_connect_list_defined_storage_pools_args {
                maxnames: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_connect_list_defined_storage_pools_ret {
    fn unpack(
        input: &mut In,
    ) -> xdr_codec::Result<(remote_connect_list_defined_storage_pools_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_connect_list_defined_storage_pools_ret {
                names: {
                    let (v, fsz) =
                        xdr_codec::unpack_flex(input, Some(REMOTE_STORAGE_POOL_LIST_MAX as usize))?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_connect_list_domains_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_connect_list_domains_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_connect_list_domains_args {
                maxids: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_connect_list_domains_ret {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_connect_list_domains_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_connect_list_domains_ret {
                ids: {
                    let (v, fsz) =
                        xdr_codec::unpack_flex(input, Some(REMOTE_DOMAIN_LIST_MAX as usize))?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_connect_list_interfaces_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_connect_list_interfaces_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_connect_list_interfaces_args {
                maxnames: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_connect_list_interfaces_ret {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_connect_list_interfaces_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_connect_list_interfaces_ret {
                names: {
                    let (v, fsz) =
                        xdr_codec::unpack_flex(input, Some(REMOTE_INTERFACE_LIST_MAX as usize))?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_connect_list_networks_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_connect_list_networks_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_connect_list_networks_args {
                maxnames: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_connect_list_networks_ret {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_connect_list_networks_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_connect_list_networks_ret {
                names: {
                    let (v, fsz) =
                        xdr_codec::unpack_flex(input, Some(REMOTE_NETWORK_LIST_MAX as usize))?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_connect_list_nwfilters_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_connect_list_nwfilters_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_connect_list_nwfilters_args {
                maxnames: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_connect_list_nwfilters_ret {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_connect_list_nwfilters_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_connect_list_nwfilters_ret {
                names: {
                    let (v, fsz) =
                        xdr_codec::unpack_flex(input, Some(REMOTE_NWFILTER_LIST_MAX as usize))?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_connect_list_secrets_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_connect_list_secrets_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_connect_list_secrets_args {
                maxuuids: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_connect_list_secrets_ret {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_connect_list_secrets_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_connect_list_secrets_ret {
                uuids: {
                    let (v, fsz) =
                        xdr_codec::unpack_flex(input, Some(REMOTE_SECRET_LIST_MAX as usize))?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_connect_list_storage_pools_args {
    fn unpack(
        input: &mut In,
    ) -> xdr_codec::Result<(remote_connect_list_storage_pools_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_connect_list_storage_pools_args {
                maxnames: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_connect_list_storage_pools_ret {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_connect_list_storage_pools_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_connect_list_storage_pools_ret {
                names: {
                    let (v, fsz) =
                        xdr_codec::unpack_flex(input, Some(REMOTE_STORAGE_POOL_LIST_MAX as usize))?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In>
    for remote_connect_network_event_deregister_any_args
{
    fn unpack(
        input: &mut In,
    ) -> xdr_codec::Result<(remote_connect_network_event_deregister_any_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_connect_network_event_deregister_any_args {
                callbackID: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_connect_network_event_register_any_args {
    fn unpack(
        input: &mut In,
    ) -> xdr_codec::Result<(remote_connect_network_event_register_any_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_connect_network_event_register_any_args {
                eventID: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                net: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_connect_network_event_register_any_ret {
    fn unpack(
        input: &mut In,
    ) -> xdr_codec::Result<(remote_connect_network_event_register_any_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_connect_network_event_register_any_ret {
                callbackID: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In>
    for remote_connect_node_device_event_deregister_any_args
{
    fn unpack(
        input: &mut In,
    ) -> xdr_codec::Result<(remote_connect_node_device_event_deregister_any_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_connect_node_device_event_deregister_any_args {
                callbackID: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In>
    for remote_connect_node_device_event_register_any_args
{
    fn unpack(
        input: &mut In,
    ) -> xdr_codec::Result<(remote_connect_node_device_event_register_any_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_connect_node_device_event_register_any_args {
                eventID: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                dev: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In>
    for remote_connect_node_device_event_register_any_ret
{
    fn unpack(
        input: &mut In,
    ) -> xdr_codec::Result<(remote_connect_node_device_event_register_any_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_connect_node_device_event_register_any_ret {
                callbackID: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_connect_num_of_defined_domains_ret {
    fn unpack(
        input: &mut In,
    ) -> xdr_codec::Result<(remote_connect_num_of_defined_domains_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_connect_num_of_defined_domains_ret {
                num: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_connect_num_of_defined_interfaces_ret {
    fn unpack(
        input: &mut In,
    ) -> xdr_codec::Result<(remote_connect_num_of_defined_interfaces_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_connect_num_of_defined_interfaces_ret {
                num: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_connect_num_of_defined_networks_ret {
    fn unpack(
        input: &mut In,
    ) -> xdr_codec::Result<(remote_connect_num_of_defined_networks_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_connect_num_of_defined_networks_ret {
                num: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In>
    for remote_connect_num_of_defined_storage_pools_ret
{
    fn unpack(
        input: &mut In,
    ) -> xdr_codec::Result<(remote_connect_num_of_defined_storage_pools_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_connect_num_of_defined_storage_pools_ret {
                num: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_connect_num_of_domains_ret {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_connect_num_of_domains_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_connect_num_of_domains_ret {
                num: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_connect_num_of_interfaces_ret {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_connect_num_of_interfaces_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_connect_num_of_interfaces_ret {
                num: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_connect_num_of_networks_ret {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_connect_num_of_networks_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_connect_num_of_networks_ret {
                num: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_connect_num_of_nwfilters_ret {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_connect_num_of_nwfilters_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_connect_num_of_nwfilters_ret {
                num: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_connect_num_of_secrets_ret {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_connect_num_of_secrets_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_connect_num_of_secrets_ret {
                num: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_connect_num_of_storage_pools_ret {
    fn unpack(
        input: &mut In,
    ) -> xdr_codec::Result<(remote_connect_num_of_storage_pools_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_connect_num_of_storage_pools_ret {
                num: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_connect_open_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_connect_open_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_connect_open_args {
                name: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                flags: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In>
    for remote_connect_secret_event_deregister_any_args
{
    fn unpack(
        input: &mut In,
    ) -> xdr_codec::Result<(remote_connect_secret_event_deregister_any_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_connect_secret_event_deregister_any_args {
                callbackID: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_connect_secret_event_register_any_args {
    fn unpack(
        input: &mut In,
    ) -> xdr_codec::Result<(remote_connect_secret_event_register_any_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_connect_secret_event_register_any_args {
                eventID: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                secret: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_connect_secret_event_register_any_ret {
    fn unpack(
        input: &mut In,
    ) -> xdr_codec::Result<(remote_connect_secret_event_register_any_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_connect_secret_event_register_any_ret {
                callbackID: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In>
    for remote_connect_storage_pool_event_deregister_any_args
{
    fn unpack(
        input: &mut In,
    ) -> xdr_codec::Result<(remote_connect_storage_pool_event_deregister_any_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_connect_storage_pool_event_deregister_any_args {
                callbackID: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In>
    for remote_connect_storage_pool_event_register_any_args
{
    fn unpack(
        input: &mut In,
    ) -> xdr_codec::Result<(remote_connect_storage_pool_event_register_any_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_connect_storage_pool_event_register_any_args {
                eventID: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                pool: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In>
    for remote_connect_storage_pool_event_register_any_ret
{
    fn unpack(
        input: &mut In,
    ) -> xdr_codec::Result<(remote_connect_storage_pool_event_register_any_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_connect_storage_pool_event_register_any_ret {
                callbackID: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_connect_supports_feature_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_connect_supports_feature_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_connect_supports_feature_args {
                feature: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_connect_supports_feature_ret {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_connect_supports_feature_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_connect_supports_feature_ret {
                supported: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_abort_job_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_domain_abort_job_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_abort_job_args {
                dom: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_add_iothread_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_domain_add_iothread_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_add_iothread_args {
                dom: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                iothread_id: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                flags: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_attach_device_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_domain_attach_device_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_attach_device_args {
                dom: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                xml: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_attach_device_flags_args {
    fn unpack(
        input: &mut In,
    ) -> xdr_codec::Result<(remote_domain_attach_device_flags_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_attach_device_flags_args {
                dom: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                xml: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                flags: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_block_commit_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_domain_block_commit_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_block_commit_args {
                dom: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                disk: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                base: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                top: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                bandwidth: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                flags: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_block_copy_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_domain_block_copy_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_block_copy_args {
                dom: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                path: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                destxml: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                params: {
                    let (v, fsz) = xdr_codec::unpack_flex(
                        input,
                        Some(REMOTE_DOMAIN_BLOCK_COPY_PARAMETERS_MAX as usize),
                    )?;
                    sz += fsz;
                    v
                },
                flags: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_block_job_abort_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_domain_block_job_abort_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_block_job_abort_args {
                dom: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                path: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                flags: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_block_job_set_speed_args {
    fn unpack(
        input: &mut In,
    ) -> xdr_codec::Result<(remote_domain_block_job_set_speed_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_block_job_set_speed_args {
                dom: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                path: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                bandwidth: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                flags: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_block_peek_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_domain_block_peek_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_block_peek_args {
                dom: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                path: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                offset: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                size: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                flags: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_block_peek_ret {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_domain_block_peek_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_block_peek_ret {
                buffer: {
                    let (v, fsz) = xdr_codec::unpack_opaque_flex(
                        input,
                        Some(REMOTE_DOMAIN_BLOCK_PEEK_BUFFER_MAX as usize),
                    )?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_block_pull_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_domain_block_pull_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_block_pull_args {
                dom: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                path: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                bandwidth: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                flags: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_block_rebase_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_domain_block_rebase_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_block_rebase_args {
                dom: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                path: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                base: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                bandwidth: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                flags: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_block_resize_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_domain_block_resize_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_block_resize_args {
                dom: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                disk: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                size: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                flags: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_block_stats_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_domain_block_stats_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_block_stats_args {
                dom: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                path: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_block_stats_flags_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_domain_block_stats_flags_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_block_stats_flags_args {
                dom: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                path: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                nparams: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                flags: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_block_stats_flags_ret {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_domain_block_stats_flags_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_block_stats_flags_ret {
                params: {
                    let (v, fsz) = xdr_codec::unpack_flex(
                        input,
                        Some(REMOTE_DOMAIN_BLOCK_STATS_PARAMETERS_MAX as usize),
                    )?;
                    sz += fsz;
                    v
                },
                nparams: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_block_stats_ret {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_domain_block_stats_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_block_stats_ret {
                rd_req: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                rd_bytes: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                wr_req: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                wr_bytes: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                errs: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_core_dump_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_domain_core_dump_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_core_dump_args {
                dom: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                to: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                flags: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_core_dump_with_format_args {
    fn unpack(
        input: &mut In,
    ) -> xdr_codec::Result<(remote_domain_core_dump_with_format_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_core_dump_with_format_args {
                dom: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                to: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                dumpformat: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                flags: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_create_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_domain_create_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_create_args {
                dom: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_create_with_files_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_domain_create_with_files_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_create_with_files_args {
                dom: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                flags: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_create_with_files_ret {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_domain_create_with_files_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_create_with_files_ret {
                dom: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_create_with_flags_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_domain_create_with_flags_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_create_with_flags_args {
                dom: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                flags: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_create_with_flags_ret {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_domain_create_with_flags_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_create_with_flags_ret {
                dom: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_create_xml_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_domain_create_xml_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_create_xml_args {
                xml_desc: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                flags: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_create_xml_ret {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_domain_create_xml_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_create_xml_ret {
                dom: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_create_xml_with_files_args {
    fn unpack(
        input: &mut In,
    ) -> xdr_codec::Result<(remote_domain_create_xml_with_files_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_create_xml_with_files_args {
                xml_desc: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                flags: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_create_xml_with_files_ret {
    fn unpack(
        input: &mut In,
    ) -> xdr_codec::Result<(remote_domain_create_xml_with_files_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_create_xml_with_files_ret {
                dom: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_define_xml_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_domain_define_xml_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_define_xml_args {
                xml: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_define_xml_flags_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_domain_define_xml_flags_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_define_xml_flags_args {
                xml: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                flags: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_define_xml_flags_ret {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_domain_define_xml_flags_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_define_xml_flags_ret {
                dom: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_define_xml_ret {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_domain_define_xml_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_define_xml_ret {
                dom: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_del_iothread_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_domain_del_iothread_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_del_iothread_args {
                dom: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                iothread_id: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                flags: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_destroy_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_domain_destroy_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_destroy_args {
                dom: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_destroy_flags_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_domain_destroy_flags_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_destroy_flags_args {
                dom: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                flags: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_detach_device_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_domain_detach_device_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_detach_device_args {
                dom: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                xml: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_detach_device_flags_args {
    fn unpack(
        input: &mut In,
    ) -> xdr_codec::Result<(remote_domain_detach_device_flags_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_detach_device_flags_args {
                dom: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                xml: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                flags: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_disk_error {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_domain_disk_error, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_disk_error {
                disk: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                error: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_event_balloon_change_msg {
    fn unpack(
        input: &mut In,
    ) -> xdr_codec::Result<(remote_domain_event_balloon_change_msg, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_event_balloon_change_msg {
                dom: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                actual: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_event_block_job_2_msg {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_domain_event_block_job_2_msg, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_event_block_job_2_msg {
                callbackID: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                dom: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                dst: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                type_: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                status: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_event_block_job_msg {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_domain_event_block_job_msg, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_event_block_job_msg {
                dom: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                path: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                type_: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                status: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_event_block_threshold_msg {
    fn unpack(
        input: &mut In,
    ) -> xdr_codec::Result<(remote_domain_event_block_threshold_msg, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_event_block_threshold_msg {
                callbackID: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                dom: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                dev: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                path: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                threshold: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                excess: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In>
    for remote_domain_event_callback_agent_lifecycle_msg
{
    fn unpack(
        input: &mut In,
    ) -> xdr_codec::Result<(remote_domain_event_callback_agent_lifecycle_msg, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_event_callback_agent_lifecycle_msg {
                callbackID: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                dom: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                state: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                reason: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In>
    for remote_domain_event_callback_balloon_change_msg
{
    fn unpack(
        input: &mut In,
    ) -> xdr_codec::Result<(remote_domain_event_callback_balloon_change_msg, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_event_callback_balloon_change_msg {
                callbackID: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                msg: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_event_callback_block_job_msg {
    fn unpack(
        input: &mut In,
    ) -> xdr_codec::Result<(remote_domain_event_callback_block_job_msg, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_event_callback_block_job_msg {
                callbackID: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                msg: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_event_callback_control_error_msg {
    fn unpack(
        input: &mut In,
    ) -> xdr_codec::Result<(remote_domain_event_callback_control_error_msg, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_event_callback_control_error_msg {
                callbackID: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                msg: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_event_callback_device_added_msg {
    fn unpack(
        input: &mut In,
    ) -> xdr_codec::Result<(remote_domain_event_callback_device_added_msg, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_event_callback_device_added_msg {
                callbackID: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                dom: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                devAlias: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In>
    for remote_domain_event_callback_device_removal_failed_msg
{
    fn unpack(
        input: &mut In,
    ) -> xdr_codec::Result<(
        remote_domain_event_callback_device_removal_failed_msg,
        usize,
    )> {
        let mut sz = 0;
        Ok((
            remote_domain_event_callback_device_removal_failed_msg {
                callbackID: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                dom: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                devAlias: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In>
    for remote_domain_event_callback_device_removed_msg
{
    fn unpack(
        input: &mut In,
    ) -> xdr_codec::Result<(remote_domain_event_callback_device_removed_msg, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_event_callback_device_removed_msg {
                callbackID: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                msg: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_event_callback_disk_change_msg {
    fn unpack(
        input: &mut In,
    ) -> xdr_codec::Result<(remote_domain_event_callback_disk_change_msg, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_event_callback_disk_change_msg {
                callbackID: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                msg: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_event_callback_graphics_msg {
    fn unpack(
        input: &mut In,
    ) -> xdr_codec::Result<(remote_domain_event_callback_graphics_msg, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_event_callback_graphics_msg {
                callbackID: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                msg: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_event_callback_io_error_msg {
    fn unpack(
        input: &mut In,
    ) -> xdr_codec::Result<(remote_domain_event_callback_io_error_msg, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_event_callback_io_error_msg {
                callbackID: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                msg: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In>
    for remote_domain_event_callback_io_error_reason_msg
{
    fn unpack(
        input: &mut In,
    ) -> xdr_codec::Result<(remote_domain_event_callback_io_error_reason_msg, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_event_callback_io_error_reason_msg {
                callbackID: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                msg: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_event_callback_job_completed_msg {
    fn unpack(
        input: &mut In,
    ) -> xdr_codec::Result<(remote_domain_event_callback_job_completed_msg, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_event_callback_job_completed_msg {
                callbackID: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                dom: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                params: {
                    let (v, fsz) =
                        xdr_codec::unpack_flex(input, Some(REMOTE_DOMAIN_JOB_STATS_MAX as usize))?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_event_callback_lifecycle_msg {
    fn unpack(
        input: &mut In,
    ) -> xdr_codec::Result<(remote_domain_event_callback_lifecycle_msg, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_event_callback_lifecycle_msg {
                callbackID: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                msg: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In>
    for remote_domain_event_callback_metadata_change_msg
{
    fn unpack(
        input: &mut In,
    ) -> xdr_codec::Result<(remote_domain_event_callback_metadata_change_msg, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_event_callback_metadata_change_msg {
                callbackID: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                dom: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                type_: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                nsuri: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In>
    for remote_domain_event_callback_migration_iteration_msg
{
    fn unpack(
        input: &mut In,
    ) -> xdr_codec::Result<(remote_domain_event_callback_migration_iteration_msg, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_event_callback_migration_iteration_msg {
                callbackID: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                dom: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                iteration: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In>
    for remote_domain_event_callback_pmsuspend_disk_msg
{
    fn unpack(
        input: &mut In,
    ) -> xdr_codec::Result<(remote_domain_event_callback_pmsuspend_disk_msg, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_event_callback_pmsuspend_disk_msg {
                callbackID: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                reason: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                msg: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_event_callback_pmsuspend_msg {
    fn unpack(
        input: &mut In,
    ) -> xdr_codec::Result<(remote_domain_event_callback_pmsuspend_msg, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_event_callback_pmsuspend_msg {
                callbackID: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                reason: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                msg: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_event_callback_pmwakeup_msg {
    fn unpack(
        input: &mut In,
    ) -> xdr_codec::Result<(remote_domain_event_callback_pmwakeup_msg, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_event_callback_pmwakeup_msg {
                callbackID: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                reason: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                msg: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_event_callback_reboot_msg {
    fn unpack(
        input: &mut In,
    ) -> xdr_codec::Result<(remote_domain_event_callback_reboot_msg, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_event_callback_reboot_msg {
                callbackID: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                msg: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_event_callback_rtc_change_msg {
    fn unpack(
        input: &mut In,
    ) -> xdr_codec::Result<(remote_domain_event_callback_rtc_change_msg, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_event_callback_rtc_change_msg {
                callbackID: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                msg: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_event_callback_tray_change_msg {
    fn unpack(
        input: &mut In,
    ) -> xdr_codec::Result<(remote_domain_event_callback_tray_change_msg, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_event_callback_tray_change_msg {
                callbackID: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                msg: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_event_callback_tunable_msg {
    fn unpack(
        input: &mut In,
    ) -> xdr_codec::Result<(remote_domain_event_callback_tunable_msg, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_event_callback_tunable_msg {
                callbackID: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                dom: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                params: {
                    let (v, fsz) = xdr_codec::unpack_flex(
                        input,
                        Some(REMOTE_DOMAIN_EVENT_TUNABLE_MAX as usize),
                    )?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_event_callback_watchdog_msg {
    fn unpack(
        input: &mut In,
    ) -> xdr_codec::Result<(remote_domain_event_callback_watchdog_msg, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_event_callback_watchdog_msg {
                callbackID: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                msg: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_event_control_error_msg {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_domain_event_control_error_msg, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_event_control_error_msg {
                dom: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_event_device_removed_msg {
    fn unpack(
        input: &mut In,
    ) -> xdr_codec::Result<(remote_domain_event_device_removed_msg, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_event_device_removed_msg {
                dom: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                devAlias: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_event_disk_change_msg {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_domain_event_disk_change_msg, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_event_disk_change_msg {
                dom: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                oldSrcPath: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                newSrcPath: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                devAlias: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                reason: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_event_graphics_address {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_domain_event_graphics_address, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_event_graphics_address {
                family: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                node: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                service: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_event_graphics_identity {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_domain_event_graphics_identity, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_event_graphics_identity {
                type_: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                name: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_event_graphics_msg {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_domain_event_graphics_msg, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_event_graphics_msg {
                dom: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                phase: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                local: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                remote: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                authScheme: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                subject: {
                    let (v, fsz) = xdr_codec::unpack_flex(
                        input,
                        Some(REMOTE_DOMAIN_EVENT_GRAPHICS_IDENTITY_MAX as usize),
                    )?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_event_io_error_msg {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_domain_event_io_error_msg, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_event_io_error_msg {
                dom: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                srcPath: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                devAlias: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                action: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_event_io_error_reason_msg {
    fn unpack(
        input: &mut In,
    ) -> xdr_codec::Result<(remote_domain_event_io_error_reason_msg, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_event_io_error_reason_msg {
                dom: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                srcPath: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                devAlias: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                action: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                reason: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_event_lifecycle_msg {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_domain_event_lifecycle_msg, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_event_lifecycle_msg {
                dom: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                event: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                detail: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_event_pmsuspend_disk_msg {
    fn unpack(
        input: &mut In,
    ) -> xdr_codec::Result<(remote_domain_event_pmsuspend_disk_msg, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_event_pmsuspend_disk_msg {
                dom: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_event_pmsuspend_msg {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_domain_event_pmsuspend_msg, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_event_pmsuspend_msg {
                dom: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_event_pmwakeup_msg {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_domain_event_pmwakeup_msg, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_event_pmwakeup_msg {
                dom: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_event_reboot_msg {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_domain_event_reboot_msg, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_event_reboot_msg {
                dom: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_event_rtc_change_msg {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_domain_event_rtc_change_msg, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_event_rtc_change_msg {
                dom: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                offset: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_event_tray_change_msg {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_domain_event_tray_change_msg, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_event_tray_change_msg {
                dom: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                devAlias: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                reason: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_event_watchdog_msg {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_domain_event_watchdog_msg, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_event_watchdog_msg {
                dom: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                action: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_fsfreeze_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_domain_fsfreeze_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_fsfreeze_args {
                dom: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                mountpoints: {
                    let (v, fsz) = xdr_codec::unpack_flex(
                        input,
                        Some(REMOTE_DOMAIN_FSFREEZE_MOUNTPOINTS_MAX as usize),
                    )?;
                    sz += fsz;
                    v
                },
                flags: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_fsfreeze_ret {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_domain_fsfreeze_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_fsfreeze_ret {
                filesystems: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_fsinfo {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_domain_fsinfo, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_fsinfo {
                mountpoint: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                name: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                fstype: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                dev_aliases: {
                    let (v, fsz) = xdr_codec::unpack_flex(
                        input,
                        Some(REMOTE_DOMAIN_FSINFO_DISKS_MAX as usize),
                    )?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_fsthaw_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_domain_fsthaw_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_fsthaw_args {
                dom: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                mountpoints: {
                    let (v, fsz) = xdr_codec::unpack_flex(
                        input,
                        Some(REMOTE_DOMAIN_FSFREEZE_MOUNTPOINTS_MAX as usize),
                    )?;
                    sz += fsz;
                    v
                },
                flags: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_fsthaw_ret {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_domain_fsthaw_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_fsthaw_ret {
                filesystems: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_fstrim_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_domain_fstrim_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_fstrim_args {
                dom: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                mountPoint: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                minimum: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                flags: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_get_autostart_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_domain_get_autostart_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_get_autostart_args {
                dom: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_get_autostart_ret {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_domain_get_autostart_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_get_autostart_ret {
                autostart: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_get_blkio_parameters_args {
    fn unpack(
        input: &mut In,
    ) -> xdr_codec::Result<(remote_domain_get_blkio_parameters_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_get_blkio_parameters_args {
                dom: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                nparams: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                flags: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_get_blkio_parameters_ret {
    fn unpack(
        input: &mut In,
    ) -> xdr_codec::Result<(remote_domain_get_blkio_parameters_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_get_blkio_parameters_ret {
                params: {
                    let (v, fsz) = xdr_codec::unpack_flex(
                        input,
                        Some(REMOTE_DOMAIN_BLKIO_PARAMETERS_MAX as usize),
                    )?;
                    sz += fsz;
                    v
                },
                nparams: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_get_block_info_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_domain_get_block_info_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_get_block_info_args {
                dom: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                path: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                flags: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_get_block_info_ret {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_domain_get_block_info_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_get_block_info_ret {
                allocation: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                capacity: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                physical: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_get_block_io_tune_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_domain_get_block_io_tune_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_get_block_io_tune_args {
                dom: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                disk: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                nparams: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                flags: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_get_block_io_tune_ret {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_domain_get_block_io_tune_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_get_block_io_tune_ret {
                params: {
                    let (v, fsz) = xdr_codec::unpack_flex(
                        input,
                        Some(REMOTE_DOMAIN_BLOCK_IO_TUNE_PARAMETERS_MAX as usize),
                    )?;
                    sz += fsz;
                    v
                },
                nparams: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_get_block_job_info_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_domain_get_block_job_info_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_get_block_job_info_args {
                dom: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                path: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                flags: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_get_block_job_info_ret {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_domain_get_block_job_info_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_get_block_job_info_ret {
                found: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                type_: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                bandwidth: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                cur: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                end: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_get_control_info_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_domain_get_control_info_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_get_control_info_args {
                dom: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                flags: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_get_control_info_ret {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_domain_get_control_info_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_get_control_info_ret {
                state: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                details: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                stateTime: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_get_cpu_stats_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_domain_get_cpu_stats_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_get_cpu_stats_args {
                dom: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                nparams: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                start_cpu: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                ncpus: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                flags: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_get_cpu_stats_ret {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_domain_get_cpu_stats_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_get_cpu_stats_ret {
                params: {
                    let (v, fsz) = xdr_codec::unpack_flex(
                        input,
                        Some(REMOTE_DOMAIN_GET_CPU_STATS_MAX as usize),
                    )?;
                    sz += fsz;
                    v
                },
                nparams: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_get_disk_errors_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_domain_get_disk_errors_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_get_disk_errors_args {
                dom: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                maxerrors: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                flags: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_get_disk_errors_ret {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_domain_get_disk_errors_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_get_disk_errors_ret {
                errors: {
                    let (v, fsz) = xdr_codec::unpack_flex(
                        input,
                        Some(REMOTE_DOMAIN_DISK_ERRORS_MAX as usize),
                    )?;
                    sz += fsz;
                    v
                },
                nerrors: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_get_emulator_pin_info_args {
    fn unpack(
        input: &mut In,
    ) -> xdr_codec::Result<(remote_domain_get_emulator_pin_info_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_get_emulator_pin_info_args {
                dom: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                maplen: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                flags: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_get_emulator_pin_info_ret {
    fn unpack(
        input: &mut In,
    ) -> xdr_codec::Result<(remote_domain_get_emulator_pin_info_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_get_emulator_pin_info_ret {
                cpumaps: {
                    let (v, fsz) =
                        xdr_codec::unpack_opaque_flex(input, Some(REMOTE_CPUMAPS_MAX as usize))?;
                    sz += fsz;
                    v
                },
                ret: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_get_fsinfo_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_domain_get_fsinfo_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_get_fsinfo_args {
                dom: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                flags: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_get_fsinfo_ret {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_domain_get_fsinfo_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_get_fsinfo_ret {
                info: {
                    let (v, fsz) =
                        xdr_codec::unpack_flex(input, Some(REMOTE_DOMAIN_FSINFO_MAX as usize))?;
                    sz += fsz;
                    v
                },
                ret: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_get_guest_vcpus_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_domain_get_guest_vcpus_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_get_guest_vcpus_args {
                dom: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                flags: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_get_guest_vcpus_ret {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_domain_get_guest_vcpus_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_get_guest_vcpus_ret {
                params: {
                    let (v, fsz) = xdr_codec::unpack_flex(
                        input,
                        Some(REMOTE_DOMAIN_GUEST_VCPU_PARAMS_MAX as usize),
                    )?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_get_hostname_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_domain_get_hostname_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_get_hostname_args {
                dom: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                flags: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_get_hostname_ret {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_domain_get_hostname_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_get_hostname_ret {
                hostname: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_get_info_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_domain_get_info_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_get_info_args {
                dom: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_get_info_ret {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_domain_get_info_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_get_info_ret {
                state: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                maxMem: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                memory: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                nrVirtCpu: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                cpuTime: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_get_interface_parameters_args {
    fn unpack(
        input: &mut In,
    ) -> xdr_codec::Result<(remote_domain_get_interface_parameters_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_get_interface_parameters_args {
                dom: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                device: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                nparams: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                flags: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_get_interface_parameters_ret {
    fn unpack(
        input: &mut In,
    ) -> xdr_codec::Result<(remote_domain_get_interface_parameters_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_get_interface_parameters_ret {
                params: {
                    let (v, fsz) = xdr_codec::unpack_flex(
                        input,
                        Some(REMOTE_DOMAIN_INTERFACE_PARAMETERS_MAX as usize),
                    )?;
                    sz += fsz;
                    v
                },
                nparams: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_get_iothread_info_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_domain_get_iothread_info_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_get_iothread_info_args {
                dom: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                flags: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_get_iothread_info_ret {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_domain_get_iothread_info_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_get_iothread_info_ret {
                info: {
                    let (v, fsz) =
                        xdr_codec::unpack_flex(input, Some(REMOTE_IOTHREAD_INFO_MAX as usize))?;
                    sz += fsz;
                    v
                },
                ret: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_get_job_info_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_domain_get_job_info_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_get_job_info_args {
                dom: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_get_job_info_ret {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_domain_get_job_info_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_get_job_info_ret {
                type_: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                timeElapsed: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                timeRemaining: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                dataTotal: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                dataProcessed: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                dataRemaining: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                memTotal: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                memProcessed: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                memRemaining: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                fileTotal: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                fileProcessed: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                fileRemaining: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_get_job_stats_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_domain_get_job_stats_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_get_job_stats_args {
                dom: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                flags: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_get_job_stats_ret {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_domain_get_job_stats_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_get_job_stats_ret {
                type_: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                params: {
                    let (v, fsz) =
                        xdr_codec::unpack_flex(input, Some(REMOTE_DOMAIN_JOB_STATS_MAX as usize))?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_get_max_memory_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_domain_get_max_memory_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_get_max_memory_args {
                dom: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_get_max_memory_ret {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_domain_get_max_memory_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_get_max_memory_ret {
                memory: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_get_max_vcpus_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_domain_get_max_vcpus_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_get_max_vcpus_args {
                dom: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_get_max_vcpus_ret {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_domain_get_max_vcpus_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_get_max_vcpus_ret {
                num: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_get_memory_parameters_args {
    fn unpack(
        input: &mut In,
    ) -> xdr_codec::Result<(remote_domain_get_memory_parameters_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_get_memory_parameters_args {
                dom: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                nparams: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                flags: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_get_memory_parameters_ret {
    fn unpack(
        input: &mut In,
    ) -> xdr_codec::Result<(remote_domain_get_memory_parameters_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_get_memory_parameters_ret {
                params: {
                    let (v, fsz) = xdr_codec::unpack_flex(
                        input,
                        Some(REMOTE_DOMAIN_MEMORY_PARAMETERS_MAX as usize),
                    )?;
                    sz += fsz;
                    v
                },
                nparams: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_get_metadata_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_domain_get_metadata_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_get_metadata_args {
                dom: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                type_: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                uri: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                flags: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_get_metadata_ret {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_domain_get_metadata_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_get_metadata_ret {
                metadata: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_get_numa_parameters_args {
    fn unpack(
        input: &mut In,
    ) -> xdr_codec::Result<(remote_domain_get_numa_parameters_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_get_numa_parameters_args {
                dom: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                nparams: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                flags: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_get_numa_parameters_ret {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_domain_get_numa_parameters_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_get_numa_parameters_ret {
                params: {
                    let (v, fsz) = xdr_codec::unpack_flex(
                        input,
                        Some(REMOTE_DOMAIN_NUMA_PARAMETERS_MAX as usize),
                    )?;
                    sz += fsz;
                    v
                },
                nparams: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_get_os_type_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_domain_get_os_type_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_get_os_type_args {
                dom: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_get_os_type_ret {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_domain_get_os_type_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_get_os_type_ret {
                type_: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_get_perf_events_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_domain_get_perf_events_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_get_perf_events_args {
                dom: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                flags: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_get_perf_events_ret {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_domain_get_perf_events_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_get_perf_events_ret {
                params: {
                    let (v, fsz) = xdr_codec::unpack_flex(
                        input,
                        Some(REMOTE_DOMAIN_PERF_EVENTS_MAX as usize),
                    )?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_get_scheduler_parameters_args {
    fn unpack(
        input: &mut In,
    ) -> xdr_codec::Result<(remote_domain_get_scheduler_parameters_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_get_scheduler_parameters_args {
                dom: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                nparams: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In>
    for remote_domain_get_scheduler_parameters_flags_args
{
    fn unpack(
        input: &mut In,
    ) -> xdr_codec::Result<(remote_domain_get_scheduler_parameters_flags_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_get_scheduler_parameters_flags_args {
                dom: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                nparams: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                flags: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In>
    for remote_domain_get_scheduler_parameters_flags_ret
{
    fn unpack(
        input: &mut In,
    ) -> xdr_codec::Result<(remote_domain_get_scheduler_parameters_flags_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_get_scheduler_parameters_flags_ret {
                params: {
                    let (v, fsz) = xdr_codec::unpack_flex(
                        input,
                        Some(REMOTE_DOMAIN_SCHEDULER_PARAMETERS_MAX as usize),
                    )?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_get_scheduler_parameters_ret {
    fn unpack(
        input: &mut In,
    ) -> xdr_codec::Result<(remote_domain_get_scheduler_parameters_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_get_scheduler_parameters_ret {
                params: {
                    let (v, fsz) = xdr_codec::unpack_flex(
                        input,
                        Some(REMOTE_DOMAIN_SCHEDULER_PARAMETERS_MAX as usize),
                    )?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_get_scheduler_type_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_domain_get_scheduler_type_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_get_scheduler_type_args {
                dom: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_get_scheduler_type_ret {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_domain_get_scheduler_type_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_get_scheduler_type_ret {
                type_: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                nparams: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_get_security_label_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_domain_get_security_label_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_get_security_label_args {
                dom: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_get_security_label_list_args {
    fn unpack(
        input: &mut In,
    ) -> xdr_codec::Result<(remote_domain_get_security_label_list_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_get_security_label_list_args {
                dom: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_get_security_label_list_ret {
    fn unpack(
        input: &mut In,
    ) -> xdr_codec::Result<(remote_domain_get_security_label_list_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_get_security_label_list_ret {
                labels: {
                    let (v, fsz) = xdr_codec::unpack_flex(
                        input,
                        Some(REMOTE_SECURITY_LABEL_LIST_MAX as usize),
                    )?;
                    sz += fsz;
                    v
                },
                ret: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_get_security_label_ret {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_domain_get_security_label_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_get_security_label_ret {
                label: {
                    let (v, fsz) =
                        xdr_codec::unpack_flex(input, Some(REMOTE_SECURITY_LABEL_MAX as usize))?;
                    sz += fsz;
                    v
                },
                enforcing: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_get_state_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_domain_get_state_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_get_state_args {
                dom: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                flags: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_get_state_ret {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_domain_get_state_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_get_state_ret {
                state: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                reason: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_get_time_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_domain_get_time_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_get_time_args {
                dom: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                flags: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_get_time_ret {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_domain_get_time_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_get_time_ret {
                seconds: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                nseconds: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_get_vcpu_pin_info_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_domain_get_vcpu_pin_info_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_get_vcpu_pin_info_args {
                dom: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                ncpumaps: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                maplen: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                flags: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_get_vcpu_pin_info_ret {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_domain_get_vcpu_pin_info_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_get_vcpu_pin_info_ret {
                cpumaps: {
                    let (v, fsz) =
                        xdr_codec::unpack_opaque_flex(input, Some(REMOTE_CPUMAPS_MAX as usize))?;
                    sz += fsz;
                    v
                },
                num: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_get_vcpus_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_domain_get_vcpus_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_get_vcpus_args {
                dom: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                maxinfo: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                maplen: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_get_vcpus_flags_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_domain_get_vcpus_flags_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_get_vcpus_flags_args {
                dom: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                flags: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_get_vcpus_flags_ret {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_domain_get_vcpus_flags_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_get_vcpus_flags_ret {
                num: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_get_vcpus_ret {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_domain_get_vcpus_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_get_vcpus_ret {
                info: {
                    let (v, fsz) =
                        xdr_codec::unpack_flex(input, Some(REMOTE_VCPUINFO_MAX as usize))?;
                    sz += fsz;
                    v
                },
                cpumaps: {
                    let (v, fsz) =
                        xdr_codec::unpack_opaque_flex(input, Some(REMOTE_CPUMAPS_MAX as usize))?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_get_xml_desc_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_domain_get_xml_desc_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_get_xml_desc_args {
                dom: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                flags: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_get_xml_desc_ret {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_domain_get_xml_desc_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_get_xml_desc_ret {
                xml: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_has_current_snapshot_args {
    fn unpack(
        input: &mut In,
    ) -> xdr_codec::Result<(remote_domain_has_current_snapshot_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_has_current_snapshot_args {
                dom: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                flags: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_has_current_snapshot_ret {
    fn unpack(
        input: &mut In,
    ) -> xdr_codec::Result<(remote_domain_has_current_snapshot_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_has_current_snapshot_ret {
                result: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_has_managed_save_image_args {
    fn unpack(
        input: &mut In,
    ) -> xdr_codec::Result<(remote_domain_has_managed_save_image_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_has_managed_save_image_args {
                dom: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                flags: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_has_managed_save_image_ret {
    fn unpack(
        input: &mut In,
    ) -> xdr_codec::Result<(remote_domain_has_managed_save_image_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_has_managed_save_image_ret {
                result: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_inject_nmi_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_domain_inject_nmi_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_inject_nmi_args {
                dom: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                flags: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_interface {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_domain_interface, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_interface {
                name: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                hwaddr: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                addrs: {
                    let (v, fsz) =
                        xdr_codec::unpack_flex(input, Some(REMOTE_DOMAIN_IP_ADDR_MAX as usize))?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_interface_addresses_args {
    fn unpack(
        input: &mut In,
    ) -> xdr_codec::Result<(remote_domain_interface_addresses_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_interface_addresses_args {
                dom: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                source: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                flags: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_interface_addresses_ret {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_domain_interface_addresses_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_interface_addresses_ret {
                ifaces: {
                    let (v, fsz) =
                        xdr_codec::unpack_flex(input, Some(REMOTE_DOMAIN_INTERFACE_MAX as usize))?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_interface_stats_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_domain_interface_stats_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_interface_stats_args {
                dom: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                path: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_interface_stats_ret {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_domain_interface_stats_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_interface_stats_ret {
                rx_bytes: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                rx_packets: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                rx_errs: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                rx_drop: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                tx_bytes: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                tx_packets: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                tx_errs: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                tx_drop: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_iothread_info {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_domain_iothread_info, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_iothread_info {
                iothread_id: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                cpumap: {
                    let (v, fsz) =
                        xdr_codec::unpack_opaque_flex(input, Some(REMOTE_CPUMAP_MAX as usize))?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_ip_addr {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_domain_ip_addr, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_ip_addr {
                type_: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                addr: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                prefix: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_is_active_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_domain_is_active_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_is_active_args {
                dom: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_is_active_ret {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_domain_is_active_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_is_active_ret {
                active: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_is_persistent_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_domain_is_persistent_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_is_persistent_args {
                dom: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_is_persistent_ret {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_domain_is_persistent_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_is_persistent_ret {
                persistent: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_is_updated_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_domain_is_updated_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_is_updated_args {
                dom: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_is_updated_ret {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_domain_is_updated_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_is_updated_ret {
                updated: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_list_all_snapshots_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_domain_list_all_snapshots_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_list_all_snapshots_args {
                dom: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                need_results: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                flags: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_list_all_snapshots_ret {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_domain_list_all_snapshots_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_list_all_snapshots_ret {
                snapshots: {
                    let (v, fsz) = xdr_codec::unpack_flex(
                        input,
                        Some(REMOTE_DOMAIN_SNAPSHOT_LIST_MAX as usize),
                    )?;
                    sz += fsz;
                    v
                },
                ret: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_lookup_by_id_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_domain_lookup_by_id_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_lookup_by_id_args {
                id: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_lookup_by_id_ret {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_domain_lookup_by_id_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_lookup_by_id_ret {
                dom: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_lookup_by_name_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_domain_lookup_by_name_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_lookup_by_name_args {
                name: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_lookup_by_name_ret {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_domain_lookup_by_name_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_lookup_by_name_ret {
                dom: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_lookup_by_uuid_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_domain_lookup_by_uuid_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_lookup_by_uuid_args {
                uuid: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_lookup_by_uuid_ret {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_domain_lookup_by_uuid_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_lookup_by_uuid_ret {
                dom: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_managed_save_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_domain_managed_save_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_managed_save_args {
                dom: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                flags: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_managed_save_remove_args {
    fn unpack(
        input: &mut In,
    ) -> xdr_codec::Result<(remote_domain_managed_save_remove_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_managed_save_remove_args {
                dom: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                flags: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_memory_peek_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_domain_memory_peek_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_memory_peek_args {
                dom: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                offset: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                size: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                flags: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_memory_peek_ret {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_domain_memory_peek_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_memory_peek_ret {
                buffer: {
                    let (v, fsz) = xdr_codec::unpack_opaque_flex(
                        input,
                        Some(REMOTE_DOMAIN_MEMORY_PEEK_BUFFER_MAX as usize),
                    )?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_memory_stat {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_domain_memory_stat, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_memory_stat {
                tag: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                val: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_memory_stats_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_domain_memory_stats_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_memory_stats_args {
                dom: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                maxStats: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                flags: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_memory_stats_ret {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_domain_memory_stats_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_memory_stats_ret {
                stats: {
                    let (v, fsz) = xdr_codec::unpack_flex(
                        input,
                        Some(REMOTE_DOMAIN_MEMORY_STATS_MAX as usize),
                    )?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_migrate_begin3_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_domain_migrate_begin3_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_migrate_begin3_args {
                dom: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                xmlin: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                flags: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                dname: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                resource: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_migrate_begin3_params_args {
    fn unpack(
        input: &mut In,
    ) -> xdr_codec::Result<(remote_domain_migrate_begin3_params_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_migrate_begin3_params_args {
                dom: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                params: {
                    let (v, fsz) = xdr_codec::unpack_flex(
                        input,
                        Some(REMOTE_DOMAIN_MIGRATE_PARAM_LIST_MAX as usize),
                    )?;
                    sz += fsz;
                    v
                },
                flags: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_migrate_begin3_params_ret {
    fn unpack(
        input: &mut In,
    ) -> xdr_codec::Result<(remote_domain_migrate_begin3_params_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_migrate_begin3_params_ret {
                cookie_out: {
                    let (v, fsz) = xdr_codec::unpack_opaque_flex(
                        input,
                        Some(REMOTE_MIGRATE_COOKIE_MAX as usize),
                    )?;
                    sz += fsz;
                    v
                },
                xml: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_migrate_begin3_ret {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_domain_migrate_begin3_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_migrate_begin3_ret {
                cookie_out: {
                    let (v, fsz) = xdr_codec::unpack_opaque_flex(
                        input,
                        Some(REMOTE_MIGRATE_COOKIE_MAX as usize),
                    )?;
                    sz += fsz;
                    v
                },
                xml: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_migrate_confirm3_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_domain_migrate_confirm3_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_migrate_confirm3_args {
                dom: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                cookie_in: {
                    let (v, fsz) = xdr_codec::unpack_opaque_flex(
                        input,
                        Some(REMOTE_MIGRATE_COOKIE_MAX as usize),
                    )?;
                    sz += fsz;
                    v
                },
                flags: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                cancelled: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_migrate_confirm3_params_args {
    fn unpack(
        input: &mut In,
    ) -> xdr_codec::Result<(remote_domain_migrate_confirm3_params_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_migrate_confirm3_params_args {
                dom: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                params: {
                    let (v, fsz) = xdr_codec::unpack_flex(
                        input,
                        Some(REMOTE_DOMAIN_MIGRATE_PARAM_LIST_MAX as usize),
                    )?;
                    sz += fsz;
                    v
                },
                cookie_in: {
                    let (v, fsz) = xdr_codec::unpack_opaque_flex(
                        input,
                        Some(REMOTE_MIGRATE_COOKIE_MAX as usize),
                    )?;
                    sz += fsz;
                    v
                },
                flags: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                cancelled: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_migrate_finish2_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_domain_migrate_finish2_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_migrate_finish2_args {
                dname: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                cookie: {
                    let (v, fsz) = xdr_codec::unpack_opaque_flex(
                        input,
                        Some(REMOTE_MIGRATE_COOKIE_MAX as usize),
                    )?;
                    sz += fsz;
                    v
                },
                uri: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                flags: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                retcode: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_migrate_finish2_ret {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_domain_migrate_finish2_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_migrate_finish2_ret {
                ddom: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_migrate_finish3_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_domain_migrate_finish3_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_migrate_finish3_args {
                dname: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                cookie_in: {
                    let (v, fsz) = xdr_codec::unpack_opaque_flex(
                        input,
                        Some(REMOTE_MIGRATE_COOKIE_MAX as usize),
                    )?;
                    sz += fsz;
                    v
                },
                dconnuri: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                uri: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                flags: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                cancelled: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_migrate_finish3_params_args {
    fn unpack(
        input: &mut In,
    ) -> xdr_codec::Result<(remote_domain_migrate_finish3_params_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_migrate_finish3_params_args {
                params: {
                    let (v, fsz) = xdr_codec::unpack_flex(
                        input,
                        Some(REMOTE_DOMAIN_MIGRATE_PARAM_LIST_MAX as usize),
                    )?;
                    sz += fsz;
                    v
                },
                cookie_in: {
                    let (v, fsz) = xdr_codec::unpack_opaque_flex(
                        input,
                        Some(REMOTE_MIGRATE_COOKIE_MAX as usize),
                    )?;
                    sz += fsz;
                    v
                },
                flags: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                cancelled: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_migrate_finish3_params_ret {
    fn unpack(
        input: &mut In,
    ) -> xdr_codec::Result<(remote_domain_migrate_finish3_params_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_migrate_finish3_params_ret {
                dom: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                cookie_out: {
                    let (v, fsz) = xdr_codec::unpack_opaque_flex(
                        input,
                        Some(REMOTE_MIGRATE_COOKIE_MAX as usize),
                    )?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_migrate_finish3_ret {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_domain_migrate_finish3_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_migrate_finish3_ret {
                dom: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                cookie_out: {
                    let (v, fsz) = xdr_codec::unpack_opaque_flex(
                        input,
                        Some(REMOTE_MIGRATE_COOKIE_MAX as usize),
                    )?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_migrate_finish_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_domain_migrate_finish_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_migrate_finish_args {
                dname: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                cookie: {
                    let (v, fsz) = xdr_codec::unpack_opaque_flex(
                        input,
                        Some(REMOTE_MIGRATE_COOKIE_MAX as usize),
                    )?;
                    sz += fsz;
                    v
                },
                uri: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                flags: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_migrate_finish_ret {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_domain_migrate_finish_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_migrate_finish_ret {
                ddom: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In>
    for remote_domain_migrate_get_compression_cache_args
{
    fn unpack(
        input: &mut In,
    ) -> xdr_codec::Result<(remote_domain_migrate_get_compression_cache_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_migrate_get_compression_cache_args {
                dom: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                flags: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In>
    for remote_domain_migrate_get_compression_cache_ret
{
    fn unpack(
        input: &mut In,
    ) -> xdr_codec::Result<(remote_domain_migrate_get_compression_cache_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_migrate_get_compression_cache_ret {
                cacheSize: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_migrate_get_max_speed_args {
    fn unpack(
        input: &mut In,
    ) -> xdr_codec::Result<(remote_domain_migrate_get_max_speed_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_migrate_get_max_speed_args {
                dom: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                flags: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_migrate_get_max_speed_ret {
    fn unpack(
        input: &mut In,
    ) -> xdr_codec::Result<(remote_domain_migrate_get_max_speed_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_migrate_get_max_speed_ret {
                bandwidth: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_migrate_perform3_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_domain_migrate_perform3_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_migrate_perform3_args {
                dom: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                xmlin: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                cookie_in: {
                    let (v, fsz) = xdr_codec::unpack_opaque_flex(
                        input,
                        Some(REMOTE_MIGRATE_COOKIE_MAX as usize),
                    )?;
                    sz += fsz;
                    v
                },
                dconnuri: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                uri: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                flags: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                dname: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                resource: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_migrate_perform3_params_args {
    fn unpack(
        input: &mut In,
    ) -> xdr_codec::Result<(remote_domain_migrate_perform3_params_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_migrate_perform3_params_args {
                dom: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                dconnuri: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                params: {
                    let (v, fsz) = xdr_codec::unpack_flex(
                        input,
                        Some(REMOTE_DOMAIN_MIGRATE_PARAM_LIST_MAX as usize),
                    )?;
                    sz += fsz;
                    v
                },
                cookie_in: {
                    let (v, fsz) = xdr_codec::unpack_opaque_flex(
                        input,
                        Some(REMOTE_MIGRATE_COOKIE_MAX as usize),
                    )?;
                    sz += fsz;
                    v
                },
                flags: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_migrate_perform3_params_ret {
    fn unpack(
        input: &mut In,
    ) -> xdr_codec::Result<(remote_domain_migrate_perform3_params_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_migrate_perform3_params_ret {
                cookie_out: {
                    let (v, fsz) = xdr_codec::unpack_opaque_flex(
                        input,
                        Some(REMOTE_MIGRATE_COOKIE_MAX as usize),
                    )?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_migrate_perform3_ret {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_domain_migrate_perform3_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_migrate_perform3_ret {
                cookie_out: {
                    let (v, fsz) = xdr_codec::unpack_opaque_flex(
                        input,
                        Some(REMOTE_MIGRATE_COOKIE_MAX as usize),
                    )?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_migrate_perform_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_domain_migrate_perform_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_migrate_perform_args {
                dom: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                cookie: {
                    let (v, fsz) = xdr_codec::unpack_opaque_flex(
                        input,
                        Some(REMOTE_MIGRATE_COOKIE_MAX as usize),
                    )?;
                    sz += fsz;
                    v
                },
                uri: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                flags: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                dname: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                resource: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_migrate_prepare2_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_domain_migrate_prepare2_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_migrate_prepare2_args {
                uri_in: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                flags: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                dname: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                resource: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                dom_xml: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_migrate_prepare2_ret {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_domain_migrate_prepare2_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_migrate_prepare2_ret {
                cookie: {
                    let (v, fsz) = xdr_codec::unpack_opaque_flex(
                        input,
                        Some(REMOTE_MIGRATE_COOKIE_MAX as usize),
                    )?;
                    sz += fsz;
                    v
                },
                uri_out: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_migrate_prepare3_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_domain_migrate_prepare3_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_migrate_prepare3_args {
                cookie_in: {
                    let (v, fsz) = xdr_codec::unpack_opaque_flex(
                        input,
                        Some(REMOTE_MIGRATE_COOKIE_MAX as usize),
                    )?;
                    sz += fsz;
                    v
                },
                uri_in: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                flags: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                dname: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                resource: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                dom_xml: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_migrate_prepare3_params_args {
    fn unpack(
        input: &mut In,
    ) -> xdr_codec::Result<(remote_domain_migrate_prepare3_params_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_migrate_prepare3_params_args {
                params: {
                    let (v, fsz) = xdr_codec::unpack_flex(
                        input,
                        Some(REMOTE_DOMAIN_MIGRATE_PARAM_LIST_MAX as usize),
                    )?;
                    sz += fsz;
                    v
                },
                cookie_in: {
                    let (v, fsz) = xdr_codec::unpack_opaque_flex(
                        input,
                        Some(REMOTE_MIGRATE_COOKIE_MAX as usize),
                    )?;
                    sz += fsz;
                    v
                },
                flags: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_migrate_prepare3_params_ret {
    fn unpack(
        input: &mut In,
    ) -> xdr_codec::Result<(remote_domain_migrate_prepare3_params_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_migrate_prepare3_params_ret {
                cookie_out: {
                    let (v, fsz) = xdr_codec::unpack_opaque_flex(
                        input,
                        Some(REMOTE_MIGRATE_COOKIE_MAX as usize),
                    )?;
                    sz += fsz;
                    v
                },
                uri_out: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_migrate_prepare3_ret {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_domain_migrate_prepare3_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_migrate_prepare3_ret {
                cookie_out: {
                    let (v, fsz) = xdr_codec::unpack_opaque_flex(
                        input,
                        Some(REMOTE_MIGRATE_COOKIE_MAX as usize),
                    )?;
                    sz += fsz;
                    v
                },
                uri_out: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_migrate_prepare_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_domain_migrate_prepare_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_migrate_prepare_args {
                uri_in: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                flags: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                dname: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                resource: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_migrate_prepare_ret {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_domain_migrate_prepare_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_migrate_prepare_ret {
                cookie: {
                    let (v, fsz) = xdr_codec::unpack_opaque_flex(
                        input,
                        Some(REMOTE_MIGRATE_COOKIE_MAX as usize),
                    )?;
                    sz += fsz;
                    v
                },
                uri_out: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_migrate_prepare_tunnel3_args {
    fn unpack(
        input: &mut In,
    ) -> xdr_codec::Result<(remote_domain_migrate_prepare_tunnel3_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_migrate_prepare_tunnel3_args {
                cookie_in: {
                    let (v, fsz) = xdr_codec::unpack_opaque_flex(
                        input,
                        Some(REMOTE_MIGRATE_COOKIE_MAX as usize),
                    )?;
                    sz += fsz;
                    v
                },
                flags: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                dname: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                resource: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                dom_xml: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In>
    for remote_domain_migrate_prepare_tunnel3_params_args
{
    fn unpack(
        input: &mut In,
    ) -> xdr_codec::Result<(remote_domain_migrate_prepare_tunnel3_params_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_migrate_prepare_tunnel3_params_args {
                params: {
                    let (v, fsz) = xdr_codec::unpack_flex(
                        input,
                        Some(REMOTE_DOMAIN_MIGRATE_PARAM_LIST_MAX as usize),
                    )?;
                    sz += fsz;
                    v
                },
                cookie_in: {
                    let (v, fsz) = xdr_codec::unpack_opaque_flex(
                        input,
                        Some(REMOTE_MIGRATE_COOKIE_MAX as usize),
                    )?;
                    sz += fsz;
                    v
                },
                flags: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In>
    for remote_domain_migrate_prepare_tunnel3_params_ret
{
    fn unpack(
        input: &mut In,
    ) -> xdr_codec::Result<(remote_domain_migrate_prepare_tunnel3_params_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_migrate_prepare_tunnel3_params_ret {
                cookie_out: {
                    let (v, fsz) = xdr_codec::unpack_opaque_flex(
                        input,
                        Some(REMOTE_MIGRATE_COOKIE_MAX as usize),
                    )?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_migrate_prepare_tunnel3_ret {
    fn unpack(
        input: &mut In,
    ) -> xdr_codec::Result<(remote_domain_migrate_prepare_tunnel3_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_migrate_prepare_tunnel3_ret {
                cookie_out: {
                    let (v, fsz) = xdr_codec::unpack_opaque_flex(
                        input,
                        Some(REMOTE_MIGRATE_COOKIE_MAX as usize),
                    )?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_migrate_prepare_tunnel_args {
    fn unpack(
        input: &mut In,
    ) -> xdr_codec::Result<(remote_domain_migrate_prepare_tunnel_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_migrate_prepare_tunnel_args {
                flags: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                dname: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                resource: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                dom_xml: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In>
    for remote_domain_migrate_set_compression_cache_args
{
    fn unpack(
        input: &mut In,
    ) -> xdr_codec::Result<(remote_domain_migrate_set_compression_cache_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_migrate_set_compression_cache_args {
                dom: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                cacheSize: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                flags: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_migrate_set_max_downtime_args {
    fn unpack(
        input: &mut In,
    ) -> xdr_codec::Result<(remote_domain_migrate_set_max_downtime_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_migrate_set_max_downtime_args {
                dom: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                downtime: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                flags: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_migrate_set_max_speed_args {
    fn unpack(
        input: &mut In,
    ) -> xdr_codec::Result<(remote_domain_migrate_set_max_speed_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_migrate_set_max_speed_args {
                dom: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                bandwidth: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                flags: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_migrate_start_post_copy_args {
    fn unpack(
        input: &mut In,
    ) -> xdr_codec::Result<(remote_domain_migrate_start_post_copy_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_migrate_start_post_copy_args {
                dom: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                flags: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_open_channel_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_domain_open_channel_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_open_channel_args {
                dom: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                name: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                flags: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_open_console_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_domain_open_console_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_open_console_args {
                dom: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                dev_name: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                flags: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_open_graphics_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_domain_open_graphics_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_open_graphics_args {
                dom: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                idx: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                flags: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_open_graphics_fd_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_domain_open_graphics_fd_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_open_graphics_fd_args {
                dom: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                idx: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                flags: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_pin_emulator_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_domain_pin_emulator_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_pin_emulator_args {
                dom: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                cpumap: {
                    let (v, fsz) =
                        xdr_codec::unpack_opaque_flex(input, Some(REMOTE_CPUMAP_MAX as usize))?;
                    sz += fsz;
                    v
                },
                flags: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_pin_iothread_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_domain_pin_iothread_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_pin_iothread_args {
                dom: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                iothreads_id: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                cpumap: {
                    let (v, fsz) =
                        xdr_codec::unpack_opaque_flex(input, Some(REMOTE_CPUMAP_MAX as usize))?;
                    sz += fsz;
                    v
                },
                flags: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_pin_vcpu_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_domain_pin_vcpu_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_pin_vcpu_args {
                dom: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                vcpu: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                cpumap: {
                    let (v, fsz) =
                        xdr_codec::unpack_opaque_flex(input, Some(REMOTE_CPUMAP_MAX as usize))?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_pin_vcpu_flags_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_domain_pin_vcpu_flags_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_pin_vcpu_flags_args {
                dom: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                vcpu: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                cpumap: {
                    let (v, fsz) =
                        xdr_codec::unpack_opaque_flex(input, Some(REMOTE_CPUMAP_MAX as usize))?;
                    sz += fsz;
                    v
                },
                flags: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_pm_suspend_for_duration_args {
    fn unpack(
        input: &mut In,
    ) -> xdr_codec::Result<(remote_domain_pm_suspend_for_duration_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_pm_suspend_for_duration_args {
                dom: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                target: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                duration: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                flags: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_pm_wakeup_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_domain_pm_wakeup_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_pm_wakeup_args {
                dom: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                flags: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_reboot_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_domain_reboot_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_reboot_args {
                dom: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                flags: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_rename_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_domain_rename_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_rename_args {
                dom: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                new_name: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                flags: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_rename_ret {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_domain_rename_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_rename_ret {
                retcode: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_reset_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_domain_reset_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_reset_args {
                dom: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                flags: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_restore_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_domain_restore_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_restore_args {
                from: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_restore_flags_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_domain_restore_flags_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_restore_flags_args {
                from: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                dxml: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                flags: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_resume_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_domain_resume_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_resume_args {
                dom: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_revert_to_snapshot_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_domain_revert_to_snapshot_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_revert_to_snapshot_args {
                snap: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                flags: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_save_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_domain_save_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_save_args {
                dom: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                to: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_save_flags_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_domain_save_flags_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_save_flags_args {
                dom: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                to: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                dxml: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                flags: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_save_image_define_xml_args {
    fn unpack(
        input: &mut In,
    ) -> xdr_codec::Result<(remote_domain_save_image_define_xml_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_save_image_define_xml_args {
                file: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                dxml: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                flags: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_save_image_get_xml_desc_args {
    fn unpack(
        input: &mut In,
    ) -> xdr_codec::Result<(remote_domain_save_image_get_xml_desc_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_save_image_get_xml_desc_args {
                file: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                flags: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_save_image_get_xml_desc_ret {
    fn unpack(
        input: &mut In,
    ) -> xdr_codec::Result<(remote_domain_save_image_get_xml_desc_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_save_image_get_xml_desc_ret {
                xml: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_screenshot_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_domain_screenshot_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_screenshot_args {
                dom: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                screen: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                flags: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_screenshot_ret {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_domain_screenshot_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_screenshot_ret {
                mime: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_send_key_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_domain_send_key_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_send_key_args {
                dom: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                codeset: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                holdtime: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                keycodes: {
                    let (v, fsz) =
                        xdr_codec::unpack_flex(input, Some(REMOTE_DOMAIN_SEND_KEY_MAX as usize))?;
                    sz += fsz;
                    v
                },
                flags: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_send_process_signal_args {
    fn unpack(
        input: &mut In,
    ) -> xdr_codec::Result<(remote_domain_send_process_signal_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_send_process_signal_args {
                dom: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                pid_value: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                signum: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                flags: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_set_autostart_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_domain_set_autostart_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_set_autostart_args {
                dom: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                autostart: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_set_blkio_parameters_args {
    fn unpack(
        input: &mut In,
    ) -> xdr_codec::Result<(remote_domain_set_blkio_parameters_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_set_blkio_parameters_args {
                dom: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                params: {
                    let (v, fsz) = xdr_codec::unpack_flex(
                        input,
                        Some(REMOTE_DOMAIN_BLKIO_PARAMETERS_MAX as usize),
                    )?;
                    sz += fsz;
                    v
                },
                flags: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_set_block_io_tune_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_domain_set_block_io_tune_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_set_block_io_tune_args {
                dom: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                disk: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                params: {
                    let (v, fsz) = xdr_codec::unpack_flex(
                        input,
                        Some(REMOTE_DOMAIN_BLOCK_IO_TUNE_PARAMETERS_MAX as usize),
                    )?;
                    sz += fsz;
                    v
                },
                flags: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_set_block_threshold_args {
    fn unpack(
        input: &mut In,
    ) -> xdr_codec::Result<(remote_domain_set_block_threshold_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_set_block_threshold_args {
                dom: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                dev: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                threshold: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                flags: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_set_guest_vcpus_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_domain_set_guest_vcpus_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_set_guest_vcpus_args {
                dom: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                cpumap: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                state: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                flags: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_set_interface_parameters_args {
    fn unpack(
        input: &mut In,
    ) -> xdr_codec::Result<(remote_domain_set_interface_parameters_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_set_interface_parameters_args {
                dom: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                device: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                params: {
                    let (v, fsz) = xdr_codec::unpack_flex(
                        input,
                        Some(REMOTE_DOMAIN_INTERFACE_PARAMETERS_MAX as usize),
                    )?;
                    sz += fsz;
                    v
                },
                flags: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_set_max_memory_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_domain_set_max_memory_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_set_max_memory_args {
                dom: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                memory: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_set_memory_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_domain_set_memory_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_set_memory_args {
                dom: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                memory: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_set_memory_flags_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_domain_set_memory_flags_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_set_memory_flags_args {
                dom: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                memory: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                flags: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_set_memory_parameters_args {
    fn unpack(
        input: &mut In,
    ) -> xdr_codec::Result<(remote_domain_set_memory_parameters_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_set_memory_parameters_args {
                dom: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                params: {
                    let (v, fsz) = xdr_codec::unpack_flex(
                        input,
                        Some(REMOTE_DOMAIN_MEMORY_PARAMETERS_MAX as usize),
                    )?;
                    sz += fsz;
                    v
                },
                flags: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_set_memory_stats_period_args {
    fn unpack(
        input: &mut In,
    ) -> xdr_codec::Result<(remote_domain_set_memory_stats_period_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_set_memory_stats_period_args {
                dom: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                period: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                flags: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_set_metadata_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_domain_set_metadata_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_set_metadata_args {
                dom: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                type_: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                metadata: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                key: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                uri: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                flags: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_set_numa_parameters_args {
    fn unpack(
        input: &mut In,
    ) -> xdr_codec::Result<(remote_domain_set_numa_parameters_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_set_numa_parameters_args {
                dom: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                params: {
                    let (v, fsz) = xdr_codec::unpack_flex(
                        input,
                        Some(REMOTE_DOMAIN_NUMA_PARAMETERS_MAX as usize),
                    )?;
                    sz += fsz;
                    v
                },
                flags: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_set_perf_events_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_domain_set_perf_events_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_set_perf_events_args {
                dom: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                params: {
                    let (v, fsz) = xdr_codec::unpack_flex(
                        input,
                        Some(REMOTE_DOMAIN_PERF_EVENTS_MAX as usize),
                    )?;
                    sz += fsz;
                    v
                },
                flags: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_set_scheduler_parameters_args {
    fn unpack(
        input: &mut In,
    ) -> xdr_codec::Result<(remote_domain_set_scheduler_parameters_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_set_scheduler_parameters_args {
                dom: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                params: {
                    let (v, fsz) = xdr_codec::unpack_flex(
                        input,
                        Some(REMOTE_DOMAIN_SCHEDULER_PARAMETERS_MAX as usize),
                    )?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In>
    for remote_domain_set_scheduler_parameters_flags_args
{
    fn unpack(
        input: &mut In,
    ) -> xdr_codec::Result<(remote_domain_set_scheduler_parameters_flags_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_set_scheduler_parameters_flags_args {
                dom: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                params: {
                    let (v, fsz) = xdr_codec::unpack_flex(
                        input,
                        Some(REMOTE_DOMAIN_SCHEDULER_PARAMETERS_MAX as usize),
                    )?;
                    sz += fsz;
                    v
                },
                flags: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_set_time_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_domain_set_time_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_set_time_args {
                dom: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                seconds: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                nseconds: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                flags: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_set_user_password_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_domain_set_user_password_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_set_user_password_args {
                dom: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                user: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                password: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                flags: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_set_vcpu_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_domain_set_vcpu_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_set_vcpu_args {
                dom: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                cpumap: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                state: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                flags: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_set_vcpus_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_domain_set_vcpus_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_set_vcpus_args {
                dom: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                nvcpus: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_set_vcpus_flags_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_domain_set_vcpus_flags_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_set_vcpus_flags_args {
                dom: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                nvcpus: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                flags: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_shutdown_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_domain_shutdown_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_shutdown_args {
                dom: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_shutdown_flags_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_domain_shutdown_flags_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_shutdown_flags_args {
                dom: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                flags: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_snapshot_create_xml_args {
    fn unpack(
        input: &mut In,
    ) -> xdr_codec::Result<(remote_domain_snapshot_create_xml_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_snapshot_create_xml_args {
                dom: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                xml_desc: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                flags: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_snapshot_create_xml_ret {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_domain_snapshot_create_xml_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_snapshot_create_xml_ret {
                snap: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_snapshot_current_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_domain_snapshot_current_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_snapshot_current_args {
                dom: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                flags: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_snapshot_current_ret {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_domain_snapshot_current_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_snapshot_current_ret {
                snap: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_snapshot_delete_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_domain_snapshot_delete_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_snapshot_delete_args {
                snap: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                flags: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_snapshot_get_parent_args {
    fn unpack(
        input: &mut In,
    ) -> xdr_codec::Result<(remote_domain_snapshot_get_parent_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_snapshot_get_parent_args {
                snap: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                flags: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_snapshot_get_parent_ret {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_domain_snapshot_get_parent_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_snapshot_get_parent_ret {
                snap: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_snapshot_get_xml_desc_args {
    fn unpack(
        input: &mut In,
    ) -> xdr_codec::Result<(remote_domain_snapshot_get_xml_desc_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_snapshot_get_xml_desc_args {
                snap: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                flags: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_snapshot_get_xml_desc_ret {
    fn unpack(
        input: &mut In,
    ) -> xdr_codec::Result<(remote_domain_snapshot_get_xml_desc_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_snapshot_get_xml_desc_ret {
                xml: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_snapshot_has_metadata_args {
    fn unpack(
        input: &mut In,
    ) -> xdr_codec::Result<(remote_domain_snapshot_has_metadata_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_snapshot_has_metadata_args {
                snap: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                flags: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_snapshot_has_metadata_ret {
    fn unpack(
        input: &mut In,
    ) -> xdr_codec::Result<(remote_domain_snapshot_has_metadata_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_snapshot_has_metadata_ret {
                metadata: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_snapshot_is_current_args {
    fn unpack(
        input: &mut In,
    ) -> xdr_codec::Result<(remote_domain_snapshot_is_current_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_snapshot_is_current_args {
                snap: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                flags: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_snapshot_is_current_ret {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_domain_snapshot_is_current_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_snapshot_is_current_ret {
                current: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_snapshot_list_all_children_args {
    fn unpack(
        input: &mut In,
    ) -> xdr_codec::Result<(remote_domain_snapshot_list_all_children_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_snapshot_list_all_children_args {
                snapshot: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                need_results: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                flags: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_snapshot_list_all_children_ret {
    fn unpack(
        input: &mut In,
    ) -> xdr_codec::Result<(remote_domain_snapshot_list_all_children_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_snapshot_list_all_children_ret {
                snapshots: {
                    let (v, fsz) = xdr_codec::unpack_flex(
                        input,
                        Some(REMOTE_DOMAIN_SNAPSHOT_LIST_MAX as usize),
                    )?;
                    sz += fsz;
                    v
                },
                ret: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In>
    for remote_domain_snapshot_list_children_names_args
{
    fn unpack(
        input: &mut In,
    ) -> xdr_codec::Result<(remote_domain_snapshot_list_children_names_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_snapshot_list_children_names_args {
                snap: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                maxnames: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                flags: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_snapshot_list_children_names_ret {
    fn unpack(
        input: &mut In,
    ) -> xdr_codec::Result<(remote_domain_snapshot_list_children_names_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_snapshot_list_children_names_ret {
                names: {
                    let (v, fsz) = xdr_codec::unpack_flex(
                        input,
                        Some(REMOTE_DOMAIN_SNAPSHOT_LIST_MAX as usize),
                    )?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_snapshot_list_names_args {
    fn unpack(
        input: &mut In,
    ) -> xdr_codec::Result<(remote_domain_snapshot_list_names_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_snapshot_list_names_args {
                dom: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                maxnames: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                flags: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_snapshot_list_names_ret {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_domain_snapshot_list_names_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_snapshot_list_names_ret {
                names: {
                    let (v, fsz) = xdr_codec::unpack_flex(
                        input,
                        Some(REMOTE_DOMAIN_SNAPSHOT_LIST_MAX as usize),
                    )?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_snapshot_lookup_by_name_args {
    fn unpack(
        input: &mut In,
    ) -> xdr_codec::Result<(remote_domain_snapshot_lookup_by_name_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_snapshot_lookup_by_name_args {
                dom: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                name: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                flags: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_snapshot_lookup_by_name_ret {
    fn unpack(
        input: &mut In,
    ) -> xdr_codec::Result<(remote_domain_snapshot_lookup_by_name_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_snapshot_lookup_by_name_ret {
                snap: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_snapshot_num_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_domain_snapshot_num_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_snapshot_num_args {
                dom: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                flags: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_snapshot_num_children_args {
    fn unpack(
        input: &mut In,
    ) -> xdr_codec::Result<(remote_domain_snapshot_num_children_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_snapshot_num_children_args {
                snap: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                flags: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_snapshot_num_children_ret {
    fn unpack(
        input: &mut In,
    ) -> xdr_codec::Result<(remote_domain_snapshot_num_children_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_snapshot_num_children_ret {
                num: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_snapshot_num_ret {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_domain_snapshot_num_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_snapshot_num_ret {
                num: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_stats_record {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_domain_stats_record, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_stats_record {
                dom: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                params: {
                    let (v, fsz) = xdr_codec::unpack_flex(
                        input,
                        Some(REMOTE_CONNECT_GET_ALL_DOMAIN_STATS_MAX as usize),
                    )?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_suspend_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_domain_suspend_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_suspend_args {
                dom: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_undefine_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_domain_undefine_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_undefine_args {
                dom: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_undefine_flags_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_domain_undefine_flags_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_undefine_flags_args {
                dom: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                flags: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_domain_update_device_flags_args {
    fn unpack(
        input: &mut In,
    ) -> xdr_codec::Result<(remote_domain_update_device_flags_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_domain_update_device_flags_args {
                dom: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                xml: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                flags: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_error {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_error, usize)> {
        let mut sz = 0;
        Ok((
            remote_error {
                code: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                domain: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                message: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                level: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                dom: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                str1: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                str2: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                str3: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                int1: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                int2: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                net: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_interface_change_begin_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_interface_change_begin_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_interface_change_begin_args {
                flags: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_interface_change_commit_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_interface_change_commit_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_interface_change_commit_args {
                flags: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_interface_change_rollback_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_interface_change_rollback_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_interface_change_rollback_args {
                flags: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_interface_create_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_interface_create_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_interface_create_args {
                iface: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                flags: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_interface_define_xml_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_interface_define_xml_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_interface_define_xml_args {
                xml: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                flags: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_interface_define_xml_ret {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_interface_define_xml_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_interface_define_xml_ret {
                iface: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_interface_destroy_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_interface_destroy_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_interface_destroy_args {
                iface: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                flags: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_interface_get_xml_desc_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_interface_get_xml_desc_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_interface_get_xml_desc_args {
                iface: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                flags: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_interface_get_xml_desc_ret {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_interface_get_xml_desc_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_interface_get_xml_desc_ret {
                xml: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_interface_is_active_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_interface_is_active_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_interface_is_active_args {
                iface: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_interface_is_active_ret {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_interface_is_active_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_interface_is_active_ret {
                active: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_interface_lookup_by_mac_string_args {
    fn unpack(
        input: &mut In,
    ) -> xdr_codec::Result<(remote_interface_lookup_by_mac_string_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_interface_lookup_by_mac_string_args {
                mac: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_interface_lookup_by_mac_string_ret {
    fn unpack(
        input: &mut In,
    ) -> xdr_codec::Result<(remote_interface_lookup_by_mac_string_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_interface_lookup_by_mac_string_ret {
                iface: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_interface_lookup_by_name_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_interface_lookup_by_name_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_interface_lookup_by_name_args {
                name: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_interface_lookup_by_name_ret {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_interface_lookup_by_name_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_interface_lookup_by_name_ret {
                iface: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_interface_undefine_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_interface_undefine_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_interface_undefine_args {
                iface: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_network_create_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_network_create_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_network_create_args {
                net: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_network_create_xml_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_network_create_xml_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_network_create_xml_args {
                xml: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_network_create_xml_ret {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_network_create_xml_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_network_create_xml_ret {
                net: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_network_define_xml_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_network_define_xml_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_network_define_xml_args {
                xml: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_network_define_xml_ret {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_network_define_xml_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_network_define_xml_ret {
                net: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_network_destroy_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_network_destroy_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_network_destroy_args {
                net: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_network_dhcp_lease {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_network_dhcp_lease, usize)> {
        let mut sz = 0;
        Ok((
            remote_network_dhcp_lease {
                iface: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                expirytime: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                type_: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                mac: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                iaid: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                ipaddr: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                prefix: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                hostname: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                clientid: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_network_event_lifecycle_msg {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_network_event_lifecycle_msg, usize)> {
        let mut sz = 0;
        Ok((
            remote_network_event_lifecycle_msg {
                callbackID: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                net: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                event: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                detail: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_network_get_autostart_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_network_get_autostart_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_network_get_autostart_args {
                net: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_network_get_autostart_ret {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_network_get_autostart_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_network_get_autostart_ret {
                autostart: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_network_get_bridge_name_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_network_get_bridge_name_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_network_get_bridge_name_args {
                net: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_network_get_bridge_name_ret {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_network_get_bridge_name_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_network_get_bridge_name_ret {
                name: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_network_get_dhcp_leases_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_network_get_dhcp_leases_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_network_get_dhcp_leases_args {
                net: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                mac: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                need_results: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                flags: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_network_get_dhcp_leases_ret {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_network_get_dhcp_leases_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_network_get_dhcp_leases_ret {
                leases: {
                    let (v, fsz) = xdr_codec::unpack_flex(
                        input,
                        Some(REMOTE_NETWORK_DHCP_LEASES_MAX as usize),
                    )?;
                    sz += fsz;
                    v
                },
                ret: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_network_get_xml_desc_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_network_get_xml_desc_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_network_get_xml_desc_args {
                net: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                flags: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_network_get_xml_desc_ret {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_network_get_xml_desc_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_network_get_xml_desc_ret {
                xml: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_network_is_active_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_network_is_active_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_network_is_active_args {
                net: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_network_is_active_ret {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_network_is_active_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_network_is_active_ret {
                active: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_network_is_persistent_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_network_is_persistent_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_network_is_persistent_args {
                net: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_network_is_persistent_ret {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_network_is_persistent_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_network_is_persistent_ret {
                persistent: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_network_lookup_by_name_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_network_lookup_by_name_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_network_lookup_by_name_args {
                name: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_network_lookup_by_name_ret {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_network_lookup_by_name_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_network_lookup_by_name_ret {
                net: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_network_lookup_by_uuid_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_network_lookup_by_uuid_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_network_lookup_by_uuid_args {
                uuid: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_network_lookup_by_uuid_ret {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_network_lookup_by_uuid_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_network_lookup_by_uuid_ret {
                net: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_network_set_autostart_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_network_set_autostart_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_network_set_autostart_args {
                net: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                autostart: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_network_undefine_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_network_undefine_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_network_undefine_args {
                net: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_network_update_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_network_update_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_network_update_args {
                net: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                command: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                section: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                parentIndex: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                xml: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                flags: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_node_alloc_pages_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_node_alloc_pages_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_node_alloc_pages_args {
                pageSizes: {
                    let (v, fsz) =
                        xdr_codec::unpack_flex(input, Some(REMOTE_NODE_MAX_CELLS as usize))?;
                    sz += fsz;
                    v
                },
                pageCounts: {
                    let (v, fsz) =
                        xdr_codec::unpack_flex(input, Some(REMOTE_NODE_MAX_CELLS as usize))?;
                    sz += fsz;
                    v
                },
                startCell: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                cellCount: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                flags: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_node_alloc_pages_ret {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_node_alloc_pages_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_node_alloc_pages_ret {
                ret: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_node_device_create_xml_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_node_device_create_xml_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_node_device_create_xml_args {
                xml_desc: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                flags: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_node_device_create_xml_ret {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_node_device_create_xml_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_node_device_create_xml_ret {
                dev: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_node_device_destroy_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_node_device_destroy_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_node_device_destroy_args {
                name: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_node_device_detach_flags_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_node_device_detach_flags_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_node_device_detach_flags_args {
                name: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                driverName: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                flags: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_node_device_dettach_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_node_device_dettach_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_node_device_dettach_args {
                name: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_node_device_event_lifecycle_msg {
    fn unpack(
        input: &mut In,
    ) -> xdr_codec::Result<(remote_node_device_event_lifecycle_msg, usize)> {
        let mut sz = 0;
        Ok((
            remote_node_device_event_lifecycle_msg {
                callbackID: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                dev: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                event: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                detail: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_node_device_event_update_msg {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_node_device_event_update_msg, usize)> {
        let mut sz = 0;
        Ok((
            remote_node_device_event_update_msg {
                callbackID: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                dev: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_node_device_get_parent_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_node_device_get_parent_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_node_device_get_parent_args {
                name: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_node_device_get_parent_ret {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_node_device_get_parent_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_node_device_get_parent_ret {
                parent: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_node_device_get_xml_desc_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_node_device_get_xml_desc_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_node_device_get_xml_desc_args {
                name: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                flags: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_node_device_get_xml_desc_ret {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_node_device_get_xml_desc_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_node_device_get_xml_desc_ret {
                xml: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_node_device_list_caps_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_node_device_list_caps_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_node_device_list_caps_args {
                name: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                maxnames: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_node_device_list_caps_ret {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_node_device_list_caps_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_node_device_list_caps_ret {
                names: {
                    let (v, fsz) = xdr_codec::unpack_flex(
                        input,
                        Some(REMOTE_NODE_DEVICE_CAPS_LIST_MAX as usize),
                    )?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_node_device_lookup_by_name_args {
    fn unpack(
        input: &mut In,
    ) -> xdr_codec::Result<(remote_node_device_lookup_by_name_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_node_device_lookup_by_name_args {
                name: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_node_device_lookup_by_name_ret {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_node_device_lookup_by_name_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_node_device_lookup_by_name_ret {
                dev: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In>
    for remote_node_device_lookup_scsi_host_by_wwn_args
{
    fn unpack(
        input: &mut In,
    ) -> xdr_codec::Result<(remote_node_device_lookup_scsi_host_by_wwn_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_node_device_lookup_scsi_host_by_wwn_args {
                wwnn: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                wwpn: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                flags: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_node_device_lookup_scsi_host_by_wwn_ret {
    fn unpack(
        input: &mut In,
    ) -> xdr_codec::Result<(remote_node_device_lookup_scsi_host_by_wwn_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_node_device_lookup_scsi_host_by_wwn_ret {
                dev: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_node_device_num_of_caps_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_node_device_num_of_caps_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_node_device_num_of_caps_args {
                name: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_node_device_num_of_caps_ret {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_node_device_num_of_caps_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_node_device_num_of_caps_ret {
                num: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_node_device_re_attach_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_node_device_re_attach_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_node_device_re_attach_args {
                name: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_node_device_reset_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_node_device_reset_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_node_device_reset_args {
                name: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_node_get_cells_free_memory_args {
    fn unpack(
        input: &mut In,
    ) -> xdr_codec::Result<(remote_node_get_cells_free_memory_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_node_get_cells_free_memory_args {
                startCell: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                maxcells: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_node_get_cells_free_memory_ret {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_node_get_cells_free_memory_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_node_get_cells_free_memory_ret {
                cells: {
                    let (v, fsz) =
                        xdr_codec::unpack_flex(input, Some(REMOTE_NODE_MAX_CELLS as usize))?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_node_get_cpu_map_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_node_get_cpu_map_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_node_get_cpu_map_args {
                need_map: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                need_online: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                flags: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_node_get_cpu_map_ret {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_node_get_cpu_map_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_node_get_cpu_map_ret {
                cpumap: {
                    let (v, fsz) =
                        xdr_codec::unpack_opaque_flex(input, Some(REMOTE_CPUMAP_MAX as usize))?;
                    sz += fsz;
                    v
                },
                online: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                ret: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_node_get_cpu_stats {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_node_get_cpu_stats, usize)> {
        let mut sz = 0;
        Ok((
            remote_node_get_cpu_stats {
                field: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                value: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_node_get_cpu_stats_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_node_get_cpu_stats_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_node_get_cpu_stats_args {
                cpuNum: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                nparams: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                flags: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_node_get_cpu_stats_ret {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_node_get_cpu_stats_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_node_get_cpu_stats_ret {
                params: {
                    let (v, fsz) =
                        xdr_codec::unpack_flex(input, Some(REMOTE_NODE_CPU_STATS_MAX as usize))?;
                    sz += fsz;
                    v
                },
                nparams: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_node_get_free_memory_ret {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_node_get_free_memory_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_node_get_free_memory_ret {
                freeMem: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_node_get_free_pages_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_node_get_free_pages_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_node_get_free_pages_args {
                pages: {
                    let (v, fsz) =
                        xdr_codec::unpack_flex(input, Some(REMOTE_NODE_MAX_CELLS as usize))?;
                    sz += fsz;
                    v
                },
                startCell: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                cellCount: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                flags: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_node_get_free_pages_ret {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_node_get_free_pages_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_node_get_free_pages_ret {
                counts: {
                    let (v, fsz) =
                        xdr_codec::unpack_flex(input, Some(REMOTE_NODE_MAX_CELLS as usize))?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_node_get_info_ret {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_node_get_info_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_node_get_info_ret {
                model: {
                    let (v, fsz) = {
                        let mut buf: [i8; 32i64 as usize] = [0; 32];
                        let sz =
                            xdr_codec::unpack_array(input, &mut buf[..], 32i64 as usize, None)?;
                        (buf, sz)
                    };
                    sz += fsz;
                    v
                },
                memory: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                cpus: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                mhz: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                nodes: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                sockets: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                cores: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                threads: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_node_get_memory_parameters_args {
    fn unpack(
        input: &mut In,
    ) -> xdr_codec::Result<(remote_node_get_memory_parameters_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_node_get_memory_parameters_args {
                nparams: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                flags: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_node_get_memory_parameters_ret {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_node_get_memory_parameters_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_node_get_memory_parameters_ret {
                params: {
                    let (v, fsz) = xdr_codec::unpack_flex(
                        input,
                        Some(REMOTE_NODE_MEMORY_PARAMETERS_MAX as usize),
                    )?;
                    sz += fsz;
                    v
                },
                nparams: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_node_get_memory_stats {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_node_get_memory_stats, usize)> {
        let mut sz = 0;
        Ok((
            remote_node_get_memory_stats {
                field: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                value: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_node_get_memory_stats_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_node_get_memory_stats_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_node_get_memory_stats_args {
                nparams: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                cellNum: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                flags: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_node_get_memory_stats_ret {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_node_get_memory_stats_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_node_get_memory_stats_ret {
                params: {
                    let (v, fsz) =
                        xdr_codec::unpack_flex(input, Some(REMOTE_NODE_MEMORY_STATS_MAX as usize))?;
                    sz += fsz;
                    v
                },
                nparams: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_node_get_security_model_ret {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_node_get_security_model_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_node_get_security_model_ret {
                model: {
                    let (v, fsz) =
                        xdr_codec::unpack_flex(input, Some(REMOTE_SECURITY_MODEL_MAX as usize))?;
                    sz += fsz;
                    v
                },
                doi: {
                    let (v, fsz) =
                        xdr_codec::unpack_flex(input, Some(REMOTE_SECURITY_DOI_MAX as usize))?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_node_list_devices_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_node_list_devices_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_node_list_devices_args {
                cap: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                maxnames: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                flags: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_node_list_devices_ret {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_node_list_devices_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_node_list_devices_ret {
                names: {
                    let (v, fsz) =
                        xdr_codec::unpack_flex(input, Some(REMOTE_NODE_DEVICE_LIST_MAX as usize))?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_node_num_of_devices_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_node_num_of_devices_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_node_num_of_devices_args {
                cap: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                flags: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_node_num_of_devices_ret {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_node_num_of_devices_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_node_num_of_devices_ret {
                num: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_node_set_memory_parameters_args {
    fn unpack(
        input: &mut In,
    ) -> xdr_codec::Result<(remote_node_set_memory_parameters_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_node_set_memory_parameters_args {
                params: {
                    let (v, fsz) = xdr_codec::unpack_flex(
                        input,
                        Some(REMOTE_NODE_MEMORY_PARAMETERS_MAX as usize),
                    )?;
                    sz += fsz;
                    v
                },
                flags: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_node_suspend_for_duration_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_node_suspend_for_duration_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_node_suspend_for_duration_args {
                target: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                duration: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                flags: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_nonnull_domain {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_nonnull_domain, usize)> {
        let mut sz = 0;
        Ok((
            remote_nonnull_domain {
                name: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                uuid: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                id: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_nonnull_domain_snapshot {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_nonnull_domain_snapshot, usize)> {
        let mut sz = 0;
        Ok((
            remote_nonnull_domain_snapshot {
                name: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                dom: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_nonnull_interface {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_nonnull_interface, usize)> {
        let mut sz = 0;
        Ok((
            remote_nonnull_interface {
                name: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                mac: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_nonnull_network {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_nonnull_network, usize)> {
        let mut sz = 0;
        Ok((
            remote_nonnull_network {
                name: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                uuid: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_nonnull_node_device {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_nonnull_node_device, usize)> {
        let mut sz = 0;
        Ok((
            remote_nonnull_node_device {
                name: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_nonnull_nwfilter {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_nonnull_nwfilter, usize)> {
        let mut sz = 0;
        Ok((
            remote_nonnull_nwfilter {
                name: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                uuid: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_nonnull_secret {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_nonnull_secret, usize)> {
        let mut sz = 0;
        Ok((
            remote_nonnull_secret {
                uuid: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                usageType: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                usageID: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_nonnull_storage_pool {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_nonnull_storage_pool, usize)> {
        let mut sz = 0;
        Ok((
            remote_nonnull_storage_pool {
                name: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                uuid: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_nonnull_storage_vol {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_nonnull_storage_vol, usize)> {
        let mut sz = 0;
        Ok((
            remote_nonnull_storage_vol {
                pool: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                name: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                key: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_nonnull_string {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_nonnull_string, usize)> {
        let mut sz = 0;
        Ok((
            {
                let (v, usz) = xdr_codec::unpack_string(input, Some(REMOTE_STRING_MAX as usize))?;
                sz = usz;
                remote_nonnull_string(v)
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_nwfilter_define_xml_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_nwfilter_define_xml_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_nwfilter_define_xml_args {
                xml: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_nwfilter_define_xml_ret {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_nwfilter_define_xml_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_nwfilter_define_xml_ret {
                nwfilter: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_nwfilter_get_xml_desc_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_nwfilter_get_xml_desc_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_nwfilter_get_xml_desc_args {
                nwfilter: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                flags: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_nwfilter_get_xml_desc_ret {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_nwfilter_get_xml_desc_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_nwfilter_get_xml_desc_ret {
                xml: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_nwfilter_lookup_by_name_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_nwfilter_lookup_by_name_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_nwfilter_lookup_by_name_args {
                name: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_nwfilter_lookup_by_name_ret {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_nwfilter_lookup_by_name_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_nwfilter_lookup_by_name_ret {
                nwfilter: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_nwfilter_lookup_by_uuid_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_nwfilter_lookup_by_uuid_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_nwfilter_lookup_by_uuid_args {
                uuid: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_nwfilter_lookup_by_uuid_ret {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_nwfilter_lookup_by_uuid_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_nwfilter_lookup_by_uuid_ret {
                nwfilter: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_nwfilter_undefine_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_nwfilter_undefine_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_nwfilter_undefine_args {
                nwfilter: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_procedure {
    #[inline]
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_procedure, usize)> {
        let mut sz = 0;
        Ok((
            {
                let (e, esz): (i32, _) = xdr_codec::Unpack::unpack(input)?;
                sz += esz;
                match e { x if x == remote_procedure :: REMOTE_PROC_CONNECT_OPEN as i32 => remote_procedure :: REMOTE_PROC_CONNECT_OPEN , x if x == remote_procedure :: REMOTE_PROC_CONNECT_CLOSE as i32 => remote_procedure :: REMOTE_PROC_CONNECT_CLOSE , x if x == remote_procedure :: REMOTE_PROC_CONNECT_GET_TYPE as i32 => remote_procedure :: REMOTE_PROC_CONNECT_GET_TYPE , x if x == remote_procedure :: REMOTE_PROC_CONNECT_GET_VERSION as i32 => remote_procedure :: REMOTE_PROC_CONNECT_GET_VERSION , x if x == remote_procedure :: REMOTE_PROC_CONNECT_GET_MAX_VCPUS as i32 => remote_procedure :: REMOTE_PROC_CONNECT_GET_MAX_VCPUS , x if x == remote_procedure :: REMOTE_PROC_NODE_GET_INFO as i32 => remote_procedure :: REMOTE_PROC_NODE_GET_INFO , x if x == remote_procedure :: REMOTE_PROC_CONNECT_GET_CAPABILITIES as i32 => remote_procedure :: REMOTE_PROC_CONNECT_GET_CAPABILITIES , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_ATTACH_DEVICE as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_ATTACH_DEVICE , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_CREATE as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_CREATE , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_CREATE_XML as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_CREATE_XML , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_DEFINE_XML as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_DEFINE_XML , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_DESTROY as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_DESTROY , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_DETACH_DEVICE as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_DETACH_DEVICE , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_GET_XML_DESC as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_GET_XML_DESC , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_GET_AUTOSTART as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_GET_AUTOSTART , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_GET_INFO as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_GET_INFO , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_GET_MAX_MEMORY as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_GET_MAX_MEMORY , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_GET_MAX_VCPUS as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_GET_MAX_VCPUS , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_GET_OS_TYPE as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_GET_OS_TYPE , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_GET_VCPUS as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_GET_VCPUS , x if x == remote_procedure :: REMOTE_PROC_CONNECT_LIST_DEFINED_DOMAINS as i32 => remote_procedure :: REMOTE_PROC_CONNECT_LIST_DEFINED_DOMAINS , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_LOOKUP_BY_ID as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_LOOKUP_BY_ID , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_LOOKUP_BY_NAME as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_LOOKUP_BY_NAME , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_LOOKUP_BY_UUID as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_LOOKUP_BY_UUID , x if x == remote_procedure :: REMOTE_PROC_CONNECT_NUM_OF_DEFINED_DOMAINS as i32 => remote_procedure :: REMOTE_PROC_CONNECT_NUM_OF_DEFINED_DOMAINS , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_PIN_VCPU as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_PIN_VCPU , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_REBOOT as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_REBOOT , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_RESUME as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_RESUME , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_SET_AUTOSTART as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_SET_AUTOSTART , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_SET_MAX_MEMORY as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_SET_MAX_MEMORY , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_SET_MEMORY as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_SET_MEMORY , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_SET_VCPUS as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_SET_VCPUS , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_SHUTDOWN as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_SHUTDOWN , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_SUSPEND as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_SUSPEND , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_UNDEFINE as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_UNDEFINE , x if x == remote_procedure :: REMOTE_PROC_CONNECT_LIST_DEFINED_NETWORKS as i32 => remote_procedure :: REMOTE_PROC_CONNECT_LIST_DEFINED_NETWORKS , x if x == remote_procedure :: REMOTE_PROC_CONNECT_LIST_DOMAINS as i32 => remote_procedure :: REMOTE_PROC_CONNECT_LIST_DOMAINS , x if x == remote_procedure :: REMOTE_PROC_CONNECT_LIST_NETWORKS as i32 => remote_procedure :: REMOTE_PROC_CONNECT_LIST_NETWORKS , x if x == remote_procedure :: REMOTE_PROC_NETWORK_CREATE as i32 => remote_procedure :: REMOTE_PROC_NETWORK_CREATE , x if x == remote_procedure :: REMOTE_PROC_NETWORK_CREATE_XML as i32 => remote_procedure :: REMOTE_PROC_NETWORK_CREATE_XML , x if x == remote_procedure :: REMOTE_PROC_NETWORK_DEFINE_XML as i32 => remote_procedure :: REMOTE_PROC_NETWORK_DEFINE_XML , x if x == remote_procedure :: REMOTE_PROC_NETWORK_DESTROY as i32 => remote_procedure :: REMOTE_PROC_NETWORK_DESTROY , x if x == remote_procedure :: REMOTE_PROC_NETWORK_GET_XML_DESC as i32 => remote_procedure :: REMOTE_PROC_NETWORK_GET_XML_DESC , x if x == remote_procedure :: REMOTE_PROC_NETWORK_GET_AUTOSTART as i32 => remote_procedure :: REMOTE_PROC_NETWORK_GET_AUTOSTART , x if x == remote_procedure :: REMOTE_PROC_NETWORK_GET_BRIDGE_NAME as i32 => remote_procedure :: REMOTE_PROC_NETWORK_GET_BRIDGE_NAME , x if x == remote_procedure :: REMOTE_PROC_NETWORK_LOOKUP_BY_NAME as i32 => remote_procedure :: REMOTE_PROC_NETWORK_LOOKUP_BY_NAME , x if x == remote_procedure :: REMOTE_PROC_NETWORK_LOOKUP_BY_UUID as i32 => remote_procedure :: REMOTE_PROC_NETWORK_LOOKUP_BY_UUID , x if x == remote_procedure :: REMOTE_PROC_NETWORK_SET_AUTOSTART as i32 => remote_procedure :: REMOTE_PROC_NETWORK_SET_AUTOSTART , x if x == remote_procedure :: REMOTE_PROC_NETWORK_UNDEFINE as i32 => remote_procedure :: REMOTE_PROC_NETWORK_UNDEFINE , x if x == remote_procedure :: REMOTE_PROC_CONNECT_NUM_OF_DEFINED_NETWORKS as i32 => remote_procedure :: REMOTE_PROC_CONNECT_NUM_OF_DEFINED_NETWORKS , x if x == remote_procedure :: REMOTE_PROC_CONNECT_NUM_OF_DOMAINS as i32 => remote_procedure :: REMOTE_PROC_CONNECT_NUM_OF_DOMAINS , x if x == remote_procedure :: REMOTE_PROC_CONNECT_NUM_OF_NETWORKS as i32 => remote_procedure :: REMOTE_PROC_CONNECT_NUM_OF_NETWORKS , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_CORE_DUMP as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_CORE_DUMP , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_RESTORE as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_RESTORE , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_SAVE as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_SAVE , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_GET_SCHEDULER_TYPE as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_GET_SCHEDULER_TYPE , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_GET_SCHEDULER_PARAMETERS as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_GET_SCHEDULER_PARAMETERS , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_SET_SCHEDULER_PARAMETERS as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_SET_SCHEDULER_PARAMETERS , x if x == remote_procedure :: REMOTE_PROC_CONNECT_GET_HOSTNAME as i32 => remote_procedure :: REMOTE_PROC_CONNECT_GET_HOSTNAME , x if x == remote_procedure :: REMOTE_PROC_CONNECT_SUPPORTS_FEATURE as i32 => remote_procedure :: REMOTE_PROC_CONNECT_SUPPORTS_FEATURE , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_MIGRATE_PREPARE as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_MIGRATE_PREPARE , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_MIGRATE_PERFORM as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_MIGRATE_PERFORM , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_MIGRATE_FINISH as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_MIGRATE_FINISH , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_BLOCK_STATS as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_BLOCK_STATS , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_INTERFACE_STATS as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_INTERFACE_STATS , x if x == remote_procedure :: REMOTE_PROC_AUTH_LIST as i32 => remote_procedure :: REMOTE_PROC_AUTH_LIST , x if x == remote_procedure :: REMOTE_PROC_AUTH_SASL_INIT as i32 => remote_procedure :: REMOTE_PROC_AUTH_SASL_INIT , x if x == remote_procedure :: REMOTE_PROC_AUTH_SASL_START as i32 => remote_procedure :: REMOTE_PROC_AUTH_SASL_START , x if x == remote_procedure :: REMOTE_PROC_AUTH_SASL_STEP as i32 => remote_procedure :: REMOTE_PROC_AUTH_SASL_STEP , x if x == remote_procedure :: REMOTE_PROC_AUTH_POLKIT as i32 => remote_procedure :: REMOTE_PROC_AUTH_POLKIT , x if x == remote_procedure :: REMOTE_PROC_CONNECT_NUM_OF_STORAGE_POOLS as i32 => remote_procedure :: REMOTE_PROC_CONNECT_NUM_OF_STORAGE_POOLS , x if x == remote_procedure :: REMOTE_PROC_CONNECT_LIST_STORAGE_POOLS as i32 => remote_procedure :: REMOTE_PROC_CONNECT_LIST_STORAGE_POOLS , x if x == remote_procedure :: REMOTE_PROC_CONNECT_NUM_OF_DEFINED_STORAGE_POOLS as i32 => remote_procedure :: REMOTE_PROC_CONNECT_NUM_OF_DEFINED_STORAGE_POOLS , x if x == remote_procedure :: REMOTE_PROC_CONNECT_LIST_DEFINED_STORAGE_POOLS as i32 => remote_procedure :: REMOTE_PROC_CONNECT_LIST_DEFINED_STORAGE_POOLS , x if x == remote_procedure :: REMOTE_PROC_CONNECT_FIND_STORAGE_POOL_SOURCES as i32 => remote_procedure :: REMOTE_PROC_CONNECT_FIND_STORAGE_POOL_SOURCES , x if x == remote_procedure :: REMOTE_PROC_STORAGE_POOL_CREATE_XML as i32 => remote_procedure :: REMOTE_PROC_STORAGE_POOL_CREATE_XML , x if x == remote_procedure :: REMOTE_PROC_STORAGE_POOL_DEFINE_XML as i32 => remote_procedure :: REMOTE_PROC_STORAGE_POOL_DEFINE_XML , x if x == remote_procedure :: REMOTE_PROC_STORAGE_POOL_CREATE as i32 => remote_procedure :: REMOTE_PROC_STORAGE_POOL_CREATE , x if x == remote_procedure :: REMOTE_PROC_STORAGE_POOL_BUILD as i32 => remote_procedure :: REMOTE_PROC_STORAGE_POOL_BUILD , x if x == remote_procedure :: REMOTE_PROC_STORAGE_POOL_DESTROY as i32 => remote_procedure :: REMOTE_PROC_STORAGE_POOL_DESTROY , x if x == remote_procedure :: REMOTE_PROC_STORAGE_POOL_DELETE as i32 => remote_procedure :: REMOTE_PROC_STORAGE_POOL_DELETE , x if x == remote_procedure :: REMOTE_PROC_STORAGE_POOL_UNDEFINE as i32 => remote_procedure :: REMOTE_PROC_STORAGE_POOL_UNDEFINE , x if x == remote_procedure :: REMOTE_PROC_STORAGE_POOL_REFRESH as i32 => remote_procedure :: REMOTE_PROC_STORAGE_POOL_REFRESH , x if x == remote_procedure :: REMOTE_PROC_STORAGE_POOL_LOOKUP_BY_NAME as i32 => remote_procedure :: REMOTE_PROC_STORAGE_POOL_LOOKUP_BY_NAME , x if x == remote_procedure :: REMOTE_PROC_STORAGE_POOL_LOOKUP_BY_UUID as i32 => remote_procedure :: REMOTE_PROC_STORAGE_POOL_LOOKUP_BY_UUID , x if x == remote_procedure :: REMOTE_PROC_STORAGE_POOL_LOOKUP_BY_VOLUME as i32 => remote_procedure :: REMOTE_PROC_STORAGE_POOL_LOOKUP_BY_VOLUME , x if x == remote_procedure :: REMOTE_PROC_STORAGE_POOL_GET_INFO as i32 => remote_procedure :: REMOTE_PROC_STORAGE_POOL_GET_INFO , x if x == remote_procedure :: REMOTE_PROC_STORAGE_POOL_GET_XML_DESC as i32 => remote_procedure :: REMOTE_PROC_STORAGE_POOL_GET_XML_DESC , x if x == remote_procedure :: REMOTE_PROC_STORAGE_POOL_GET_AUTOSTART as i32 => remote_procedure :: REMOTE_PROC_STORAGE_POOL_GET_AUTOSTART , x if x == remote_procedure :: REMOTE_PROC_STORAGE_POOL_SET_AUTOSTART as i32 => remote_procedure :: REMOTE_PROC_STORAGE_POOL_SET_AUTOSTART , x if x == remote_procedure :: REMOTE_PROC_STORAGE_POOL_NUM_OF_VOLUMES as i32 => remote_procedure :: REMOTE_PROC_STORAGE_POOL_NUM_OF_VOLUMES , x if x == remote_procedure :: REMOTE_PROC_STORAGE_POOL_LIST_VOLUMES as i32 => remote_procedure :: REMOTE_PROC_STORAGE_POOL_LIST_VOLUMES , x if x == remote_procedure :: REMOTE_PROC_STORAGE_VOL_CREATE_XML as i32 => remote_procedure :: REMOTE_PROC_STORAGE_VOL_CREATE_XML , x if x == remote_procedure :: REMOTE_PROC_STORAGE_VOL_DELETE as i32 => remote_procedure :: REMOTE_PROC_STORAGE_VOL_DELETE , x if x == remote_procedure :: REMOTE_PROC_STORAGE_VOL_LOOKUP_BY_NAME as i32 => remote_procedure :: REMOTE_PROC_STORAGE_VOL_LOOKUP_BY_NAME , x if x == remote_procedure :: REMOTE_PROC_STORAGE_VOL_LOOKUP_BY_KEY as i32 => remote_procedure :: REMOTE_PROC_STORAGE_VOL_LOOKUP_BY_KEY , x if x == remote_procedure :: REMOTE_PROC_STORAGE_VOL_LOOKUP_BY_PATH as i32 => remote_procedure :: REMOTE_PROC_STORAGE_VOL_LOOKUP_BY_PATH , x if x == remote_procedure :: REMOTE_PROC_STORAGE_VOL_GET_INFO as i32 => remote_procedure :: REMOTE_PROC_STORAGE_VOL_GET_INFO , x if x == remote_procedure :: REMOTE_PROC_STORAGE_VOL_GET_XML_DESC as i32 => remote_procedure :: REMOTE_PROC_STORAGE_VOL_GET_XML_DESC , x if x == remote_procedure :: REMOTE_PROC_STORAGE_VOL_GET_PATH as i32 => remote_procedure :: REMOTE_PROC_STORAGE_VOL_GET_PATH , x if x == remote_procedure :: REMOTE_PROC_NODE_GET_CELLS_FREE_MEMORY as i32 => remote_procedure :: REMOTE_PROC_NODE_GET_CELLS_FREE_MEMORY , x if x == remote_procedure :: REMOTE_PROC_NODE_GET_FREE_MEMORY as i32 => remote_procedure :: REMOTE_PROC_NODE_GET_FREE_MEMORY , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_BLOCK_PEEK as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_BLOCK_PEEK , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_MEMORY_PEEK as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_MEMORY_PEEK , x if x == remote_procedure :: REMOTE_PROC_CONNECT_DOMAIN_EVENT_REGISTER as i32 => remote_procedure :: REMOTE_PROC_CONNECT_DOMAIN_EVENT_REGISTER , x if x == remote_procedure :: REMOTE_PROC_CONNECT_DOMAIN_EVENT_DEREGISTER as i32 => remote_procedure :: REMOTE_PROC_CONNECT_DOMAIN_EVENT_DEREGISTER , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_EVENT_LIFECYCLE as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_EVENT_LIFECYCLE , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_MIGRATE_PREPARE2 as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_MIGRATE_PREPARE2 , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_MIGRATE_FINISH2 as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_MIGRATE_FINISH2 , x if x == remote_procedure :: REMOTE_PROC_CONNECT_GET_URI as i32 => remote_procedure :: REMOTE_PROC_CONNECT_GET_URI , x if x == remote_procedure :: REMOTE_PROC_NODE_NUM_OF_DEVICES as i32 => remote_procedure :: REMOTE_PROC_NODE_NUM_OF_DEVICES , x if x == remote_procedure :: REMOTE_PROC_NODE_LIST_DEVICES as i32 => remote_procedure :: REMOTE_PROC_NODE_LIST_DEVICES , x if x == remote_procedure :: REMOTE_PROC_NODE_DEVICE_LOOKUP_BY_NAME as i32 => remote_procedure :: REMOTE_PROC_NODE_DEVICE_LOOKUP_BY_NAME , x if x == remote_procedure :: REMOTE_PROC_NODE_DEVICE_GET_XML_DESC as i32 => remote_procedure :: REMOTE_PROC_NODE_DEVICE_GET_XML_DESC , x if x == remote_procedure :: REMOTE_PROC_NODE_DEVICE_GET_PARENT as i32 => remote_procedure :: REMOTE_PROC_NODE_DEVICE_GET_PARENT , x if x == remote_procedure :: REMOTE_PROC_NODE_DEVICE_NUM_OF_CAPS as i32 => remote_procedure :: REMOTE_PROC_NODE_DEVICE_NUM_OF_CAPS , x if x == remote_procedure :: REMOTE_PROC_NODE_DEVICE_LIST_CAPS as i32 => remote_procedure :: REMOTE_PROC_NODE_DEVICE_LIST_CAPS , x if x == remote_procedure :: REMOTE_PROC_NODE_DEVICE_DETTACH as i32 => remote_procedure :: REMOTE_PROC_NODE_DEVICE_DETTACH , x if x == remote_procedure :: REMOTE_PROC_NODE_DEVICE_RE_ATTACH as i32 => remote_procedure :: REMOTE_PROC_NODE_DEVICE_RE_ATTACH , x if x == remote_procedure :: REMOTE_PROC_NODE_DEVICE_RESET as i32 => remote_procedure :: REMOTE_PROC_NODE_DEVICE_RESET , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_GET_SECURITY_LABEL as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_GET_SECURITY_LABEL , x if x == remote_procedure :: REMOTE_PROC_NODE_GET_SECURITY_MODEL as i32 => remote_procedure :: REMOTE_PROC_NODE_GET_SECURITY_MODEL , x if x == remote_procedure :: REMOTE_PROC_NODE_DEVICE_CREATE_XML as i32 => remote_procedure :: REMOTE_PROC_NODE_DEVICE_CREATE_XML , x if x == remote_procedure :: REMOTE_PROC_NODE_DEVICE_DESTROY as i32 => remote_procedure :: REMOTE_PROC_NODE_DEVICE_DESTROY , x if x == remote_procedure :: REMOTE_PROC_STORAGE_VOL_CREATE_XML_FROM as i32 => remote_procedure :: REMOTE_PROC_STORAGE_VOL_CREATE_XML_FROM , x if x == remote_procedure :: REMOTE_PROC_CONNECT_NUM_OF_INTERFACES as i32 => remote_procedure :: REMOTE_PROC_CONNECT_NUM_OF_INTERFACES , x if x == remote_procedure :: REMOTE_PROC_CONNECT_LIST_INTERFACES as i32 => remote_procedure :: REMOTE_PROC_CONNECT_LIST_INTERFACES , x if x == remote_procedure :: REMOTE_PROC_INTERFACE_LOOKUP_BY_NAME as i32 => remote_procedure :: REMOTE_PROC_INTERFACE_LOOKUP_BY_NAME , x if x == remote_procedure :: REMOTE_PROC_INTERFACE_LOOKUP_BY_MAC_STRING as i32 => remote_procedure :: REMOTE_PROC_INTERFACE_LOOKUP_BY_MAC_STRING , x if x == remote_procedure :: REMOTE_PROC_INTERFACE_GET_XML_DESC as i32 => remote_procedure :: REMOTE_PROC_INTERFACE_GET_XML_DESC , x if x == remote_procedure :: REMOTE_PROC_INTERFACE_DEFINE_XML as i32 => remote_procedure :: REMOTE_PROC_INTERFACE_DEFINE_XML , x if x == remote_procedure :: REMOTE_PROC_INTERFACE_UNDEFINE as i32 => remote_procedure :: REMOTE_PROC_INTERFACE_UNDEFINE , x if x == remote_procedure :: REMOTE_PROC_INTERFACE_CREATE as i32 => remote_procedure :: REMOTE_PROC_INTERFACE_CREATE , x if x == remote_procedure :: REMOTE_PROC_INTERFACE_DESTROY as i32 => remote_procedure :: REMOTE_PROC_INTERFACE_DESTROY , x if x == remote_procedure :: REMOTE_PROC_CONNECT_DOMAIN_XML_FROM_NATIVE as i32 => remote_procedure :: REMOTE_PROC_CONNECT_DOMAIN_XML_FROM_NATIVE , x if x == remote_procedure :: REMOTE_PROC_CONNECT_DOMAIN_XML_TO_NATIVE as i32 => remote_procedure :: REMOTE_PROC_CONNECT_DOMAIN_XML_TO_NATIVE , x if x == remote_procedure :: REMOTE_PROC_CONNECT_NUM_OF_DEFINED_INTERFACES as i32 => remote_procedure :: REMOTE_PROC_CONNECT_NUM_OF_DEFINED_INTERFACES , x if x == remote_procedure :: REMOTE_PROC_CONNECT_LIST_DEFINED_INTERFACES as i32 => remote_procedure :: REMOTE_PROC_CONNECT_LIST_DEFINED_INTERFACES , x if x == remote_procedure :: REMOTE_PROC_CONNECT_NUM_OF_SECRETS as i32 => remote_procedure :: REMOTE_PROC_CONNECT_NUM_OF_SECRETS , x if x == remote_procedure :: REMOTE_PROC_CONNECT_LIST_SECRETS as i32 => remote_procedure :: REMOTE_PROC_CONNECT_LIST_SECRETS , x if x == remote_procedure :: REMOTE_PROC_SECRET_LOOKUP_BY_UUID as i32 => remote_procedure :: REMOTE_PROC_SECRET_LOOKUP_BY_UUID , x if x == remote_procedure :: REMOTE_PROC_SECRET_DEFINE_XML as i32 => remote_procedure :: REMOTE_PROC_SECRET_DEFINE_XML , x if x == remote_procedure :: REMOTE_PROC_SECRET_GET_XML_DESC as i32 => remote_procedure :: REMOTE_PROC_SECRET_GET_XML_DESC , x if x == remote_procedure :: REMOTE_PROC_SECRET_SET_VALUE as i32 => remote_procedure :: REMOTE_PROC_SECRET_SET_VALUE , x if x == remote_procedure :: REMOTE_PROC_SECRET_GET_VALUE as i32 => remote_procedure :: REMOTE_PROC_SECRET_GET_VALUE , x if x == remote_procedure :: REMOTE_PROC_SECRET_UNDEFINE as i32 => remote_procedure :: REMOTE_PROC_SECRET_UNDEFINE , x if x == remote_procedure :: REMOTE_PROC_SECRET_LOOKUP_BY_USAGE as i32 => remote_procedure :: REMOTE_PROC_SECRET_LOOKUP_BY_USAGE , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_MIGRATE_PREPARE_TUNNEL as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_MIGRATE_PREPARE_TUNNEL , x if x == remote_procedure :: REMOTE_PROC_CONNECT_IS_SECURE as i32 => remote_procedure :: REMOTE_PROC_CONNECT_IS_SECURE , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_IS_ACTIVE as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_IS_ACTIVE , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_IS_PERSISTENT as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_IS_PERSISTENT , x if x == remote_procedure :: REMOTE_PROC_NETWORK_IS_ACTIVE as i32 => remote_procedure :: REMOTE_PROC_NETWORK_IS_ACTIVE , x if x == remote_procedure :: REMOTE_PROC_NETWORK_IS_PERSISTENT as i32 => remote_procedure :: REMOTE_PROC_NETWORK_IS_PERSISTENT , x if x == remote_procedure :: REMOTE_PROC_STORAGE_POOL_IS_ACTIVE as i32 => remote_procedure :: REMOTE_PROC_STORAGE_POOL_IS_ACTIVE , x if x == remote_procedure :: REMOTE_PROC_STORAGE_POOL_IS_PERSISTENT as i32 => remote_procedure :: REMOTE_PROC_STORAGE_POOL_IS_PERSISTENT , x if x == remote_procedure :: REMOTE_PROC_INTERFACE_IS_ACTIVE as i32 => remote_procedure :: REMOTE_PROC_INTERFACE_IS_ACTIVE , x if x == remote_procedure :: REMOTE_PROC_CONNECT_GET_LIB_VERSION as i32 => remote_procedure :: REMOTE_PROC_CONNECT_GET_LIB_VERSION , x if x == remote_procedure :: REMOTE_PROC_CONNECT_COMPARE_CPU as i32 => remote_procedure :: REMOTE_PROC_CONNECT_COMPARE_CPU , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_MEMORY_STATS as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_MEMORY_STATS , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_ATTACH_DEVICE_FLAGS as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_ATTACH_DEVICE_FLAGS , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_DETACH_DEVICE_FLAGS as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_DETACH_DEVICE_FLAGS , x if x == remote_procedure :: REMOTE_PROC_CONNECT_BASELINE_CPU as i32 => remote_procedure :: REMOTE_PROC_CONNECT_BASELINE_CPU , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_GET_JOB_INFO as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_GET_JOB_INFO , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_ABORT_JOB as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_ABORT_JOB , x if x == remote_procedure :: REMOTE_PROC_STORAGE_VOL_WIPE as i32 => remote_procedure :: REMOTE_PROC_STORAGE_VOL_WIPE , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_MIGRATE_SET_MAX_DOWNTIME as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_MIGRATE_SET_MAX_DOWNTIME , x if x == remote_procedure :: REMOTE_PROC_CONNECT_DOMAIN_EVENT_REGISTER_ANY as i32 => remote_procedure :: REMOTE_PROC_CONNECT_DOMAIN_EVENT_REGISTER_ANY , x if x == remote_procedure :: REMOTE_PROC_CONNECT_DOMAIN_EVENT_DEREGISTER_ANY as i32 => remote_procedure :: REMOTE_PROC_CONNECT_DOMAIN_EVENT_DEREGISTER_ANY , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_EVENT_REBOOT as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_EVENT_REBOOT , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_EVENT_RTC_CHANGE as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_EVENT_RTC_CHANGE , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_EVENT_WATCHDOG as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_EVENT_WATCHDOG , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_EVENT_IO_ERROR as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_EVENT_IO_ERROR , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_EVENT_GRAPHICS as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_EVENT_GRAPHICS , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_UPDATE_DEVICE_FLAGS as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_UPDATE_DEVICE_FLAGS , x if x == remote_procedure :: REMOTE_PROC_NWFILTER_LOOKUP_BY_NAME as i32 => remote_procedure :: REMOTE_PROC_NWFILTER_LOOKUP_BY_NAME , x if x == remote_procedure :: REMOTE_PROC_NWFILTER_LOOKUP_BY_UUID as i32 => remote_procedure :: REMOTE_PROC_NWFILTER_LOOKUP_BY_UUID , x if x == remote_procedure :: REMOTE_PROC_NWFILTER_GET_XML_DESC as i32 => remote_procedure :: REMOTE_PROC_NWFILTER_GET_XML_DESC , x if x == remote_procedure :: REMOTE_PROC_CONNECT_NUM_OF_NWFILTERS as i32 => remote_procedure :: REMOTE_PROC_CONNECT_NUM_OF_NWFILTERS , x if x == remote_procedure :: REMOTE_PROC_CONNECT_LIST_NWFILTERS as i32 => remote_procedure :: REMOTE_PROC_CONNECT_LIST_NWFILTERS , x if x == remote_procedure :: REMOTE_PROC_NWFILTER_DEFINE_XML as i32 => remote_procedure :: REMOTE_PROC_NWFILTER_DEFINE_XML , x if x == remote_procedure :: REMOTE_PROC_NWFILTER_UNDEFINE as i32 => remote_procedure :: REMOTE_PROC_NWFILTER_UNDEFINE , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_MANAGED_SAVE as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_MANAGED_SAVE , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_HAS_MANAGED_SAVE_IMAGE as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_HAS_MANAGED_SAVE_IMAGE , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_MANAGED_SAVE_REMOVE as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_MANAGED_SAVE_REMOVE , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_SNAPSHOT_CREATE_XML as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_SNAPSHOT_CREATE_XML , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_SNAPSHOT_GET_XML_DESC as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_SNAPSHOT_GET_XML_DESC , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_SNAPSHOT_NUM as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_SNAPSHOT_NUM , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_SNAPSHOT_LIST_NAMES as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_SNAPSHOT_LIST_NAMES , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_SNAPSHOT_LOOKUP_BY_NAME as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_SNAPSHOT_LOOKUP_BY_NAME , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_HAS_CURRENT_SNAPSHOT as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_HAS_CURRENT_SNAPSHOT , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_SNAPSHOT_CURRENT as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_SNAPSHOT_CURRENT , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_REVERT_TO_SNAPSHOT as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_REVERT_TO_SNAPSHOT , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_SNAPSHOT_DELETE as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_SNAPSHOT_DELETE , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_GET_BLOCK_INFO as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_GET_BLOCK_INFO , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_EVENT_IO_ERROR_REASON as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_EVENT_IO_ERROR_REASON , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_CREATE_WITH_FLAGS as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_CREATE_WITH_FLAGS , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_SET_MEMORY_PARAMETERS as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_SET_MEMORY_PARAMETERS , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_GET_MEMORY_PARAMETERS as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_GET_MEMORY_PARAMETERS , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_SET_VCPUS_FLAGS as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_SET_VCPUS_FLAGS , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_GET_VCPUS_FLAGS as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_GET_VCPUS_FLAGS , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_OPEN_CONSOLE as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_OPEN_CONSOLE , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_IS_UPDATED as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_IS_UPDATED , x if x == remote_procedure :: REMOTE_PROC_CONNECT_GET_SYSINFO as i32 => remote_procedure :: REMOTE_PROC_CONNECT_GET_SYSINFO , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_SET_MEMORY_FLAGS as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_SET_MEMORY_FLAGS , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_SET_BLKIO_PARAMETERS as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_SET_BLKIO_PARAMETERS , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_GET_BLKIO_PARAMETERS as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_GET_BLKIO_PARAMETERS , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_MIGRATE_SET_MAX_SPEED as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_MIGRATE_SET_MAX_SPEED , x if x == remote_procedure :: REMOTE_PROC_STORAGE_VOL_UPLOAD as i32 => remote_procedure :: REMOTE_PROC_STORAGE_VOL_UPLOAD , x if x == remote_procedure :: REMOTE_PROC_STORAGE_VOL_DOWNLOAD as i32 => remote_procedure :: REMOTE_PROC_STORAGE_VOL_DOWNLOAD , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_INJECT_NMI as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_INJECT_NMI , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_SCREENSHOT as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_SCREENSHOT , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_GET_STATE as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_GET_STATE , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_MIGRATE_BEGIN3 as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_MIGRATE_BEGIN3 , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_MIGRATE_PREPARE3 as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_MIGRATE_PREPARE3 , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_MIGRATE_PREPARE_TUNNEL3 as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_MIGRATE_PREPARE_TUNNEL3 , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_MIGRATE_PERFORM3 as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_MIGRATE_PERFORM3 , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_MIGRATE_FINISH3 as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_MIGRATE_FINISH3 , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_MIGRATE_CONFIRM3 as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_MIGRATE_CONFIRM3 , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_SET_SCHEDULER_PARAMETERS_FLAGS as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_SET_SCHEDULER_PARAMETERS_FLAGS , x if x == remote_procedure :: REMOTE_PROC_INTERFACE_CHANGE_BEGIN as i32 => remote_procedure :: REMOTE_PROC_INTERFACE_CHANGE_BEGIN , x if x == remote_procedure :: REMOTE_PROC_INTERFACE_CHANGE_COMMIT as i32 => remote_procedure :: REMOTE_PROC_INTERFACE_CHANGE_COMMIT , x if x == remote_procedure :: REMOTE_PROC_INTERFACE_CHANGE_ROLLBACK as i32 => remote_procedure :: REMOTE_PROC_INTERFACE_CHANGE_ROLLBACK , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_GET_SCHEDULER_PARAMETERS_FLAGS as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_GET_SCHEDULER_PARAMETERS_FLAGS , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_EVENT_CONTROL_ERROR as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_EVENT_CONTROL_ERROR , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_PIN_VCPU_FLAGS as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_PIN_VCPU_FLAGS , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_SEND_KEY as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_SEND_KEY , x if x == remote_procedure :: REMOTE_PROC_NODE_GET_CPU_STATS as i32 => remote_procedure :: REMOTE_PROC_NODE_GET_CPU_STATS , x if x == remote_procedure :: REMOTE_PROC_NODE_GET_MEMORY_STATS as i32 => remote_procedure :: REMOTE_PROC_NODE_GET_MEMORY_STATS , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_GET_CONTROL_INFO as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_GET_CONTROL_INFO , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_GET_VCPU_PIN_INFO as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_GET_VCPU_PIN_INFO , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_UNDEFINE_FLAGS as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_UNDEFINE_FLAGS , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_SAVE_FLAGS as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_SAVE_FLAGS , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_RESTORE_FLAGS as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_RESTORE_FLAGS , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_DESTROY_FLAGS as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_DESTROY_FLAGS , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_SAVE_IMAGE_GET_XML_DESC as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_SAVE_IMAGE_GET_XML_DESC , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_SAVE_IMAGE_DEFINE_XML as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_SAVE_IMAGE_DEFINE_XML , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_BLOCK_JOB_ABORT as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_BLOCK_JOB_ABORT , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_GET_BLOCK_JOB_INFO as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_GET_BLOCK_JOB_INFO , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_BLOCK_JOB_SET_SPEED as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_BLOCK_JOB_SET_SPEED , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_BLOCK_PULL as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_BLOCK_PULL , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_EVENT_BLOCK_JOB as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_EVENT_BLOCK_JOB , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_MIGRATE_GET_MAX_SPEED as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_MIGRATE_GET_MAX_SPEED , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_BLOCK_STATS_FLAGS as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_BLOCK_STATS_FLAGS , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_SNAPSHOT_GET_PARENT as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_SNAPSHOT_GET_PARENT , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_RESET as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_RESET , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_SNAPSHOT_NUM_CHILDREN as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_SNAPSHOT_NUM_CHILDREN , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_SNAPSHOT_LIST_CHILDREN_NAMES as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_SNAPSHOT_LIST_CHILDREN_NAMES , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_EVENT_DISK_CHANGE as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_EVENT_DISK_CHANGE , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_OPEN_GRAPHICS as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_OPEN_GRAPHICS , x if x == remote_procedure :: REMOTE_PROC_NODE_SUSPEND_FOR_DURATION as i32 => remote_procedure :: REMOTE_PROC_NODE_SUSPEND_FOR_DURATION , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_BLOCK_RESIZE as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_BLOCK_RESIZE , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_SET_BLOCK_IO_TUNE as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_SET_BLOCK_IO_TUNE , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_GET_BLOCK_IO_TUNE as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_GET_BLOCK_IO_TUNE , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_SET_NUMA_PARAMETERS as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_SET_NUMA_PARAMETERS , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_GET_NUMA_PARAMETERS as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_GET_NUMA_PARAMETERS , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_SET_INTERFACE_PARAMETERS as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_SET_INTERFACE_PARAMETERS , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_GET_INTERFACE_PARAMETERS as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_GET_INTERFACE_PARAMETERS , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_SHUTDOWN_FLAGS as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_SHUTDOWN_FLAGS , x if x == remote_procedure :: REMOTE_PROC_STORAGE_VOL_WIPE_PATTERN as i32 => remote_procedure :: REMOTE_PROC_STORAGE_VOL_WIPE_PATTERN , x if x == remote_procedure :: REMOTE_PROC_STORAGE_VOL_RESIZE as i32 => remote_procedure :: REMOTE_PROC_STORAGE_VOL_RESIZE , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_PM_SUSPEND_FOR_DURATION as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_PM_SUSPEND_FOR_DURATION , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_GET_CPU_STATS as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_GET_CPU_STATS , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_GET_DISK_ERRORS as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_GET_DISK_ERRORS , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_SET_METADATA as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_SET_METADATA , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_GET_METADATA as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_GET_METADATA , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_BLOCK_REBASE as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_BLOCK_REBASE , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_PM_WAKEUP as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_PM_WAKEUP , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_EVENT_TRAY_CHANGE as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_EVENT_TRAY_CHANGE , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_EVENT_PMWAKEUP as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_EVENT_PMWAKEUP , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_EVENT_PMSUSPEND as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_EVENT_PMSUSPEND , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_SNAPSHOT_IS_CURRENT as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_SNAPSHOT_IS_CURRENT , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_SNAPSHOT_HAS_METADATA as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_SNAPSHOT_HAS_METADATA , x if x == remote_procedure :: REMOTE_PROC_CONNECT_LIST_ALL_DOMAINS as i32 => remote_procedure :: REMOTE_PROC_CONNECT_LIST_ALL_DOMAINS , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_LIST_ALL_SNAPSHOTS as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_LIST_ALL_SNAPSHOTS , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_SNAPSHOT_LIST_ALL_CHILDREN as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_SNAPSHOT_LIST_ALL_CHILDREN , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_EVENT_BALLOON_CHANGE as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_EVENT_BALLOON_CHANGE , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_GET_HOSTNAME as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_GET_HOSTNAME , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_GET_SECURITY_LABEL_LIST as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_GET_SECURITY_LABEL_LIST , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_PIN_EMULATOR as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_PIN_EMULATOR , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_GET_EMULATOR_PIN_INFO as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_GET_EMULATOR_PIN_INFO , x if x == remote_procedure :: REMOTE_PROC_CONNECT_LIST_ALL_STORAGE_POOLS as i32 => remote_procedure :: REMOTE_PROC_CONNECT_LIST_ALL_STORAGE_POOLS , x if x == remote_procedure :: REMOTE_PROC_STORAGE_POOL_LIST_ALL_VOLUMES as i32 => remote_procedure :: REMOTE_PROC_STORAGE_POOL_LIST_ALL_VOLUMES , x if x == remote_procedure :: REMOTE_PROC_CONNECT_LIST_ALL_NETWORKS as i32 => remote_procedure :: REMOTE_PROC_CONNECT_LIST_ALL_NETWORKS , x if x == remote_procedure :: REMOTE_PROC_CONNECT_LIST_ALL_INTERFACES as i32 => remote_procedure :: REMOTE_PROC_CONNECT_LIST_ALL_INTERFACES , x if x == remote_procedure :: REMOTE_PROC_CONNECT_LIST_ALL_NODE_DEVICES as i32 => remote_procedure :: REMOTE_PROC_CONNECT_LIST_ALL_NODE_DEVICES , x if x == remote_procedure :: REMOTE_PROC_CONNECT_LIST_ALL_NWFILTERS as i32 => remote_procedure :: REMOTE_PROC_CONNECT_LIST_ALL_NWFILTERS , x if x == remote_procedure :: REMOTE_PROC_CONNECT_LIST_ALL_SECRETS as i32 => remote_procedure :: REMOTE_PROC_CONNECT_LIST_ALL_SECRETS , x if x == remote_procedure :: REMOTE_PROC_NODE_SET_MEMORY_PARAMETERS as i32 => remote_procedure :: REMOTE_PROC_NODE_SET_MEMORY_PARAMETERS , x if x == remote_procedure :: REMOTE_PROC_NODE_GET_MEMORY_PARAMETERS as i32 => remote_procedure :: REMOTE_PROC_NODE_GET_MEMORY_PARAMETERS , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_BLOCK_COMMIT as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_BLOCK_COMMIT , x if x == remote_procedure :: REMOTE_PROC_NETWORK_UPDATE as i32 => remote_procedure :: REMOTE_PROC_NETWORK_UPDATE , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_EVENT_PMSUSPEND_DISK as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_EVENT_PMSUSPEND_DISK , x if x == remote_procedure :: REMOTE_PROC_NODE_GET_CPU_MAP as i32 => remote_procedure :: REMOTE_PROC_NODE_GET_CPU_MAP , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_FSTRIM as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_FSTRIM , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_SEND_PROCESS_SIGNAL as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_SEND_PROCESS_SIGNAL , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_OPEN_CHANNEL as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_OPEN_CHANNEL , x if x == remote_procedure :: REMOTE_PROC_NODE_DEVICE_LOOKUP_SCSI_HOST_BY_WWN as i32 => remote_procedure :: REMOTE_PROC_NODE_DEVICE_LOOKUP_SCSI_HOST_BY_WWN , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_GET_JOB_STATS as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_GET_JOB_STATS , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_MIGRATE_GET_COMPRESSION_CACHE as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_MIGRATE_GET_COMPRESSION_CACHE , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_MIGRATE_SET_COMPRESSION_CACHE as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_MIGRATE_SET_COMPRESSION_CACHE , x if x == remote_procedure :: REMOTE_PROC_NODE_DEVICE_DETACH_FLAGS as i32 => remote_procedure :: REMOTE_PROC_NODE_DEVICE_DETACH_FLAGS , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_MIGRATE_BEGIN3_PARAMS as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_MIGRATE_BEGIN3_PARAMS , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_MIGRATE_PREPARE3_PARAMS as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_MIGRATE_PREPARE3_PARAMS , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_MIGRATE_PREPARE_TUNNEL3_PARAMS as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_MIGRATE_PREPARE_TUNNEL3_PARAMS , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_MIGRATE_PERFORM3_PARAMS as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_MIGRATE_PERFORM3_PARAMS , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_MIGRATE_FINISH3_PARAMS as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_MIGRATE_FINISH3_PARAMS , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_MIGRATE_CONFIRM3_PARAMS as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_MIGRATE_CONFIRM3_PARAMS , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_SET_MEMORY_STATS_PERIOD as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_SET_MEMORY_STATS_PERIOD , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_CREATE_XML_WITH_FILES as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_CREATE_XML_WITH_FILES , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_CREATE_WITH_FILES as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_CREATE_WITH_FILES , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_EVENT_DEVICE_REMOVED as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_EVENT_DEVICE_REMOVED , x if x == remote_procedure :: REMOTE_PROC_CONNECT_GET_CPU_MODEL_NAMES as i32 => remote_procedure :: REMOTE_PROC_CONNECT_GET_CPU_MODEL_NAMES , x if x == remote_procedure :: REMOTE_PROC_CONNECT_NETWORK_EVENT_REGISTER_ANY as i32 => remote_procedure :: REMOTE_PROC_CONNECT_NETWORK_EVENT_REGISTER_ANY , x if x == remote_procedure :: REMOTE_PROC_CONNECT_NETWORK_EVENT_DEREGISTER_ANY as i32 => remote_procedure :: REMOTE_PROC_CONNECT_NETWORK_EVENT_DEREGISTER_ANY , x if x == remote_procedure :: REMOTE_PROC_NETWORK_EVENT_LIFECYCLE as i32 => remote_procedure :: REMOTE_PROC_NETWORK_EVENT_LIFECYCLE , x if x == remote_procedure :: REMOTE_PROC_CONNECT_DOMAIN_EVENT_CALLBACK_REGISTER_ANY as i32 => remote_procedure :: REMOTE_PROC_CONNECT_DOMAIN_EVENT_CALLBACK_REGISTER_ANY , x if x == remote_procedure :: REMOTE_PROC_CONNECT_DOMAIN_EVENT_CALLBACK_DEREGISTER_ANY as i32 => remote_procedure :: REMOTE_PROC_CONNECT_DOMAIN_EVENT_CALLBACK_DEREGISTER_ANY , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_EVENT_CALLBACK_LIFECYCLE as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_EVENT_CALLBACK_LIFECYCLE , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_EVENT_CALLBACK_REBOOT as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_EVENT_CALLBACK_REBOOT , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_EVENT_CALLBACK_RTC_CHANGE as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_EVENT_CALLBACK_RTC_CHANGE , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_EVENT_CALLBACK_WATCHDOG as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_EVENT_CALLBACK_WATCHDOG , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_EVENT_CALLBACK_IO_ERROR as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_EVENT_CALLBACK_IO_ERROR , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_EVENT_CALLBACK_GRAPHICS as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_EVENT_CALLBACK_GRAPHICS , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_EVENT_CALLBACK_IO_ERROR_REASON as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_EVENT_CALLBACK_IO_ERROR_REASON , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_EVENT_CALLBACK_CONTROL_ERROR as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_EVENT_CALLBACK_CONTROL_ERROR , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_EVENT_CALLBACK_BLOCK_JOB as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_EVENT_CALLBACK_BLOCK_JOB , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_EVENT_CALLBACK_DISK_CHANGE as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_EVENT_CALLBACK_DISK_CHANGE , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_EVENT_CALLBACK_TRAY_CHANGE as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_EVENT_CALLBACK_TRAY_CHANGE , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_EVENT_CALLBACK_PMWAKEUP as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_EVENT_CALLBACK_PMWAKEUP , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_EVENT_CALLBACK_PMSUSPEND as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_EVENT_CALLBACK_PMSUSPEND , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_EVENT_CALLBACK_BALLOON_CHANGE as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_EVENT_CALLBACK_BALLOON_CHANGE , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_EVENT_CALLBACK_PMSUSPEND_DISK as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_EVENT_CALLBACK_PMSUSPEND_DISK , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_EVENT_CALLBACK_DEVICE_REMOVED as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_EVENT_CALLBACK_DEVICE_REMOVED , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_CORE_DUMP_WITH_FORMAT as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_CORE_DUMP_WITH_FORMAT , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_FSFREEZE as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_FSFREEZE , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_FSTHAW as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_FSTHAW , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_GET_TIME as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_GET_TIME , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_SET_TIME as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_SET_TIME , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_EVENT_BLOCK_JOB_2 as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_EVENT_BLOCK_JOB_2 , x if x == remote_procedure :: REMOTE_PROC_NODE_GET_FREE_PAGES as i32 => remote_procedure :: REMOTE_PROC_NODE_GET_FREE_PAGES , x if x == remote_procedure :: REMOTE_PROC_NETWORK_GET_DHCP_LEASES as i32 => remote_procedure :: REMOTE_PROC_NETWORK_GET_DHCP_LEASES , x if x == remote_procedure :: REMOTE_PROC_CONNECT_GET_DOMAIN_CAPABILITIES as i32 => remote_procedure :: REMOTE_PROC_CONNECT_GET_DOMAIN_CAPABILITIES , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_OPEN_GRAPHICS_FD as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_OPEN_GRAPHICS_FD , x if x == remote_procedure :: REMOTE_PROC_CONNECT_GET_ALL_DOMAIN_STATS as i32 => remote_procedure :: REMOTE_PROC_CONNECT_GET_ALL_DOMAIN_STATS , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_BLOCK_COPY as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_BLOCK_COPY , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_EVENT_CALLBACK_TUNABLE as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_EVENT_CALLBACK_TUNABLE , x if x == remote_procedure :: REMOTE_PROC_NODE_ALLOC_PAGES as i32 => remote_procedure :: REMOTE_PROC_NODE_ALLOC_PAGES , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_EVENT_CALLBACK_AGENT_LIFECYCLE as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_EVENT_CALLBACK_AGENT_LIFECYCLE , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_GET_FSINFO as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_GET_FSINFO , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_DEFINE_XML_FLAGS as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_DEFINE_XML_FLAGS , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_GET_IOTHREAD_INFO as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_GET_IOTHREAD_INFO , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_PIN_IOTHREAD as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_PIN_IOTHREAD , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_INTERFACE_ADDRESSES as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_INTERFACE_ADDRESSES , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_EVENT_CALLBACK_DEVICE_ADDED as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_EVENT_CALLBACK_DEVICE_ADDED , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_ADD_IOTHREAD as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_ADD_IOTHREAD , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_DEL_IOTHREAD as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_DEL_IOTHREAD , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_SET_USER_PASSWORD as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_SET_USER_PASSWORD , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_RENAME as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_RENAME , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_EVENT_CALLBACK_MIGRATION_ITERATION as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_EVENT_CALLBACK_MIGRATION_ITERATION , x if x == remote_procedure :: REMOTE_PROC_CONNECT_REGISTER_CLOSE_CALLBACK as i32 => remote_procedure :: REMOTE_PROC_CONNECT_REGISTER_CLOSE_CALLBACK , x if x == remote_procedure :: REMOTE_PROC_CONNECT_UNREGISTER_CLOSE_CALLBACK as i32 => remote_procedure :: REMOTE_PROC_CONNECT_UNREGISTER_CLOSE_CALLBACK , x if x == remote_procedure :: REMOTE_PROC_CONNECT_EVENT_CONNECTION_CLOSED as i32 => remote_procedure :: REMOTE_PROC_CONNECT_EVENT_CONNECTION_CLOSED , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_EVENT_CALLBACK_JOB_COMPLETED as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_EVENT_CALLBACK_JOB_COMPLETED , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_MIGRATE_START_POST_COPY as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_MIGRATE_START_POST_COPY , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_GET_PERF_EVENTS as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_GET_PERF_EVENTS , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_SET_PERF_EVENTS as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_SET_PERF_EVENTS , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_EVENT_CALLBACK_DEVICE_REMOVAL_FAILED as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_EVENT_CALLBACK_DEVICE_REMOVAL_FAILED , x if x == remote_procedure :: REMOTE_PROC_CONNECT_STORAGE_POOL_EVENT_REGISTER_ANY as i32 => remote_procedure :: REMOTE_PROC_CONNECT_STORAGE_POOL_EVENT_REGISTER_ANY , x if x == remote_procedure :: REMOTE_PROC_CONNECT_STORAGE_POOL_EVENT_DEREGISTER_ANY as i32 => remote_procedure :: REMOTE_PROC_CONNECT_STORAGE_POOL_EVENT_DEREGISTER_ANY , x if x == remote_procedure :: REMOTE_PROC_STORAGE_POOL_EVENT_LIFECYCLE as i32 => remote_procedure :: REMOTE_PROC_STORAGE_POOL_EVENT_LIFECYCLE , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_GET_GUEST_VCPUS as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_GET_GUEST_VCPUS , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_SET_GUEST_VCPUS as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_SET_GUEST_VCPUS , x if x == remote_procedure :: REMOTE_PROC_STORAGE_POOL_EVENT_REFRESH as i32 => remote_procedure :: REMOTE_PROC_STORAGE_POOL_EVENT_REFRESH , x if x == remote_procedure :: REMOTE_PROC_CONNECT_NODE_DEVICE_EVENT_REGISTER_ANY as i32 => remote_procedure :: REMOTE_PROC_CONNECT_NODE_DEVICE_EVENT_REGISTER_ANY , x if x == remote_procedure :: REMOTE_PROC_CONNECT_NODE_DEVICE_EVENT_DEREGISTER_ANY as i32 => remote_procedure :: REMOTE_PROC_CONNECT_NODE_DEVICE_EVENT_DEREGISTER_ANY , x if x == remote_procedure :: REMOTE_PROC_NODE_DEVICE_EVENT_LIFECYCLE as i32 => remote_procedure :: REMOTE_PROC_NODE_DEVICE_EVENT_LIFECYCLE , x if x == remote_procedure :: REMOTE_PROC_NODE_DEVICE_EVENT_UPDATE as i32 => remote_procedure :: REMOTE_PROC_NODE_DEVICE_EVENT_UPDATE , x if x == remote_procedure :: REMOTE_PROC_STORAGE_VOL_GET_INFO_FLAGS as i32 => remote_procedure :: REMOTE_PROC_STORAGE_VOL_GET_INFO_FLAGS , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_EVENT_CALLBACK_METADATA_CHANGE as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_EVENT_CALLBACK_METADATA_CHANGE , x if x == remote_procedure :: REMOTE_PROC_CONNECT_SECRET_EVENT_REGISTER_ANY as i32 => remote_procedure :: REMOTE_PROC_CONNECT_SECRET_EVENT_REGISTER_ANY , x if x == remote_procedure :: REMOTE_PROC_CONNECT_SECRET_EVENT_DEREGISTER_ANY as i32 => remote_procedure :: REMOTE_PROC_CONNECT_SECRET_EVENT_DEREGISTER_ANY , x if x == remote_procedure :: REMOTE_PROC_SECRET_EVENT_LIFECYCLE as i32 => remote_procedure :: REMOTE_PROC_SECRET_EVENT_LIFECYCLE , x if x == remote_procedure :: REMOTE_PROC_SECRET_EVENT_VALUE_CHANGED as i32 => remote_procedure :: REMOTE_PROC_SECRET_EVENT_VALUE_CHANGED , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_SET_VCPU as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_SET_VCPU , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_EVENT_BLOCK_THRESHOLD as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_EVENT_BLOCK_THRESHOLD , x if x == remote_procedure :: REMOTE_PROC_DOMAIN_SET_BLOCK_THRESHOLD as i32 => remote_procedure :: REMOTE_PROC_DOMAIN_SET_BLOCK_THRESHOLD , e => return Err ( xdr_codec :: Error :: invalidenum ( e ) ) }
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_secret_define_xml_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_secret_define_xml_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_secret_define_xml_args {
                xml: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                flags: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_secret_define_xml_ret {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_secret_define_xml_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_secret_define_xml_ret {
                secret: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_secret_event_lifecycle_msg {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_secret_event_lifecycle_msg, usize)> {
        let mut sz = 0;
        Ok((
            remote_secret_event_lifecycle_msg {
                callbackID: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                secret: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                event: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                detail: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_secret_event_value_changed_msg {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_secret_event_value_changed_msg, usize)> {
        let mut sz = 0;
        Ok((
            remote_secret_event_value_changed_msg {
                callbackID: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                secret: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_secret_get_value_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_secret_get_value_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_secret_get_value_args {
                secret: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                flags: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_secret_get_value_ret {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_secret_get_value_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_secret_get_value_ret {
                value: {
                    let (v, fsz) = xdr_codec::unpack_opaque_flex(
                        input,
                        Some(REMOTE_SECRET_VALUE_MAX as usize),
                    )?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_secret_get_xml_desc_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_secret_get_xml_desc_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_secret_get_xml_desc_args {
                secret: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                flags: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_secret_get_xml_desc_ret {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_secret_get_xml_desc_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_secret_get_xml_desc_ret {
                xml: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_secret_lookup_by_usage_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_secret_lookup_by_usage_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_secret_lookup_by_usage_args {
                usageType: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                usageID: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_secret_lookup_by_usage_ret {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_secret_lookup_by_usage_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_secret_lookup_by_usage_ret {
                secret: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_secret_lookup_by_uuid_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_secret_lookup_by_uuid_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_secret_lookup_by_uuid_args {
                uuid: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_secret_lookup_by_uuid_ret {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_secret_lookup_by_uuid_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_secret_lookup_by_uuid_ret {
                secret: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_secret_set_value_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_secret_set_value_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_secret_set_value_args {
                secret: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                value: {
                    let (v, fsz) = xdr_codec::unpack_opaque_flex(
                        input,
                        Some(REMOTE_SECRET_VALUE_MAX as usize),
                    )?;
                    sz += fsz;
                    v
                },
                flags: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_secret_undefine_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_secret_undefine_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_secret_undefine_args {
                secret: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_storage_pool_build_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_storage_pool_build_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_storage_pool_build_args {
                pool: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                flags: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_storage_pool_create_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_storage_pool_create_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_storage_pool_create_args {
                pool: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                flags: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_storage_pool_create_xml_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_storage_pool_create_xml_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_storage_pool_create_xml_args {
                xml: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                flags: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_storage_pool_create_xml_ret {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_storage_pool_create_xml_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_storage_pool_create_xml_ret {
                pool: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_storage_pool_define_xml_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_storage_pool_define_xml_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_storage_pool_define_xml_args {
                xml: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                flags: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_storage_pool_define_xml_ret {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_storage_pool_define_xml_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_storage_pool_define_xml_ret {
                pool: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_storage_pool_delete_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_storage_pool_delete_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_storage_pool_delete_args {
                pool: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                flags: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_storage_pool_destroy_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_storage_pool_destroy_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_storage_pool_destroy_args {
                pool: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_storage_pool_event_lifecycle_msg {
    fn unpack(
        input: &mut In,
    ) -> xdr_codec::Result<(remote_storage_pool_event_lifecycle_msg, usize)> {
        let mut sz = 0;
        Ok((
            remote_storage_pool_event_lifecycle_msg {
                callbackID: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                pool: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                event: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                detail: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_storage_pool_event_refresh_msg {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_storage_pool_event_refresh_msg, usize)> {
        let mut sz = 0;
        Ok((
            remote_storage_pool_event_refresh_msg {
                callbackID: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                pool: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_storage_pool_get_autostart_args {
    fn unpack(
        input: &mut In,
    ) -> xdr_codec::Result<(remote_storage_pool_get_autostart_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_storage_pool_get_autostart_args {
                pool: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_storage_pool_get_autostart_ret {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_storage_pool_get_autostart_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_storage_pool_get_autostart_ret {
                autostart: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_storage_pool_get_info_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_storage_pool_get_info_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_storage_pool_get_info_args {
                pool: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_storage_pool_get_info_ret {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_storage_pool_get_info_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_storage_pool_get_info_ret {
                state: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                capacity: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                allocation: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                available: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_storage_pool_get_xml_desc_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_storage_pool_get_xml_desc_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_storage_pool_get_xml_desc_args {
                pool: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                flags: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_storage_pool_get_xml_desc_ret {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_storage_pool_get_xml_desc_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_storage_pool_get_xml_desc_ret {
                xml: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_storage_pool_is_active_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_storage_pool_is_active_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_storage_pool_is_active_args {
                pool: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_storage_pool_is_active_ret {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_storage_pool_is_active_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_storage_pool_is_active_ret {
                active: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_storage_pool_is_persistent_args {
    fn unpack(
        input: &mut In,
    ) -> xdr_codec::Result<(remote_storage_pool_is_persistent_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_storage_pool_is_persistent_args {
                pool: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_storage_pool_is_persistent_ret {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_storage_pool_is_persistent_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_storage_pool_is_persistent_ret {
                persistent: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_storage_pool_list_all_volumes_args {
    fn unpack(
        input: &mut In,
    ) -> xdr_codec::Result<(remote_storage_pool_list_all_volumes_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_storage_pool_list_all_volumes_args {
                pool: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                need_results: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                flags: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_storage_pool_list_all_volumes_ret {
    fn unpack(
        input: &mut In,
    ) -> xdr_codec::Result<(remote_storage_pool_list_all_volumes_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_storage_pool_list_all_volumes_ret {
                vols: {
                    let (v, fsz) =
                        xdr_codec::unpack_flex(input, Some(REMOTE_STORAGE_VOL_LIST_MAX as usize))?;
                    sz += fsz;
                    v
                },
                ret: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_storage_pool_list_volumes_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_storage_pool_list_volumes_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_storage_pool_list_volumes_args {
                pool: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                maxnames: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_storage_pool_list_volumes_ret {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_storage_pool_list_volumes_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_storage_pool_list_volumes_ret {
                names: {
                    let (v, fsz) =
                        xdr_codec::unpack_flex(input, Some(REMOTE_STORAGE_VOL_LIST_MAX as usize))?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_storage_pool_lookup_by_name_args {
    fn unpack(
        input: &mut In,
    ) -> xdr_codec::Result<(remote_storage_pool_lookup_by_name_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_storage_pool_lookup_by_name_args {
                name: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_storage_pool_lookup_by_name_ret {
    fn unpack(
        input: &mut In,
    ) -> xdr_codec::Result<(remote_storage_pool_lookup_by_name_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_storage_pool_lookup_by_name_ret {
                pool: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_storage_pool_lookup_by_uuid_args {
    fn unpack(
        input: &mut In,
    ) -> xdr_codec::Result<(remote_storage_pool_lookup_by_uuid_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_storage_pool_lookup_by_uuid_args {
                uuid: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_storage_pool_lookup_by_uuid_ret {
    fn unpack(
        input: &mut In,
    ) -> xdr_codec::Result<(remote_storage_pool_lookup_by_uuid_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_storage_pool_lookup_by_uuid_ret {
                pool: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_storage_pool_lookup_by_volume_args {
    fn unpack(
        input: &mut In,
    ) -> xdr_codec::Result<(remote_storage_pool_lookup_by_volume_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_storage_pool_lookup_by_volume_args {
                vol: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_storage_pool_lookup_by_volume_ret {
    fn unpack(
        input: &mut In,
    ) -> xdr_codec::Result<(remote_storage_pool_lookup_by_volume_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_storage_pool_lookup_by_volume_ret {
                pool: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_storage_pool_num_of_volumes_args {
    fn unpack(
        input: &mut In,
    ) -> xdr_codec::Result<(remote_storage_pool_num_of_volumes_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_storage_pool_num_of_volumes_args {
                pool: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_storage_pool_num_of_volumes_ret {
    fn unpack(
        input: &mut In,
    ) -> xdr_codec::Result<(remote_storage_pool_num_of_volumes_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_storage_pool_num_of_volumes_ret {
                num: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_storage_pool_refresh_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_storage_pool_refresh_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_storage_pool_refresh_args {
                pool: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                flags: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_storage_pool_set_autostart_args {
    fn unpack(
        input: &mut In,
    ) -> xdr_codec::Result<(remote_storage_pool_set_autostart_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_storage_pool_set_autostart_args {
                pool: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                autostart: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_storage_pool_undefine_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_storage_pool_undefine_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_storage_pool_undefine_args {
                pool: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_storage_vol_create_xml_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_storage_vol_create_xml_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_storage_vol_create_xml_args {
                pool: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                xml: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                flags: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_storage_vol_create_xml_from_args {
    fn unpack(
        input: &mut In,
    ) -> xdr_codec::Result<(remote_storage_vol_create_xml_from_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_storage_vol_create_xml_from_args {
                pool: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                xml: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                clonevol: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                flags: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_storage_vol_create_xml_from_ret {
    fn unpack(
        input: &mut In,
    ) -> xdr_codec::Result<(remote_storage_vol_create_xml_from_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_storage_vol_create_xml_from_ret {
                vol: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_storage_vol_create_xml_ret {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_storage_vol_create_xml_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_storage_vol_create_xml_ret {
                vol: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_storage_vol_delete_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_storage_vol_delete_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_storage_vol_delete_args {
                vol: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                flags: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_storage_vol_download_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_storage_vol_download_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_storage_vol_download_args {
                vol: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                offset: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                length: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                flags: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_storage_vol_get_info_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_storage_vol_get_info_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_storage_vol_get_info_args {
                vol: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_storage_vol_get_info_flags_args {
    fn unpack(
        input: &mut In,
    ) -> xdr_codec::Result<(remote_storage_vol_get_info_flags_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_storage_vol_get_info_flags_args {
                vol: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                flags: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_storage_vol_get_info_flags_ret {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_storage_vol_get_info_flags_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_storage_vol_get_info_flags_ret {
                type_: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                capacity: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                allocation: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_storage_vol_get_info_ret {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_storage_vol_get_info_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_storage_vol_get_info_ret {
                type_: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                capacity: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                allocation: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_storage_vol_get_path_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_storage_vol_get_path_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_storage_vol_get_path_args {
                vol: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_storage_vol_get_path_ret {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_storage_vol_get_path_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_storage_vol_get_path_ret {
                name: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_storage_vol_get_xml_desc_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_storage_vol_get_xml_desc_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_storage_vol_get_xml_desc_args {
                vol: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                flags: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_storage_vol_get_xml_desc_ret {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_storage_vol_get_xml_desc_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_storage_vol_get_xml_desc_ret {
                xml: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_storage_vol_lookup_by_key_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_storage_vol_lookup_by_key_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_storage_vol_lookup_by_key_args {
                key: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_storage_vol_lookup_by_key_ret {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_storage_vol_lookup_by_key_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_storage_vol_lookup_by_key_ret {
                vol: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_storage_vol_lookup_by_name_args {
    fn unpack(
        input: &mut In,
    ) -> xdr_codec::Result<(remote_storage_vol_lookup_by_name_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_storage_vol_lookup_by_name_args {
                pool: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                name: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_storage_vol_lookup_by_name_ret {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_storage_vol_lookup_by_name_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_storage_vol_lookup_by_name_ret {
                vol: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_storage_vol_lookup_by_path_args {
    fn unpack(
        input: &mut In,
    ) -> xdr_codec::Result<(remote_storage_vol_lookup_by_path_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_storage_vol_lookup_by_path_args {
                path: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_storage_vol_lookup_by_path_ret {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_storage_vol_lookup_by_path_ret, usize)> {
        let mut sz = 0;
        Ok((
            remote_storage_vol_lookup_by_path_ret {
                vol: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_storage_vol_resize_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_storage_vol_resize_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_storage_vol_resize_args {
                vol: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                capacity: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                flags: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_storage_vol_upload_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_storage_vol_upload_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_storage_vol_upload_args {
                vol: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                offset: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                length: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                flags: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_storage_vol_wipe_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_storage_vol_wipe_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_storage_vol_wipe_args {
                vol: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                flags: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_storage_vol_wipe_pattern_args {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_storage_vol_wipe_pattern_args, usize)> {
        let mut sz = 0;
        Ok((
            remote_storage_vol_wipe_pattern_args {
                vol: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                algorithm: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                flags: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_typed_param {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_typed_param, usize)> {
        let mut sz = 0;
        Ok((
            remote_typed_param {
                field: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                value: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_typed_param_value {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_typed_param_value, usize)> {
        let mut sz = 0;
        Ok((
            match {
                let (v, dsz): (i32, _) = xdr_codec::Unpack::unpack(input)?;
                sz += dsz;
                v
            } {
                x if x == (1i32 as i32) => remote_typed_param_value::Const1({
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                }),
                x if x == (2i32 as i32) => remote_typed_param_value::Const2({
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                }),
                x if x == (3i32 as i32) => remote_typed_param_value::Const3({
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                }),
                x if x == (4i32 as i32) => remote_typed_param_value::Const4({
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                }),
                x if x == (5i32 as i32) => remote_typed_param_value::Const5({
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                }),
                x if x == (6i32 as i32) => remote_typed_param_value::Const6({
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                }),
                x if x == (7i32 as i32) => remote_typed_param_value::Const7({
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                }),
                v => return Err(xdr_codec::Error::invalidcase(v as i32)),
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_uuid {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_uuid, usize)> {
        let mut sz = 0;
        Ok((
            {
                let (v, usz) = {
                    let mut buf: [u8; 16i64 as usize] = [0; 16];
                    let sz = xdr_codec::unpack_opaque_array(input, &mut buf[..], 16i64 as usize)?;
                    (buf, sz)
                };
                sz = usz;
                remote_uuid(v)
            },
            sz,
        ))
    }
}

impl<In: xdr_codec::Read> xdr_codec::Unpack<In> for remote_vcpu_info {
    fn unpack(input: &mut In) -> xdr_codec::Result<(remote_vcpu_info, usize)> {
        let mut sz = 0;
        Ok((
            remote_vcpu_info {
                number: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                state: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                cpu_time: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
                cpu: {
                    let (v, fsz) = xdr_codec::Unpack::unpack(input)?;
                    sz += fsz;
                    v
                },
            },
            sz,
        ))
    }
}
