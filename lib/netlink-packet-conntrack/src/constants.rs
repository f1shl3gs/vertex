/// https://github.com/torvalds/linux/blob/dbe69e43372212527abf48609aba7fc39a6daa27/include/uapi/linux/netfilter/nfnetlink.h#L51

pub const NFNL_SUBSYS_NONE: u8 = 0;
pub const NFNL_SUBSYS_CTNETLINK: u8 = 1;
pub const NFNL_SUBSYS_CTNETLINK_EXP: u8 = 2;
pub const NFNL_SUBSYS_QUEUE: u8 = 3;
pub const NFNL_SUBSYS_ULOG: u8 = 4;
pub const NFNL_SUBSYS_OSF: u8 = 5;
pub const NFNL_SUBSYS_IPSET: u8 = 6;
pub const NFNL_SUBSYS_ACCT: u8 = 7;
pub const NFNL_SUBSYS_CTNETLINK_TIMEOUT: u8 = 8;
pub const NFNL_SUBSYS_CTHELPER: u8 = 9;
pub const NFNL_SUBSYS_NFTABLES: u8 = 10;
pub const NFNL_SUBSYS_NFT_COMPAT: u8 = 11;
pub const NFNL_SUBSYS_HOOK: u8 = 12;
pub const NFNL_SUBSYS_COUNT: u8 = 13;


/// https://github.com/torvalds/linux/blob/9e9fb7655ed585da8f468e29221f0ba194a5f613/include/uapi/linux/netfilter/nfnetlink_conntrack.h
pub const IPCTNL_MSG_CT_NEW: u8 = 0;
pub const IPCTNL_MSG_CT_GET: u8 = 1;
pub const IPCTNL_MSG_CT_DELETE: u8 = 2;
pub const IPCTNL_MSG_CT_GET_CTRZERO: u8 = 3;
pub const IPCTNL_MSG_CT_GET_STATS_CPU: u16 = 4;
pub const IPCTNL_MSG_CT_GET_STATS: u16 = 5;
pub const IPCTNL_MSG_CT_GET_DYING: u8 = 6;
pub const IPCTNL_MSG_CT_GET_UNCONFIRMED: u8 = 7;
