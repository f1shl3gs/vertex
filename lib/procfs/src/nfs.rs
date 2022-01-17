use crate::{Error, ProcFS};
use tokio::io::AsyncBufReadExt;

/// Network models the "net" line.
#[derive(Debug, Default, PartialEq)]
pub struct Network {
    pub net_count: u64,
    pub udp_count: u64,
    pub tcp_count: u64,
    pub tcp_connect: u64,
}

impl TryFrom<Vec<u64>> for Network {
    type Error = Error;

    fn try_from(values: Vec<u64>) -> Result<Self, Self::Error> {
        if values.len() != 4 {
            return Err(Error::invalid_data("invalid Network"));
        }

        Ok(Network {
            net_count: values[0],
            udp_count: values[1],
            tcp_count: values[2],
            tcp_connect: values[3],
        })
    }
}

/// V2Stats models the "proc2" line.
#[derive(Debug, Default, PartialEq)]
pub struct V2Stats {
    pub null: u64,
    pub get_attr: u64,
    pub set_attr: u64,
    pub root: u64,
    pub lookup: u64,
    pub read_link: u64,
    pub read: u64,
    pub wr_cache: u64,
    pub write: u64,
    pub create: u64,
    pub remove: u64,
    pub rename: u64,
    pub link: u64,
    pub sym_link: u64,
    pub mkdir: u64,
    pub rmdir: u64,
    pub read_dir: u64,
    pub fs_stat: u64,
}

impl TryFrom<Vec<u64>> for V2Stats {
    type Error = Error;

    fn try_from(values: Vec<u64>) -> Result<Self, Self::Error> {
        let vs = values[0] as usize;
        if values.len() - 1 != vs || vs < 18 {
            return Err(Error::invalid_data("invalid V2Stats line"));
        }

        Ok(Self {
            null: values[1],
            get_attr: values[2],
            set_attr: values[3],
            root: values[4],
            lookup: values[5],
            read_link: values[6],
            read: values[7],
            wr_cache: values[8],
            write: values[9],
            create: values[10],
            remove: values[11],
            rename: values[12],
            link: values[13],
            sym_link: values[14],
            mkdir: values[15],
            rmdir: values[16],
            read_dir: values[17],
            fs_stat: values[18],
        })
    }
}

/// V3Stats models the "proc3" line.
#[derive(Debug, Default, PartialEq)]
pub struct V3Stats {
    pub null: u64,
    pub get_attr: u64,
    pub set_attr: u64,
    pub lookup: u64,
    pub access: u64,
    pub read_link: u64,
    pub read: u64,
    pub write: u64,
    pub create: u64,
    pub mkdir: u64,
    pub sym_link: u64,
    pub mknod: u64,
    pub remove: u64,
    pub rmdir: u64,
    pub rename: u64,
    pub link: u64,
    pub read_dir: u64,
    pub read_dir_plus: u64,
    pub fs_stat: u64,
    pub fs_info: u64,
    pub path_conf: u64,
    pub commit: u64,
}

impl TryFrom<Vec<u64>> for V3Stats {
    type Error = Error;

    fn try_from(values: Vec<u64>) -> Result<Self, Self::Error> {
        let vs = values[0] as usize;
        if values.len() - 1 != vs || vs < 22 {
            return Err(Error::invalid_data("invalid V3Stats line"));
        }

        Ok(V3Stats {
            null: values[1],
            get_attr: values[2],
            set_attr: values[3],
            lookup: values[4],
            access: values[5],
            read_link: values[6],
            read: values[7],
            write: values[8],
            create: values[9],
            mkdir: values[10],
            sym_link: values[11],
            mknod: values[12],
            remove: values[13],
            rmdir: values[14],
            rename: values[15],
            link: values[16],
            read_dir: values[17],
            read_dir_plus: values[18],
            fs_stat: values[19],
            fs_info: values[20],
            path_conf: values[21],
            commit: values[22],
        })
    }
}

/// ClientV4Stats models the nfs "proc4" line
#[derive(Debug, Default, PartialEq)]
pub struct ClientV4Stats {
    null: u64,
    read: u64,
    write: u64,
    commit: u64,
    open: u64,
    open_confirm: u64,
    open_noattr: u64,
    open_downgrade: u64,
    close: u64,
    setattr: u64,
    fs_info: u64,
    renew: u64,
    set_client_id: u64,
    set_client_id_confirm: u64,
    lock: u64,
    lockt: u64,
    locku: u64,
    access: u64,
    getattr: u64,
    lookup: u64,
    lookup_root: u64,
    remove: u64,
    rename: u64,
    link: u64,
    sym_link: u64,
    create: u64,
    pathconf: u64,
    stat_fs: u64,
    read_link: u64,
    read_dir: u64,
    server_caps: u64,
    deleg_return: u64,
    get_acl: u64,
    set_acl: u64,
    fs_locations: u64,
    release_lockowner: u64,
    secinfo: u64,
    fsid_present: u64,
    exchange_id: u64,
    create_session: u64,
    destroy_session: u64,
    sequence: u64,
    get_lease_time: u64,
    reclaim_complete: u64,
    layout_get: u64,
    get_device_info: u64,
    layout_commit: u64,
    layout_return: u64,
    secinfo_no_name: u64,
    test_state_id: u64,
    free_state_id: u64,
    get_device_list: u64,
    bind_conn_to_session: u64,
    destroy_client_id: u64,
    seek: u64,
    allocate: u64,
    deallocate: u64,
    layout_stats: u64,
    clone: u64,
}

impl TryFrom<Vec<u64>> for ClientV4Stats {
    type Error = Error;

    fn try_from(mut v: Vec<u64>) -> Result<Self, Self::Error> {
        let vs = v[0] as usize;
        if v.len() - 1 != vs {
            return Err(Error::invalid_data("invalid ClientV4Stats line"));
        }

        // This function currently supports mapping 59 NFS v4 client stats. Older
        // kernels may emit fewer stats, so we must detect this and pad out the
        // values to match the expected slice size.
        while v.len() < 60 {
            v.push(0);
        }

        Ok(Self {
            null: v[1],
            read: v[2],
            write: v[3],
            commit: v[4],
            open: v[5],
            open_confirm: v[6],
            open_noattr: v[7],
            open_downgrade: v[8],
            close: v[9],
            setattr: v[10],
            fs_info: v[11],
            renew: v[12],
            set_client_id: v[13],
            set_client_id_confirm: v[14],
            lock: v[15],
            lockt: v[16],
            locku: v[17],
            access: v[18],
            getattr: v[19],
            lookup: v[20],
            lookup_root: v[21],
            remove: v[22],
            rename: v[23],
            link: v[24],
            sym_link: v[25],
            create: v[26],
            pathconf: v[27],
            stat_fs: v[28],
            read_link: v[29],
            read_dir: v[30],
            server_caps: v[31],
            deleg_return: v[32],
            get_acl: v[33],
            set_acl: v[34],
            fs_locations: v[35],
            release_lockowner: v[36],
            secinfo: v[37],
            fsid_present: v[38],
            exchange_id: v[39],
            create_session: v[40],
            destroy_session: v[41],
            sequence: v[42],
            get_lease_time: v[43],
            reclaim_complete: v[44],
            layout_get: v[45],
            get_device_info: v[46],
            layout_commit: v[47],
            layout_return: v[48],
            secinfo_no_name: v[49],
            test_state_id: v[50],
            free_state_id: v[51],
            get_device_list: v[52],
            bind_conn_to_session: v[53],
            destroy_client_id: v[54],
            seek: v[55],
            allocate: v[56],
            deallocate: v[57],
            layout_stats: v[58],
            clone: v[59],
        })
    }
}

/// ClientRPC models the nfs "rpc" line
#[derive(Debug, Default, PartialEq)]
pub struct ClientRPC {
    rpc_count: u64,
    retransmissions: u64,
    auth_refreshes: u64,
}

impl TryFrom<Vec<u64>> for ClientRPC {
    type Error = Error;

    fn try_from(values: Vec<u64>) -> Result<Self, Self::Error> {
        if values.len() != 3 {
            return Err(Error::invalid_data("invalid RPC line"));
        }

        Ok(ClientRPC {
            rpc_count: values[0],
            retransmissions: values[1],
            auth_refreshes: values[2],
        })
    }
}

/// ClientRPCStats models all stats from /proc/net/rpc/nfs
#[derive(Debug, Default, PartialEq)]
pub struct ClientRPCStats {
    network: Network,
    client_rpc: ClientRPC,
    v2_stats: V2Stats,
    v3_stats: V3Stats,
    client_v4_stats: ClientV4Stats,
}

// ReplyCache models the "rc" line.
#[derive(Debug, Default, PartialEq)]
struct ReplyCache {
    hits: u64,
    misses: u64,
    no_cache: u64,
}

impl TryFrom<Vec<u64>> for ReplyCache {
    type Error = Error;

    fn try_from(v: Vec<u64>) -> Result<Self, Self::Error> {
        if v.len() != 3 {
            let msg = format!("invalid ReplyCache line {:?}", v);
            return Err(Error::invalid_data(msg));
        }

        Ok(Self {
            hits: v[0],
            misses: v[1],
            no_cache: v[2],
        })
    }
}

// FileHandles models the "fh" line.
#[derive(Debug, Default, PartialEq)]
struct FileHandles {
    stale: u64,
    total_lookups: u64,
    anon_lookups: u64,
    dir_no_cache: u64,
    no_dir_no_cache: u64,
}

impl TryFrom<Vec<u64>> for FileHandles {
    type Error = Error;

    fn try_from(v: Vec<u64>) -> Result<Self, Self::Error> {
        if v.len() != 5 {
            let msg = format!("invalid FileHandles line {:?}", v);
            return Err(Error::invalid_data(msg));
        }

        Ok(Self {
            stale: v[0],
            total_lookups: v[1],
            anon_lookups: v[2],
            dir_no_cache: v[3],
            no_dir_no_cache: v[4],
        })
    }
}

// Threads models the "th" line.
#[derive(Debug, Default, PartialEq)]
struct Threads {
    threads: u64,
    full_cnt: u64,
}

impl TryFrom<Vec<u64>> for Threads {
    type Error = Error;

    fn try_from(v: Vec<u64>) -> Result<Self, Self::Error> {
        if v.len() != 2 {
            let msg = format!("invalid Threads line {:?}", v);
            return Err(Error::invalid_data(msg));
        }

        Ok(Self {
            threads: v[0],
            full_cnt: v[1],
        })
    }
}

// ReadAheadCache models the "ra" line.
#[derive(Debug, Default, PartialEq)]
struct ReadAheadCache {
    cache_size: u64,
    cache_histogram: Vec<u64>,
    not_found: u64,
}

impl TryFrom<Vec<u64>> for ReadAheadCache {
    type Error = Error;

    fn try_from(v: Vec<u64>) -> Result<Self, Self::Error> {
        if v.len() != 12 {
            let msg = format!("invalid ReadAheadCache line {:?}", v);
            return Err(Error::invalid_data(msg));
        }

        Ok(Self {
            cache_size: v[0],
            cache_histogram: v[1..11].to_vec(),
            not_found: v[11],
        })
    }
}

// ServerRPC models the nfsd "rpc" line.
#[derive(Debug, Default, PartialEq)]
struct ServerRPC {
    rpc_count: u64,
    bad_cnt: u64,
    bad_fmt: u64,
    bad_auth: u64,
    badc_int: u64,
}

impl TryFrom<Vec<u64>> for ServerRPC {
    type Error = Error;

    fn try_from(v: Vec<u64>) -> Result<Self, Self::Error> {
        if v.len() != 5 {
            let msg = format!("invalid RPC line {:?}", v);
            return Err(Error::invalid_data(msg));
        }

        Ok(Self {
            rpc_count: v[0],
            bad_cnt: v[1],
            bad_fmt: v[2],
            bad_auth: v[3],
            badc_int: v[4],
        })
    }
}

// ServerV4Stats models the nfsd "proc4" line.
#[derive(Debug, Default, PartialEq)]
struct ServerV4Stats {
    null: u64,
    compound: u64,
}

impl TryFrom<Vec<u64>> for ServerV4Stats {
    type Error = Error;

    fn try_from(v: Vec<u64>) -> Result<Self, Self::Error> {
        let vs = v[0] as usize;
        if v.len() - 1 != vs || vs != 2 {
            let msg = format!("invalid V4Stats line {:?}", v);
            return Err(Error::invalid_data(msg));
        }

        Ok(Self {
            null: v[1],
            compound: v[2],
        })
    }
}

// V4Ops models the "proc4ops" line: NFSv4 operations
// Variable list, see:
// v4.0 https://tools.ietf.org/html/rfc3010 (38 operations)
// v4.1 https://tools.ietf.org/html/rfc5661 (58 operations)
// v4.2 https://tools.ietf.org/html/draft-ietf-nfsv4-minorversion2-41 (71 operations)
#[derive(Debug, Default, PartialEq)]
struct V4Ops {
    // values:       u64 // Variable depending on v4.x sub-version. TODO: Will this always at least include the fields in this struct?
    op0_unused: u64,
    op1_unused: u64,
    op2_future: u64,
    access: u64,
    close: u64,
    commit: u64,
    create: u64,
    deleg_purge: u64,
    deleg_return: u64,
    get_attr: u64,
    get_fh: u64,
    link: u64,
    lock: u64,
    lockt: u64,
    locku: u64,
    lookup: u64,
    lookup_root: u64,
    nverify: u64,
    open: u64,
    open_attr: u64,
    open_confirm: u64,
    open_dgrd: u64,
    put_fh: u64,
    put_pub_fh: u64,
    put_root_fh: u64,
    read: u64,
    readdir: u64,
    read_link: u64,
    remove: u64,
    rename: u64,
    renew: u64,
    restore_fh: u64,
    save_fh: u64,
    sec_info: u64,
    set_attr: u64,
    verify: u64,
    write: u64,
    rel_lock_owner: u64,
}

impl TryFrom<Vec<u64>> for V4Ops {
    type Error = Error;

    fn try_from(v: Vec<u64>) -> Result<Self, Self::Error> {
        let vs = v[0] as usize;
        if v.len() - 1 != vs || vs < 39 {
            let msg = format!("invalid V4Ops line {:?}", v);
            return Err(Error::invalid_data(msg));
        }

        Ok(Self {
            op0_unused: v[1],
            op1_unused: v[2],
            op2_future: v[3],
            access: v[4],
            close: v[5],
            commit: v[6],
            create: v[7],
            deleg_purge: v[8],
            deleg_return: v[9],
            get_attr: v[10],
            get_fh: v[11],
            link: v[12],
            lock: v[13],
            lockt: v[14],
            locku: v[15],
            lookup: v[16],
            lookup_root: v[17],
            nverify: v[18],
            open: v[19],
            open_attr: v[20],
            open_confirm: v[21],
            open_dgrd: v[22],
            put_fh: v[23],
            put_pub_fh: v[24],
            put_root_fh: v[25],
            read: v[26],
            readdir: v[27],
            read_link: v[28],
            remove: v[29],
            rename: v[30],
            renew: v[31],
            restore_fh: v[32],
            save_fh: v[33],
            sec_info: v[34],
            set_attr: v[35],
            verify: v[36],
            write: v[37],
            rel_lock_owner: v[38],
        })
    }
}

// InputOutput models the "io" line.
#[derive(Debug, Default, PartialEq)]
struct InputOutput {
    read: u64,
    write: u64,
}

impl TryFrom<Vec<u64>> for InputOutput {
    type Error = Error;

    fn try_from(v: Vec<u64>) -> Result<Self, Self::Error> {
        if v.len() != 2 {
            let msg = format!("invalid InputOutput line {:?}", v);
            return Err(Error::invalid_data(msg));
        }

        Ok(Self {
            read: v[0],
            write: v[1],
        })
    }
}

#[derive(Debug, Default, PartialEq)]
pub struct ServerRPCStats {
    reply_cache: ReplyCache,
    file_handles: FileHandles,
    input_output: InputOutput,
    threads: Threads,
    read_ahead_cache: ReadAheadCache,
    network: Network,
    server_rpc: ServerRPC,
    v2_stats: V2Stats,
    v3_stats: V3Stats,
    server_v4_stats: ServerV4Stats,
    v4_ops: V4Ops,
}

impl ProcFS {
    /// client_rpc_stats retrieves NFS client RPC statistics from file
    pub async fn nfs_client_rpc_stats(&self) -> Result<ClientRPCStats, Error> {
        let path = self.root.join("net/rpc/nfs");
        let f = tokio::fs::File::open(path).await?;
        let reader = tokio::io::BufReader::new(f);
        let mut lines = reader.lines();

        let mut stats = ClientRPCStats::default();
        while let Some(line) = lines.next_line().await? {
            let parts = line.trim().split_ascii_whitespace().collect::<Vec<_>>();

            if parts.len() < 2 {
                return Err(Error::invalid_data("invalid NFS metric line"));
            }

            // TODO: the error is not handled
            let values = parts
                .iter()
                .skip(1)
                .map(|p| (*p).parse::<u64>().unwrap_or(0))
                .collect::<Vec<_>>();

            match parts[0] {
                "net" => stats.network = values.try_into()?,
                "rpc" => stats.client_rpc = values.try_into()?,
                "proc2" => stats.v2_stats = values.try_into()?,
                "proc3" => stats.v3_stats = values.try_into()?,
                "proc4" => stats.client_v4_stats = values.try_into()?,
                _ => {
                    return Err(Error::invalid_data("errors parsing NFS metric line"));
                }
            }
        }

        Ok(stats)
    }

    pub async fn nfs_server_rpc_stats(&self) -> Result<ServerRPCStats, Error> {
        let path = self.root.join("net/rpc/nfsd");
        let f = tokio::fs::File::open(path).await?;
        let reader = tokio::io::BufReader::new(f);
        let mut lines = reader.lines();

        let mut stats = ServerRPCStats::default();
        while let Some(line) = lines.next_line().await? {
            let parts = line.trim().split_ascii_whitespace().collect::<Vec<_>>();

            if parts.len() < 2 {
                let msg = format!("invalid NFSd metric line {}", line);
                return Err(Error::invalid_data(msg));
            }

            let label = parts[0];
            let values = match label {
                "th" => {
                    if parts.len() < 3 {
                        let msg = format!("invalid NFSd th metric line {}", line);
                        return Err(Error::invalid_data(msg));
                    }

                    // TODO: handle the parse error
                    parts[1..3]
                        .iter()
                        .map(|p| (*p).parse::<u64>().unwrap_or(0))
                        .collect::<Vec<_>>()
                }
                _ => parts[1..]
                    .iter()
                    .map(|p| (*p).parse::<u64>().unwrap_or(0))
                    .collect::<Vec<_>>(),
            };

            let metric_line = parts[0];
            match metric_line {
                "rc" => stats.reply_cache = values.try_into()?,
                "fh" => stats.file_handles = values.try_into()?,
                "io" => stats.input_output = values.try_into()?,
                "th" => stats.threads = values.try_into()?,
                "ra" => stats.read_ahead_cache = values.try_into()?,
                "net" => stats.network = values.try_into()?,
                "rpc" => stats.server_rpc = values.try_into()?,
                "proc2" => stats.v2_stats = values.try_into()?,
                "proc3" => stats.v3_stats = values.try_into()?,
                "proc4" => stats.server_v4_stats = values.try_into()?,
                "proc4ops" => stats.v4_ops = values.try_into()?,
                _ => {
                    let msg = format!("errors parsing NFSd metric line {}", line);
                    return Err(Error::invalid_data(msg));
                }
            }
        }

        Ok(stats)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_client_rpc_stats() {
        struct Case {
            name: String,
            content: String,
            invalid: bool,
            stats: ClientRPCStats,
        }

        let cases = vec![
            Case {
                name: "invalid file".to_string(),
                content: "invalid".to_string(),
                invalid: true,
                stats: ClientRPCStats::default(),
            },
            Case {
                name: "good old kernel version file".to_string(),
                content: "net 70 70 69 45
rpc 1218785755 374636 1218815394
proc2 18 16 57 74 52 71 73 45 86 0 52 83 61 17 53 50 23 70 82
proc3 22 0 1061909262 48906 4077635 117661341 5 29391916 2570425 2993289 590 0 0 7815 15 1130 0 3983 92385 13332 2 1 23729
proc4 48 98 51 54 83 85 23 24 1 28 73 68 83 12 84 39 68 59 58 88 29 74 69 96 21 84 15 53 86 54 66 56 97 36 49 32 85 81 11 58 32 67 13 28 35 90 1 26 1337
".to_string(),
                invalid: false,
                stats: ClientRPCStats {
                    network: Network {
                        net_count: 70,
                        udp_count: 70,
                        tcp_count: 69,
                        tcp_connect: 45,
                    },
                    client_rpc: ClientRPC {
                        rpc_count: 1218785755,
                        retransmissions: 374636,
                        auth_refreshes: 1218815394,
                    },
                    v2_stats: V2Stats {
                        null: 16,
                        get_attr: 57,
                        set_attr: 74,
                        root: 52,
                        lookup: 71,
                        read_link: 73,
                        read: 45,
                        wr_cache: 86,
                        write: 0,
                        create: 52,
                        remove: 83,
                        rename: 61,
                        link: 17,
                        sym_link: 53,
                        mkdir: 50,
                        rmdir: 23,
                        read_dir: 70,
                        fs_stat: 82,
                    },
                    v3_stats: V3Stats {
                        null: 0,
                        get_attr: 1061909262,
                        set_attr: 48906,
                        lookup: 4077635,
                        access: 117661341,
                        read_link: 5,
                        read: 29391916,
                        write: 2570425,
                        create: 2993289,
                        mkdir: 590,
                        sym_link: 0,
                        mknod: 0,
                        remove: 7815,
                        rmdir: 15,
                        rename: 1130,
                        link: 0,
                        read_dir: 3983,
                        read_dir_plus: 92385,
                        fs_stat: 13332,
                        fs_info: 2,
                        path_conf: 1,
                        commit: 23729,
                    },
                    client_v4_stats: ClientV4Stats {
                        null: 98,
                        read: 51,
                        write: 54,
                        commit: 83,
                        open: 85,
                        open_confirm: 23,
                        open_noattr: 24,
                        open_downgrade: 1,
                        close: 28,
                        setattr: 73,
                        fs_info: 68,
                        renew: 83,
                        set_client_id: 12,
                        set_client_id_confirm: 84,
                        lock: 39,
                        lockt: 68,
                        locku: 59,
                        access: 58,
                        getattr: 88,
                        lookup: 29,
                        lookup_root: 74,
                        remove: 69,
                        rename: 96,
                        link: 21,
                        sym_link: 84,
                        create: 15,
                        pathconf: 53,
                        stat_fs: 86,
                        read_link: 54,
                        read_dir: 66,
                        server_caps: 56,
                        deleg_return: 97,
                        get_acl: 36,
                        set_acl: 49,
                        fs_locations: 32,
                        release_lockowner: 85,
                        secinfo: 81,
                        fsid_present: 11,
                        exchange_id: 58,
                        create_session: 32,
                        destroy_session: 67,
                        sequence: 13,
                        get_lease_time: 28,
                        reclaim_complete: 35,
                        layout_get: 90,
                        get_device_info: 1,
                        layout_commit: 26,
                        layout_return: 1337,
                        secinfo_no_name: 0,
                        test_state_id: 0,
                        free_state_id: 0,
                        get_device_list: 0,
                        bind_conn_to_session: 0,
                        destroy_client_id: 0,
                        seek: 0,
                        allocate: 0,
                        deallocate: 0,
                        layout_stats: 0,
                        clone: 0,
                    },
                },
            },
            Case {
                name: "good file".to_string(),
                content: "net 18628 0 18628 6
rpc 4329785 0 4338291
proc2 18 2 69 0 0 4410 0 0 0 0 0 0 0 0 0 0 0 99 2
proc3 22 1 4084749 29200 94754 32580 186 47747 7981 8639 0 6356 0 6962 0 7958 0 0 241 4 4 2 39
proc4 61 1 0 0 0 0 0 0 0 0 0 0 0 1 1 0 0 0 0 0 0 0 2 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0
".to_string(),
                invalid: false,
                stats: ClientRPCStats {
                    network: Network {
                        net_count: 18628,
                        udp_count: 0,
                        tcp_count: 18628,
                        tcp_connect: 6,
                    },
                    client_rpc: ClientRPC {
                        rpc_count: 4329785,
                        retransmissions: 0,
                        auth_refreshes: 4338291,
                    },
                    v2_stats: V2Stats {
                        null: 2,
                        get_attr: 69,
                        set_attr: 0,
                        root: 0,
                        lookup: 4410,
                        read_link: 0,
                        read: 0,
                        wr_cache: 0,
                        write: 0,
                        create: 0,
                        remove: 0,
                        rename: 0,
                        link: 0,
                        sym_link: 0,
                        mkdir: 0,
                        rmdir: 0,
                        read_dir: 99,
                        fs_stat: 2,
                    },
                    v3_stats: V3Stats {
                        null: 1,
                        get_attr: 4084749,
                        set_attr: 29200,
                        lookup: 94754,
                        access: 32580,
                        read_link: 186,
                        read: 47747,
                        write: 7981,
                        create: 8639,
                        mkdir: 0,
                        sym_link: 6356,
                        mknod: 0,
                        remove: 6962,
                        rmdir: 0,
                        rename: 7958,
                        link: 0,
                        read_dir: 0,
                        read_dir_plus: 241,
                        fs_stat: 4,
                        fs_info: 4,
                        path_conf: 2,
                        commit: 39,
                    },
                    client_v4_stats: ClientV4Stats {
                        null: 1,
                        read: 0,
                        write: 0,
                        commit: 0,
                        open: 0,
                        open_confirm: 0,
                        open_noattr: 0,
                        open_downgrade: 0,
                        close: 0,
                        setattr: 0,
                        fs_info: 0,
                        renew: 0,
                        set_client_id: 1,
                        set_client_id_confirm: 1,
                        lock: 0,
                        lockt: 0,
                        locku: 0,
                        access: 0,
                        getattr: 0,
                        lookup: 0,
                        lookup_root: 0,
                        remove: 2,
                        rename: 0,
                        link: 0,
                        sym_link: 0,
                        create: 0,
                        pathconf: 0,
                        stat_fs: 0,
                        read_link: 0,
                        read_dir: 0,
                        server_caps: 0,
                        deleg_return: 0,
                        get_acl: 0,
                        set_acl: 0,
                        fs_locations: 0,
                        release_lockowner: 0,
                        secinfo: 0,
                        fsid_present: 0,
                        exchange_id: 0,
                        create_session: 0,
                        destroy_session: 0,
                        sequence: 0,
                        get_lease_time: 0,
                        reclaim_complete: 0,
                        layout_get: 0,
                        get_device_info: 0,
                        layout_commit: 0,
                        layout_return: 0,
                        secinfo_no_name: 0,
                        test_state_id: 0,
                        free_state_id: 0,
                        get_device_list: 0,
                        bind_conn_to_session: 0,
                        destroy_client_id: 0,
                        seek: 0,
                        allocate: 0,
                        deallocate: 0,
                        layout_stats: 0,
                        clone: 0,
                    },
                },
            },
        ];

        for case in cases {
            // prepare nfs file
            let tmpdir = tempdir().unwrap();
            let root = tmpdir.path();
            std::fs::create_dir_all(root.join("net/rpc")).unwrap();
            let path = root.join("net/rpc/nfs");
            std::fs::write(path, case.content).unwrap();

            let procfs = ProcFS::new(root);
            let result = procfs.nfs_client_rpc_stats().await;
            if case.invalid && result.is_err() {
                continue;
            }

            assert_eq!(result.unwrap(), case.stats)
        }
    }

    #[tokio::test]
    async fn test_server_rpc_stats() {
        struct Case {
            name: String,
            content: String,
            stats: ServerRPCStats,
            invalid: bool,
        }

        let cases = vec![
            Case {
                name: "invalid file".to_string(),
                content: "invalid".to_string(),
                stats: Default::default(),
                invalid: true,
            },
            Case {
                name: "good file".to_string(),
                content: "rc 0 6 18622
fh 0 0 0 0 0
io 157286400 0
th 8 0 0.000 0.000 0.000 0.000 0.000 0.000 0.000 0.000 0.000 0.000
ra 32 0 0 0 0 0 0 0 0 0 0 0
net 18628 0 18628 6
rpc 18628 0 0 0 0
proc2 18 2 69 0 0 4410 0 0 0 0 0 0 0 0 0 0 0 99 2
proc3 22 2 112 0 2719 111 0 0 0 0 0 0 0 0 0 0 0 27 216 0 2 1 0
proc4 2 2 10853
proc4ops 72 0 0 0 1098 2 0 0 0 0 8179 5896 0 0 0 0 5900 0 0 2 0 2 0 9609 0 2 150 1272 0 0 0 1236 0 0 0 0 3 3 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0
".to_string(),
                stats: ServerRPCStats {
                    reply_cache: ReplyCache {
                        hits: 0,
                        misses: 6,
                        no_cache: 18622,
                    },
                    file_handles: FileHandles {
                        stale: 0,
                        total_lookups: 0,
                        anon_lookups: 0,
                        dir_no_cache: 0,
                        no_dir_no_cache: 0,
                    },
                    input_output: InputOutput {
                        read: 157286400,
                        write: 0,
                    },
                    threads: Threads {
                        threads: 8,
                        full_cnt: 0,
                    },
                    read_ahead_cache: ReadAheadCache {
                        cache_size: 32,
                        cache_histogram: vec![0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
                        not_found: 0,
                    },
                    network: Network {
                        net_count: 18628,
                        udp_count: 0,
                        tcp_count: 18628,
                        tcp_connect: 6,
                    },
                    server_rpc: ServerRPC {
                        rpc_count: 18628,
                        bad_cnt: 0,
                        bad_fmt: 0,
                        bad_auth: 0,
                        badc_int: 0,
                    },
                    v2_stats: V2Stats {
                        null: 2,
                        get_attr: 69,
                        set_attr: 0,
                        root: 0,
                        lookup: 4410,
                        read_link: 0,
                        read: 0,
                        wr_cache: 0,
                        write: 0,
                        create: 0,
                        remove: 0,
                        rename: 0,
                        link: 0,
                        sym_link: 0,
                        mkdir: 0,
                        rmdir: 0,
                        read_dir: 99,
                        fs_stat: 2,
                    },
                    v3_stats: V3Stats {
                        null: 2,
                        get_attr: 112,
                        set_attr: 0,
                        lookup: 2719,
                        access: 111,
                        read_link: 0,
                        read: 0,
                        write: 0,
                        create: 0,
                        mkdir: 0,
                        sym_link: 0,
                        mknod: 0,
                        remove: 0,
                        rmdir: 0,
                        rename: 0,
                        link: 0,
                        read_dir: 27,
                        read_dir_plus: 216,
                        fs_stat: 0,
                        fs_info: 2,
                        path_conf: 1,
                        commit: 0,
                    },
                    server_v4_stats: ServerV4Stats {
                        null: 2,
                        compound: 10853,
                    },
                    v4_ops: V4Ops {
                        op0_unused: 0,
                        op1_unused: 0,
                        op2_future: 0,
                        access: 1098,
                        close: 2,
                        commit: 0,
                        create: 0,
                        deleg_purge: 0,
                        deleg_return: 0,
                        get_attr: 8179,
                        get_fh: 5896,
                        link: 0,
                        lock: 0,
                        lockt: 0,
                        locku: 0,
                        lookup: 5900,
                        lookup_root: 0,
                        nverify: 0,
                        open: 2,
                        open_attr: 0,
                        open_confirm: 2,
                        open_dgrd: 0,
                        put_fh: 9609,
                        put_pub_fh: 0,
                        put_root_fh: 2,
                        read: 150,
                        readdir: 1272,
                        read_link: 0,
                        remove: 0,
                        rename: 0,
                        renew: 1236,
                        restore_fh: 0,
                        save_fh: 0,
                        sec_info: 0,
                        set_attr: 0,
                        verify: 3,
                        write: 3,
                        rel_lock_owner: 0,
                    },
                },
                invalid: false,
            },
        ];

        for case in cases {
            // prepare nfsd file
            let tmpdir = tempdir().unwrap();
            let root = tmpdir.path();
            std::fs::create_dir_all(root.join("net/rpc")).unwrap();
            let path = root.join("net/rpc/nfsd");
            std::fs::write(path, case.content).unwrap();

            let procfs = ProcFS::new(root);
            let result = procfs.nfs_server_rpc_stats().await;
            if case.invalid {
                assert_eq!(result.is_err(), true);
            } else {
                assert_eq!(result.unwrap(), case.stats);
            }
        }
    }
}
