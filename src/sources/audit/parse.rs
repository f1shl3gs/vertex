use std::collections::BTreeMap;
use std::fmt::Display;

use bytes::Bytes;
use chrono::{DateTime, Utc};
use value::{Value, value};

use super::syscalls::{arch_name, syscall_name};

const SELINUX_SUBJ_KEY_SUFFIXES: &[&str] = &["_user", "_role", "_domain", "_level", "_category"];

pub fn message_typ(id: u16) -> Option<&'static str> {
    const TYPES: [(u16, &str); 232] = [
        (1000, "get"),
        (1001, "set"),
        (1002, "list"),
        (1003, "add"),
        (1004, "del"),
        (1005, "user"),
        (1006, "login"),
        (1007, "watch_ins"),
        (1008, "watch_rem"),
        (1009, "watch_list"),
        (1010, "signal_info"),
        (1011, "add_rule"),
        (1012, "del_rule"),
        (1013, "list_rules"),
        (1014, "trim"),
        (1015, "make_equiv"),
        (1016, "tty_get"),
        (1017, "tty_set"),
        (1018, "set_feature"),
        (1019, "get_feature"),
        (1100, "user_auth"),
        (1101, "user_acct"),
        (1102, "user_mgmt"),
        (1103, "cred_acq"),
        (1104, "cred_disp"),
        (1105, "user_start"),
        (1106, "user_end"),
        (1107, "user_avc"),
        (1108, "user_chauthtok"),
        (1109, "user_err"),
        (1110, "cred_refr"),
        (1111, "usys_config"),
        (1112, "user_login"),
        (1113, "user_logout"),
        (1114, "add_user"),
        (1115, "del_user"),
        (1116, "add_group"),
        (1117, "del_group"),
        (1118, "dac_check"),
        (1119, "chgrp_id"),
        (1120, "test"),
        (1121, "trusted_app"),
        (1122, "user_selinux_err"),
        (1123, "user_cmd"),
        (1124, "user_tty"),
        (1125, "chuser_id"),
        (1126, "grp_auth"),
        (1127, "system_boot"),
        (1128, "system_shutdown"),
        (1129, "system_runlevel"),
        (1130, "service_start"),
        (1131, "service_stop"),
        (1132, "grp_mgmt"),
        (1133, "grp_chauthtok"),
        (1134, "mac_check"),
        (1135, "acct_lock"),
        (1136, "acct_unlock"),
        (1137, "user_device"),
        (1138, "software_update"),
        (1199, "last_user_msg"),
        (1200, "daemon_start"),
        (1201, "daemon_end"),
        (1202, "daemon_abort"),
        (1203, "daemon_config"),
        (1204, "daemon_reconfig"),
        (1205, "daemon_rotate"),
        (1206, "daemon_resume"),
        (1207, "daemon_accept"),
        (1208, "daemon_close"),
        (1209, "daemon_err"),
        (1299, "last_daemon"),
        (1300, "syscall"),
        (1302, "path"),
        (1303, "ipc"),
        (1304, "socketcall"),
        (1305, "config_change"),
        (1306, "sockaddr"),
        (1307, "cwd"),
        (1309, "execve"),
        (1311, "ipc_set_perm"),
        (1312, "mq_open"),
        (1313, "mq_sendrecv"),
        (1314, "mq_notify"),
        (1315, "mq_getsetattr"),
        (1316, "kernel_other"),
        (1317, "fd_pair"),
        (1318, "obj_pid"),
        (1319, "tty"),
        (1320, "eoe"),
        (1321, "bprm_fcaps"),
        (1322, "capset"),
        (1323, "mmap"),
        (1324, "netfilter_pkt"),
        (1325, "netfilter_cfg"),
        (1326, "seccomp"),
        (1327, "proctitle"),
        (1328, "feature_change"),
        (1329, "replace"),
        (1330, "kern_module"),
        (1331, "fanotify"),
        (1332, "time_injoffset"),
        (1333, "time_adjntpval"),
        (1334, "bpf"),
        (1335, "event_listener"),
        (1336, "uringop"),
        (1337, "openat2"),
        (1338, "dm_ctrl"),
        (1339, "dm_event"),
        (1399, "last_event"),
        (1400, "avc"),
        (1401, "selinux_err"),
        (1402, "avc_path"),
        (1403, "mac_policy_load"),
        (1404, "mac_status"),
        (1405, "mac_config_change"),
        (1406, "mac_unlbl_allow"),
        (1407, "mac_cipsov4_add"),
        (1408, "mac_cipsov4_del"),
        (1409, "mac_map_add"),
        (1410, "mac_map_del"),
        (1411, "mac_ipsec_addsa"),
        (1412, "mac_ipsec_delsa"),
        (1413, "mac_ipsec_addspd"),
        (1414, "mac_ipsec_delspd"),
        (1415, "mac_ipsec_event"),
        (1416, "mac_unlbl_stcadd"),
        (1417, "mac_unlbl_stcdel"),
        (1418, "mac_calipso_add"),
        (1419, "mac_calipso_del"),
        (1499, "last_selinux"),
        (1500, "aa"),
        (1501, "apparmor_audit"),
        (1502, "apparmor_allowed"),
        (1503, "apparmor_denied"),
        (1504, "apparmor_hint"),
        (1505, "apparmor_status"),
        (1506, "apparmor_error"),
        (1507, "apparmor_kill"),
        (1599, "last_apparmor"),
        (1600, "first_kern_crypto_msg"),
        (1699, "last_kern_crypto_msg"),
        (1700, "anom_promiscuous"),
        (1701, "anom_abend"),
        (1702, "anom_link"),
        (1703, "anom_creat"),
        (1799, "last_kern_anom_msg"),
        (1800, "integrity_data"),
        (1801, "integrity_metadata"),
        (1802, "integrity_status"),
        (1803, "integrity_hash"),
        (1804, "integrity_pcr"),
        (1805, "integrity_rule"),
        (1806, "integrity_evm_xattr"),
        (1807, "integrity_policy_rule"),
        (1899, "integrity_last_msg"),
        (2000, "kernel"),
        (2100, "anom_login_failures"),
        (2101, "anom_login_time"),
        (2102, "anom_login_sessions"),
        (2103, "anom_login_acct"),
        (2104, "anom_login_location"),
        (2105, "anom_max_dac"),
        (2106, "anom_max_mac"),
        (2107, "anom_amtu_fail"),
        (2108, "anom_rbac_fail"),
        (2109, "anom_rbac_integrity_fail"),
        (2110, "anom_crypto_fail"),
        (2111, "anom_access_fs"),
        (2112, "anom_exec"),
        (2113, "anom_mk_exec"),
        (2114, "anom_add_acct"),
        (2115, "anom_del_acct"),
        (2116, "anom_mod_acct"),
        (2117, "anom_root_trans"),
        (2118, "anom_login_service"),
        (2119, "anom_login_root"),
        (2120, "anom_origin_failures"),
        (2121, "anom_session"),
        (2199, "last_anom_msg"),
        (2200, "resp_anomaly"),
        (2201, "resp_alert"),
        (2202, "resp_kill_proc"),
        (2203, "resp_term_access"),
        (2204, "resp_acct_remote"),
        (2205, "resp_acct_lock_timed"),
        (2206, "resp_acct_unlock_timed"),
        (2207, "resp_acct_lock"),
        (2208, "resp_term_lock"),
        (2209, "resp_sebool"),
        (2210, "resp_exec"),
        (2211, "resp_single"),
        (2212, "resp_halt"),
        (2213, "resp_origin_block"),
        (2214, "resp_origin_block_timed"),
        (2215, "resp_origin_unblock_timed"),
        (2299, "last_anom_resp"),
        (2300, "user_role_change"),
        (2301, "role_assign"),
        (2302, "role_remove"),
        (2303, "label_override"),
        (2304, "label_level_change"),
        (2305, "user_labeled_export"),
        (2306, "user_unlabeled_export"),
        (2307, "dev_alloc"),
        (2308, "dev_dealloc"),
        (2309, "fs_relabel"),
        (2310, "user_mac_policy_load"),
        (2311, "role_modify"),
        (2312, "user_mac_config_change"),
        (2313, "user_mac_status"),
        (2399, "last_user_lspp_msg"),
        (2400, "crypto_test_user"),
        (2401, "crypto_param_change_user"),
        (2402, "crypto_login"),
        (2403, "crypto_logout"),
        (2404, "crypto_key_user"),
        (2405, "crypto_failure_user"),
        (2406, "crypto_replay_user"),
        (2407, "crypto_session"),
        (2408, "crypto_ike_sa"),
        (2409, "crypto_ipsec_sa"),
        (2499, "last_crypto_msg"),
        (2500, "virt_control"),
        (2501, "virt_resource"),
        (2502, "virt_machine_id"),
        (2503, "virt_integrity_check"),
        (2504, "virt_create"),
        (2505, "virt_destroy"),
        (2506, "virt_migrate_in"),
        (2507, "virt_migrate_out"),
        (2599, "last_virt_msg"),
        (2999, "last_user_msg2"),
    ];

    TYPES
        .binary_search_by(|(tid, _)| tid.cmp(&id))
        .map(|index| TYPES[index].1)
        .ok()
}

#[derive(Debug)]
pub enum Error<'a> {
    SplitAudit,
    Timestamp(&'a [u8]),
}

impl Display for Error<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::SplitAudit => f.write_str("cannot split audit part"),
            Error::Timestamp(part) => write!(
                f,
                "invalid timestamp part \"{}\"",
                String::from_utf8_lossy(part)
            ),
        }
    }
}

// audit(1744907888.074:9277): pid=2886492 uid=0 auid=1000 tty=pts6 ses=3 subj=unconfined_u:unconfined_r:unconfined_t:s0-s0:c0.c1023 comm="vertex-worker" exe="/path/to/vertex" nl-mcgrp=1 op=connect res=1
//
// `exe` in the netlink response is not hex encoded
pub fn parse(input: &[u8]) -> Result<Value, Error<'_>> {
    let id = u16::from_ne_bytes(input[4..6].try_into().unwrap());
    let msg_typ = match message_typ(id) {
        None => Bytes::from(format!("UNKNOWN[{id}]")),
        Some(value) => Bytes::from_static(value.as_ref()),
    };

    let input = &input[16..];
    let raw_msg = input;

    // try split audit
    let Some(index) = input.iter().position(|c| c.is_ascii_whitespace()) else {
        return Err(Error::SplitAudit);
    };

    let Some((timestamp, sequence)) = parse_timestamp(&input[..index]) else {
        return Err(Error::Timestamp(&input[..index]));
    };
    let input = &input[index + 1..];

    let mut data = parse_pairs(input);
    if let Some(value) = data.remove("msg")
        && let Some(input) = value.as_bytes()
    {
        let pairs = parse_pairs(input.as_ref());

        for (key, value) in pairs {
            data.insert(key, value);
        }
    }

    Ok(value!({
        timestamp: timestamp,
        sequence: sequence,
        data: data,
        record_type: msg_typ,
        raw_msg: raw_msg,
    }))
}

fn parse_pairs(input: &[u8]) -> BTreeMap<String, Value> {
    let mut fields = BTreeMap::<String, Value>::new();
    let mut buf = String::new();
    let mut key = None;

    let mut escape = false;
    let mut quoted = false;
    let mut garbage = false;

    for ch in input {
        let ch = *ch as char;

        match (quoted, ch) {
            (false, ' ') => {
                if noop_buf(&buf) {
                    buf.clear();
                    continue;
                }

                if !garbage {
                    // the buffer that we just processed is either a value or a valueless
                    // key depending on the current state of `pair`
                    if let Some(key) = key {
                        if key == "syscall" {
                            let value = syscall_name(&buf).unwrap_or("UNKNOWN");
                            fields.insert(key, value.into());
                        } else if key == "arch" {
                            let value = arch_name(&buf).unwrap_or("UNKNOWN");
                            fields.insert(key, value.into());
                        } else if key == "auid" {
                            if buf == "4294967295" || buf == "-1" {
                                fields.insert(key, "unset".into());
                            } else {
                                fields.insert(key, buf.into());
                            }
                        } else if key == "subj" {
                            for (suffix, value) in
                                SELINUX_SUBJ_KEY_SUFFIXES.iter().zip(buf.split(":"))
                            {
                                fields.insert(format!("subj{suffix}"), value.into());
                            }
                        } else {
                            fields.insert(key, buf.into());
                        }
                    }

                    key = None;
                }

                buf = String::new();
                garbage = false;
            }
            (false, '=') => {
                if !buf.is_empty() {
                    key = Some(buf);
                    buf = String::new();
                } else {
                    garbage = true;
                }
            }
            (true, '\\') => {
                escape = true;
            }
            (_, '"') => {
                if escape {
                    buf.push(ch);
                    escape = false;
                } else {
                    quoted = !quoted;
                }
            }
            (_, '\'') => {
                if escape {
                    buf.push(ch);
                    escape = false;
                } else {
                    quoted = !quoted;
                }
            }
            _ => {
                // if the last character we read was an escape, but this character was not
                // a quote, then store the escape back into the buffer
                if escape {
                    buf.push('\\');
                    escape = false;
                }

                buf.push(ch);
            }
        }
    }

    // and process one final time at the end of the message to get the last data point
    if !garbage && !noop_buf(&buf) {
        match key {
            Some(key) if key == "subj" => {
                if key == "subj" {
                    for (suffix, value) in SELINUX_SUBJ_KEY_SUFFIXES.iter().zip(buf.split(":")) {
                        fields.insert(format!("subj{suffix}"), value.into());
                    }
                }
            }
            Some(key) => {
                let value = if key == "syscall" {
                    syscall_name(&buf).unwrap_or("UNKNOWN").into()
                } else if key == "arch" {
                    arch_name(&buf).unwrap_or("UNKNOWN").into()
                } else if key == "auid" && (buf == "4294967295" || buf == "-1") {
                    "unset".into()
                } else {
                    buf.into()
                };

                fields.insert(key, value);
            }
            None => {}
        }
    }

    fields
}

#[inline]
fn noop_buf(buf: &str) -> bool {
    buf.is_empty() || buf == "?" || buf == "?," || buf == "(null)"
}

// input audit(1744907888.074:9277)
fn parse_timestamp(input: &[u8]) -> Option<(DateTime<Utc>, i64)> {
    let input = input.strip_prefix(b"audit(")?;
    let input = input.strip_suffix(b"):")?;

    let parts = input
        .splitn(3, |c| *c == b'.' || *c == b':')
        .collect::<Vec<_>>();

    if parts.len() != 3 {
        return None;
    }

    let secs = parse_number(parts[0]);
    let msecs = parse_number(parts[1]);
    let seq = parse_number(parts[2]);

    let ts = DateTime::from_timestamp_millis(secs * 1000 + msecs)?;

    Some((ts, seq))
}

fn parse_number(input: &[u8]) -> i64 {
    let mut num = 0i64;

    for n in input {
        num = num * 10 + (*n - b'0') as i64;
    }

    num
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_msg() {
        let input = br#"unit=NetworkManager-dispatcher comm="systemd" exe="/usr/lib/systemd/systemd" hostname=? addr=? terminal=? res=success"#;
        let value = parse_pairs(input);

        assert_eq!(
            value.get("unit").unwrap().to_string_lossy(),
            "NetworkManager-dispatcher"
        );
        assert_eq!(value.get("comm").unwrap().to_string_lossy(), "systemd");
        assert_eq!(
            value.get("exe").unwrap().to_string_lossy(),
            "/usr/lib/systemd/systemd"
        );
        assert_eq!(value.get("res").unwrap().to_string_lossy(), "success");
    }

    #[test]
    fn simple() {
        // audit(1771592947.631:4632): prog-id=837 op=UNLOAD
        let input: [u8; 65] = [
            65, 0, 0, 0, 54, 5, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 97, 117, 100, 105, 116, 40, 49, 55,
            55, 49, 53, 57, 50, 57, 52, 55, 46, 54, 51, 49, 58, 52, 54, 51, 50, 41, 58, 32, 112,
            114, 111, 103, 45, 105, 100, 61, 56, 51, 55, 32, 111, 112, 61, 85, 78, 76, 79, 65, 68,
        ];
        let got = parse(&input).unwrap();

        assert_eq!(
            got.get("data").unwrap(),
            &value!({
                "op": "UNLOAD",
                "prog-id": "837"
            })
        )
    }

    #[test]
    fn with_msg() {
        // ["_user", "_role", "_domain", "_level", "_category"];
        // audit(1744987302.959:9600): pid=1 uid=0 auid=4294967295 ses=4294967295 subj=system_u:system_r:init_t:s0 msg='unit=NetworkManager-dispatcher comm="systemd" exe="/usr/lib/systemd/systemd" hostname=? addr=? terminal=? res=success'
        let input = [
            243, 0, 0, 0, 107, 4, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 97, 117, 100, 105, 116, 40, 49, 55,
            52, 52, 57, 56, 55, 51, 48, 50, 46, 57, 53, 57, 58, 57, 54, 48, 48, 41, 58, 32, 112,
            105, 100, 61, 49, 32, 117, 105, 100, 61, 48, 32, 97, 117, 105, 100, 61, 52, 50, 57, 52,
            57, 54, 55, 50, 57, 53, 32, 115, 101, 115, 61, 52, 50, 57, 52, 57, 54, 55, 50, 57, 53,
            32, 115, 117, 98, 106, 61, 115, 121, 115, 116, 101, 109, 95, 117, 58, 115, 121, 115,
            116, 101, 109, 95, 114, 58, 105, 110, 105, 116, 95, 116, 58, 115, 48, 32, 109, 115,
            103, 61, 39, 117, 110, 105, 116, 61, 78, 101, 116, 119, 111, 114, 107, 77, 97, 110, 97,
            103, 101, 114, 45, 100, 105, 115, 112, 97, 116, 99, 104, 101, 114, 32, 99, 111, 109,
            109, 61, 34, 115, 121, 115, 116, 101, 109, 100, 34, 32, 101, 120, 101, 61, 34, 47, 117,
            115, 114, 47, 108, 105, 98, 47, 115, 121, 115, 116, 101, 109, 100, 47, 115, 121, 115,
            116, 101, 109, 100, 34, 32, 104, 111, 115, 116, 110, 97, 109, 101, 61, 63, 32, 97, 100,
            100, 114, 61, 63, 32, 116, 101, 114, 109, 105, 110, 97, 108, 61, 63, 32, 114, 101, 115,
            61, 115, 117, 99, 99, 101, 115, 115, 39,
        ];
        let got = parse(&input).unwrap();

        assert_eq!(
            got.get("data").unwrap(),
            &value!({
                "pid": "1",
                "uid": "0",
                "auid": "unset",
                "ses": "4294967295",
                "subj_user": "system_u",
                "subj_role": "system_r",
                "subj_domain": "init_t",
                "subj_level": "s0",
                "unit": "NetworkManager-dispatcher",
                "comm": "systemd",
                "exe": "/usr/lib/systemd/systemd",
                "res": "success",
            })
        );
    }
}
