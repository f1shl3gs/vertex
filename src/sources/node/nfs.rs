use crate::{
    event::Metric,
    invalid_error
};
use crate::sources::node::errors::{
    Error,
};
use tokio::io::AsyncBufReadExt;
use std::convert::TryFrom;
use std::path::Path;
use std::convert::TryInto;


/// Network models the "net" line.
#[derive(Debug, Default, PartialEq)]
pub struct Network {
    net_count: u64,
    udp_count: u64,
    tcp_count: u64,
    tcp_connect: u64,
}

impl TryFrom<Vec<u64>> for Network {
    type Error = Error;

    fn try_from(values: Vec<u64>) -> Result<Self, Self::Error> {
        if values.len() != 4 {
            return invalid_error!("invalid Network {:?}", values);
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
    null: u64,
    get_attr: u64,
    set_attr: u64,
    root: u64,
    lookup: u64,
    read_link: u64,
    read: u64,
    wr_cache: u64,
    write: u64,
    create: u64,
    remove: u64,
    rename: u64,
    link: u64,
    sym_link: u64,
    mkdir: u64,
    rmdir: u64,
    read_dir: u64,
    fs_stat: u64,
}

impl V2Stats {
    fn new(values: Vec<u64>) -> Result<Self, Error> {
        let vs = values[0] as usize;
        if values.len() - 1 != vs || vs < 18 {
            return Err(Error::new_invalid("invalid V2Stats line"));
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
    null: u64,
    get_attr: u64,
    set_attr: u64,
    lookup: u64,
    access: u64,
    read_link: u64,
    read: u64,
    write: u64,
    create: u64,
    mkdir: u64,
    sym_link: u64,
    mknod: u64,
    remove: u64,
    rmdir: u64,
    rename: u64,
    link: u64,
    read_dir: u64,
    read_dir_plus: u64,
    fs_stat: u64,
    fs_info: u64,
    path_conf: u64,
    commit: u64,
}

impl V3Stats {
    fn new(values: Vec<u64>) -> Result<Self, Error> {
        let vs = values[0] as usize;
        if values.len() - 1 != vs || vs < 22 {
            return Err(Error::new_invalid("invalid V3Stats line"));
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
            return invalid_error!("invalid ClientV4Stats line {:?}", v);
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

impl ClientRPC {
    fn new(values: Vec<u64>) -> Result<Self, Error> {
        if values.len() != 3 {
            return Err(Error::new_invalid("invalid RPC line"));
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

pub async fn client_rpc_stats<P: AsRef<Path>>(path: P) -> Result<ClientRPCStats, Error> {
    let f = tokio::fs::File::open(&path).await?;
    let reader = tokio::io::BufReader::new(f);
    let mut lines = reader.lines();

    let mut stats = ClientRPCStats::default();
    while let Some(line) = lines.next_line().await? {
        let parts = line
            .trim()
            .split_ascii_whitespace()
            .collect::<Vec<_>>();

        if parts.len() < 2 {
            println!("{}", line);
            return invalid_error!("invalid NFS metric line, {}", line);
        }

        // TODO: the error is not handled
        let values = parts
            .iter()
            .skip(1)
            .map(|p| (*p).parse::<u64>().unwrap_or(0))
            .collect::<Vec<_>>();

        match parts[0] {
            "net" => stats.network = values.try_into()?,

            "rpc" => stats.client_rpc = ClientRPC::new(values)?,

            "proc2" => stats.v2_stats = V2Stats::new(values)?,

            "proc3" => stats.v3_stats = V3Stats::new(values)?,

            "proc4" => stats.client_v4_stats = values.try_into()?,

            _ => {
                return Err(Error::new_invalid("errors parsing NFS metric line"));
            }
        }
    }

    Ok(stats)
}

async fn gather() -> Result<Vec<Metric>, Error> {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

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
                stats: ClientRPCStats::default()
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
                        tcp_connect: 45
                    },
                    client_rpc: ClientRPC {
                        rpc_count: 1218785755,
                        retransmissions: 374636,
                        auth_refreshes: 1218815394
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
                    }
                }
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
                    network: Network{
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
                    v3_stats: V3Stats{
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
                    }
                }
            }
        ];

        for case in cases {
            let path = Path::new(case.name.as_str());
            std::fs::write(path, case.content).unwrap();

            let result = client_rpc_stats(path).await;
            if case.invalid && result.is_err() {
                continue;
            }

            assert_eq!(result.unwrap(), case.stats)
        }
    }
}