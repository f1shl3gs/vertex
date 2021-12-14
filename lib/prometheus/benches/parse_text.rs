use criterion::{criterion_group, criterion_main, Criterion, Throughput};
use pprof::criterion::{Output, PProfProfiler};

const INPUT: &str = r##"# HELP go_gc_duration_seconds A summary of the pause duration of garbage collection cycles.
# TYPE go_gc_duration_seconds summary
go_gc_duration_seconds{quantile="0"} 0
go_gc_duration_seconds{quantile="0.25"} 0
go_gc_duration_seconds{quantile="0.5"} 0
go_gc_duration_seconds{quantile="0.75"} 0
go_gc_duration_seconds{quantile="1"} 0
go_gc_duration_seconds_sum 0
go_gc_duration_seconds_count 0
# HELP go_goroutines Number of goroutines that currently exist.
# TYPE go_goroutines gauge
go_goroutines 7
# HELP go_info Information about the Go environment.
# TYPE go_info gauge
go_info{version="go1.17"} 1
# HELP go_memstats_alloc_bytes Number of bytes allocated and still in use.
# TYPE go_memstats_alloc_bytes gauge
go_memstats_alloc_bytes 1.501088e+06
# HELP go_memstats_alloc_bytes_total Total number of bytes allocated, even if freed.
# TYPE go_memstats_alloc_bytes_total counter
go_memstats_alloc_bytes_total 1.501088e+06
# HELP go_memstats_buck_hash_sys_bytes Number of bytes used by the profiling bucket hash table.
# TYPE go_memstats_buck_hash_sys_bytes gauge
go_memstats_buck_hash_sys_bytes 1.446494e+06
# HELP go_memstats_frees_total Total number of frees.
# TYPE go_memstats_frees_total counter
go_memstats_frees_total 746
# HELP go_memstats_gc_cpu_fraction The fraction of this program's available CPU time used by the GC since the program started.
# TYPE go_memstats_gc_cpu_fraction gauge
go_memstats_gc_cpu_fraction 0
# HELP go_memstats_gc_sys_bytes Number of bytes used for garbage collection system metadata.
# TYPE go_memstats_gc_sys_bytes gauge
go_memstats_gc_sys_bytes 4.122512e+06
# HELP go_memstats_heap_alloc_bytes Number of heap bytes allocated and still in use.
# TYPE go_memstats_heap_alloc_bytes gauge
go_memstats_heap_alloc_bytes 1.501088e+06
# HELP go_memstats_heap_idle_bytes Number of heap bytes waiting to be used.
# TYPE go_memstats_heap_idle_bytes gauge
go_memstats_heap_idle_bytes 5.103616e+06
# HELP go_memstats_heap_inuse_bytes Number of heap bytes that are in use.
# TYPE go_memstats_heap_inuse_bytes gauge
go_memstats_heap_inuse_bytes 2.891776e+06
# HELP go_memstats_heap_objects Number of allocated objects.
# TYPE go_memstats_heap_objects gauge
go_memstats_heap_objects 9199
# HELP go_memstats_heap_released_bytes Number of heap bytes released to OS.
# TYPE go_memstats_heap_released_bytes gauge
go_memstats_heap_released_bytes 5.103616e+06
# HELP go_memstats_heap_sys_bytes Number of heap bytes obtained from system.
# TYPE go_memstats_heap_sys_bytes gauge
go_memstats_heap_sys_bytes 7.995392e+06
# HELP go_memstats_last_gc_time_seconds Number of seconds since 1970 of last garbage collection.
# TYPE go_memstats_last_gc_time_seconds gauge
go_memstats_last_gc_time_seconds 0
# HELP go_memstats_lookups_total Total number of pointer lookups.
# TYPE go_memstats_lookups_total counter
go_memstats_lookups_total 0
# HELP go_memstats_mallocs_total Total number of mallocs.
# TYPE go_memstats_mallocs_total counter
go_memstats_mallocs_total 9945
# HELP go_memstats_mcache_inuse_bytes Number of bytes in use by mcache structures.
# TYPE go_memstats_mcache_inuse_bytes gauge
go_memstats_mcache_inuse_bytes 19200
# HELP go_memstats_mcache_sys_bytes Number of bytes used for mcache structures obtained from system.
# TYPE go_memstats_mcache_sys_bytes gauge
go_memstats_mcache_sys_bytes 32768
# HELP go_memstats_mspan_inuse_bytes Number of bytes in use by mspan structures.
# TYPE go_memstats_mspan_inuse_bytes gauge
go_memstats_mspan_inuse_bytes 55896
# HELP go_memstats_mspan_sys_bytes Number of bytes used for mspan structures obtained from system.
# TYPE go_memstats_mspan_sys_bytes gauge
go_memstats_mspan_sys_bytes 65536
# HELP go_memstats_next_gc_bytes Number of heap bytes when next garbage collection will take place.
# TYPE go_memstats_next_gc_bytes gauge
go_memstats_next_gc_bytes 4.473924e+06
# HELP go_memstats_other_sys_bytes Number of bytes used for other system allocations.
# TYPE go_memstats_other_sys_bytes gauge
go_memstats_other_sys_bytes 1.10057e+06
# HELP go_memstats_stack_inuse_bytes Number of bytes in use by the stack allocator.
# TYPE go_memstats_stack_inuse_bytes gauge
go_memstats_stack_inuse_bytes 393216
# HELP go_memstats_stack_sys_bytes Number of bytes obtained from system for stack allocator.
# TYPE go_memstats_stack_sys_bytes gauge
go_memstats_stack_sys_bytes 393216
# HELP go_memstats_sys_bytes Number of bytes obtained from system.
# TYPE go_memstats_sys_bytes gauge
go_memstats_sys_bytes 1.5156488e+07
# HELP go_threads Number of OS threads created.
# TYPE go_threads gauge
go_threads 7
# HELP node_arp_entries ARP entries by device
# TYPE node_arp_entries gauge
node_arp_entries{device="docker0"} 1
node_arp_entries{device="virbr1"} 1
# HELP node_boot_time_seconds Node boot time, in unixtime.
# TYPE node_boot_time_seconds gauge
node_boot_time_seconds 1.63452308e+09
# HELP node_context_switches_total Total number of context switches.
# TYPE node_context_switches_total counter
node_context_switches_total 7.624853415e+09
# HELP node_cooling_device_cur_state Current throttle state of the cooling device
# TYPE node_cooling_device_cur_state gauge
node_cooling_device_cur_state{name="0",type="Processor"} 0
node_cooling_device_cur_state{name="1",type="Processor"} 0
node_cooling_device_cur_state{name="10",type="Processor"} 0
node_cooling_device_cur_state{name="11",type="Processor"} 0
node_cooling_device_cur_state{name="12",type="Processor"} 0
node_cooling_device_cur_state{name="13",type="Processor"} 0
node_cooling_device_cur_state{name="14",type="Processor"} 0
node_cooling_device_cur_state{name="15",type="Processor"} 0
node_cooling_device_cur_state{name="2",type="Processor"} 0
node_cooling_device_cur_state{name="3",type="Processor"} 0
node_cooling_device_cur_state{name="4",type="Processor"} 0
node_cooling_device_cur_state{name="5",type="Processor"} 0
node_cooling_device_cur_state{name="6",type="Processor"} 0
node_cooling_device_cur_state{name="7",type="Processor"} 0
node_cooling_device_cur_state{name="8",type="Processor"} 0
node_cooling_device_cur_state{name="9",type="Processor"} 0
# HELP node_cooling_device_max_state Maximum throttle state of the cooling device
# TYPE node_cooling_device_max_state gauge
node_cooling_device_max_state{name="0",type="Processor"} 10
node_cooling_device_max_state{name="1",type="Processor"} 10
node_cooling_device_max_state{name="10",type="Processor"} 10
node_cooling_device_max_state{name="11",type="Processor"} 10
node_cooling_device_max_state{name="12",type="Processor"} 10
node_cooling_device_max_state{name="13",type="Processor"} 10
node_cooling_device_max_state{name="14",type="Processor"} 10
node_cooling_device_max_state{name="15",type="Processor"} 10
node_cooling_device_max_state{name="2",type="Processor"} 10
node_cooling_device_max_state{name="3",type="Processor"} 10
node_cooling_device_max_state{name="4",type="Processor"} 10
node_cooling_device_max_state{name="5",type="Processor"} 10
node_cooling_device_max_state{name="6",type="Processor"} 10
node_cooling_device_max_state{name="7",type="Processor"} 10
node_cooling_device_max_state{name="8",type="Processor"} 10
node_cooling_device_max_state{name="9",type="Processor"} 10
# HELP node_cpu_frequency_max_hertz Maximum cpu thread frequency in hertz.
# TYPE node_cpu_frequency_max_hertz gauge
node_cpu_frequency_max_hertz{cpu="0"} 4e+09
node_cpu_frequency_max_hertz{cpu="1"} 4e+09
node_cpu_frequency_max_hertz{cpu="10"} 4e+09
node_cpu_frequency_max_hertz{cpu="11"} 4e+09
node_cpu_frequency_max_hertz{cpu="12"} 4e+09
node_cpu_frequency_max_hertz{cpu="13"} 4e+09
node_cpu_frequency_max_hertz{cpu="14"} 4e+09
node_cpu_frequency_max_hertz{cpu="15"} 4e+09
node_cpu_frequency_max_hertz{cpu="2"} 4e+09
node_cpu_frequency_max_hertz{cpu="3"} 4e+09
node_cpu_frequency_max_hertz{cpu="4"} 4e+09
node_cpu_frequency_max_hertz{cpu="5"} 4e+09
node_cpu_frequency_max_hertz{cpu="6"} 4e+09
node_cpu_frequency_max_hertz{cpu="7"} 4e+09
node_cpu_frequency_max_hertz{cpu="8"} 4e+09
node_cpu_frequency_max_hertz{cpu="9"} 4e+09
# HELP node_cpu_frequency_min_hertz Minimum cpu thread frequency in hertz.
# TYPE node_cpu_frequency_min_hertz gauge
node_cpu_frequency_min_hertz{cpu="0"} 2.2e+09
node_cpu_frequency_min_hertz{cpu="1"} 2.2e+09
node_cpu_frequency_min_hertz{cpu="10"} 2.2e+09
node_cpu_frequency_min_hertz{cpu="11"} 2.2e+09
node_cpu_frequency_min_hertz{cpu="12"} 2.2e+09
node_cpu_frequency_min_hertz{cpu="13"} 2.2e+09
node_cpu_frequency_min_hertz{cpu="14"} 2.2e+09
node_cpu_frequency_min_hertz{cpu="15"} 2.2e+09
node_cpu_frequency_min_hertz{cpu="2"} 2.2e+09
node_cpu_frequency_min_hertz{cpu="3"} 2.2e+09
node_cpu_frequency_min_hertz{cpu="4"} 2.2e+09
node_cpu_frequency_min_hertz{cpu="5"} 2.2e+09
node_cpu_frequency_min_hertz{cpu="6"} 2.2e+09
node_cpu_frequency_min_hertz{cpu="7"} 2.2e+09
node_cpu_frequency_min_hertz{cpu="8"} 2.2e+09
node_cpu_frequency_min_hertz{cpu="9"} 2.2e+09
# HELP node_cpu_guest_seconds_total Seconds the CPUs spent in guests (VMs) for each mode.
# TYPE node_cpu_guest_seconds_total counter
node_cpu_guest_seconds_total{cpu="0",mode="nice"} 0
node_cpu_guest_seconds_total{cpu="0",mode="user"} 0
node_cpu_guest_seconds_total{cpu="1",mode="nice"} 0
node_cpu_guest_seconds_total{cpu="1",mode="user"} 0
node_cpu_guest_seconds_total{cpu="10",mode="nice"} 0
node_cpu_guest_seconds_total{cpu="10",mode="user"} 0
node_cpu_guest_seconds_total{cpu="11",mode="nice"} 0
node_cpu_guest_seconds_total{cpu="11",mode="user"} 0
node_cpu_guest_seconds_total{cpu="12",mode="nice"} 0
node_cpu_guest_seconds_total{cpu="12",mode="user"} 0
node_cpu_guest_seconds_total{cpu="13",mode="nice"} 0
node_cpu_guest_seconds_total{cpu="13",mode="user"} 0
node_cpu_guest_seconds_total{cpu="14",mode="nice"} 0
node_cpu_guest_seconds_total{cpu="14",mode="user"} 0
node_cpu_guest_seconds_total{cpu="15",mode="nice"} 0
node_cpu_guest_seconds_total{cpu="15",mode="user"} 0
node_cpu_guest_seconds_total{cpu="2",mode="nice"} 0
node_cpu_guest_seconds_total{cpu="2",mode="user"} 0
node_cpu_guest_seconds_total{cpu="3",mode="nice"} 0
node_cpu_guest_seconds_total{cpu="3",mode="user"} 0
node_cpu_guest_seconds_total{cpu="4",mode="nice"} 0
node_cpu_guest_seconds_total{cpu="4",mode="user"} 0
node_cpu_guest_seconds_total{cpu="5",mode="nice"} 0
node_cpu_guest_seconds_total{cpu="5",mode="user"} 0
node_cpu_guest_seconds_total{cpu="6",mode="nice"} 0
node_cpu_guest_seconds_total{cpu="6",mode="user"} 0
node_cpu_guest_seconds_total{cpu="7",mode="nice"} 0
node_cpu_guest_seconds_total{cpu="7",mode="user"} 0
node_cpu_guest_seconds_total{cpu="8",mode="nice"} 0
node_cpu_guest_seconds_total{cpu="8",mode="user"} 0
node_cpu_guest_seconds_total{cpu="9",mode="nice"} 0
node_cpu_guest_seconds_total{cpu="9",mode="user"} 0
# HELP node_cpu_scaling_frequency_hertz Current scaled CPU thread frequency in hertz.
# TYPE node_cpu_scaling_frequency_hertz gauge
node_cpu_scaling_frequency_hertz{cpu="0"} 3.993075e+09
node_cpu_scaling_frequency_hertz{cpu="1"} 3.911477e+09
node_cpu_scaling_frequency_hertz{cpu="10"} 3.992172e+09
node_cpu_scaling_frequency_hertz{cpu="11"} 3.9794e+09
node_cpu_scaling_frequency_hertz{cpu="12"} 3.993197e+09
node_cpu_scaling_frequency_hertz{cpu="13"} 3.936269e+09
node_cpu_scaling_frequency_hertz{cpu="14"} 3.976514e+09
node_cpu_scaling_frequency_hertz{cpu="15"} 3.986516e+09
node_cpu_scaling_frequency_hertz{cpu="2"} 3.993735e+09
node_cpu_scaling_frequency_hertz{cpu="3"} 3.976894e+09
node_cpu_scaling_frequency_hertz{cpu="4"} 3.994824e+09
node_cpu_scaling_frequency_hertz{cpu="5"} 3.929901e+09
node_cpu_scaling_frequency_hertz{cpu="6"} 3.99123e+09
node_cpu_scaling_frequency_hertz{cpu="7"} 3.989795e+09
node_cpu_scaling_frequency_hertz{cpu="8"} 3.992795e+09
node_cpu_scaling_frequency_hertz{cpu="9"} 3.696201e+09
# HELP node_cpu_scaling_frequency_max_hertz Maximum scaled CPU thread frequency in hertz.
# TYPE node_cpu_scaling_frequency_max_hertz gauge
node_cpu_scaling_frequency_max_hertz{cpu="0"} 4e+09
node_cpu_scaling_frequency_max_hertz{cpu="1"} 4e+09
node_cpu_scaling_frequency_max_hertz{cpu="10"} 4e+09
node_cpu_scaling_frequency_max_hertz{cpu="11"} 4e+09
node_cpu_scaling_frequency_max_hertz{cpu="12"} 4e+09
node_cpu_scaling_frequency_max_hertz{cpu="13"} 4e+09
node_cpu_scaling_frequency_max_hertz{cpu="14"} 4e+09
node_cpu_scaling_frequency_max_hertz{cpu="15"} 4e+09
node_cpu_scaling_frequency_max_hertz{cpu="2"} 4e+09
node_cpu_scaling_frequency_max_hertz{cpu="3"} 4e+09
node_cpu_scaling_frequency_max_hertz{cpu="4"} 4e+09
node_cpu_scaling_frequency_max_hertz{cpu="5"} 4e+09
node_cpu_scaling_frequency_max_hertz{cpu="6"} 4e+09
node_cpu_scaling_frequency_max_hertz{cpu="7"} 4e+09
node_cpu_scaling_frequency_max_hertz{cpu="8"} 4e+09
node_cpu_scaling_frequency_max_hertz{cpu="9"} 4e+09
# HELP node_cpu_scaling_frequency_min_hertz Minimum scaled CPU thread frequency in hertz.
# TYPE node_cpu_scaling_frequency_min_hertz gauge
node_cpu_scaling_frequency_min_hertz{cpu="0"} 2.2e+09
node_cpu_scaling_frequency_min_hertz{cpu="1"} 2.2e+09
node_cpu_scaling_frequency_min_hertz{cpu="10"} 2.2e+09
node_cpu_scaling_frequency_min_hertz{cpu="11"} 2.2e+09
node_cpu_scaling_frequency_min_hertz{cpu="12"} 2.2e+09
node_cpu_scaling_frequency_min_hertz{cpu="13"} 2.2e+09
node_cpu_scaling_frequency_min_hertz{cpu="14"} 2.2e+09
node_cpu_scaling_frequency_min_hertz{cpu="15"} 2.2e+09
node_cpu_scaling_frequency_min_hertz{cpu="2"} 2.2e+09
node_cpu_scaling_frequency_min_hertz{cpu="3"} 2.2e+09
node_cpu_scaling_frequency_min_hertz{cpu="4"} 2.2e+09
node_cpu_scaling_frequency_min_hertz{cpu="5"} 2.2e+09
node_cpu_scaling_frequency_min_hertz{cpu="6"} 2.2e+09
node_cpu_scaling_frequency_min_hertz{cpu="7"} 2.2e+09
node_cpu_scaling_frequency_min_hertz{cpu="8"} 2.2e+09
node_cpu_scaling_frequency_min_hertz{cpu="9"} 2.2e+09
# HELP node_cpu_seconds_total Seconds the CPUs spent in each mode.
# TYPE node_cpu_seconds_total counter
node_cpu_seconds_total{cpu="0",mode="idle"} 244633.16
node_cpu_seconds_total{cpu="0",mode="iowait"} 42.81
node_cpu_seconds_total{cpu="0",mode="irq"} 173.11
node_cpu_seconds_total{cpu="0",mode="nice"} 0.65
node_cpu_seconds_total{cpu="0",mode="softirq"} 392.95
node_cpu_seconds_total{cpu="0",mode="steal"} 0
node_cpu_seconds_total{cpu="0",mode="system"} 4144.86
node_cpu_seconds_total{cpu="0",mode="user"} 10894.09
node_cpu_seconds_total{cpu="1",mode="idle"} 244202.94
node_cpu_seconds_total{cpu="1",mode="iowait"} 48.03
node_cpu_seconds_total{cpu="1",mode="irq"} 230.67
node_cpu_seconds_total{cpu="1",mode="nice"} 1.31
node_cpu_seconds_total{cpu="1",mode="softirq"} 197.71
node_cpu_seconds_total{cpu="1",mode="steal"} 0
node_cpu_seconds_total{cpu="1",mode="system"} 4369.46
node_cpu_seconds_total{cpu="1",mode="user"} 11285.18
node_cpu_seconds_total{cpu="10",mode="idle"} 244244.81
node_cpu_seconds_total{cpu="10",mode="iowait"} 49.97
node_cpu_seconds_total{cpu="10",mode="irq"} 327.64
node_cpu_seconds_total{cpu="10",mode="nice"} 1.27
node_cpu_seconds_total{cpu="10",mode="softirq"} 93.26
node_cpu_seconds_total{cpu="10",mode="steal"} 0
node_cpu_seconds_total{cpu="10",mode="system"} 4531.77
node_cpu_seconds_total{cpu="10",mode="user"} 10923.34
node_cpu_seconds_total{cpu="11",mode="idle"} 244617.33
node_cpu_seconds_total{cpu="11",mode="iowait"} 46.69
node_cpu_seconds_total{cpu="11",mode="irq"} 246.29
node_cpu_seconds_total{cpu="11",mode="nice"} 1.53
node_cpu_seconds_total{cpu="11",mode="softirq"} 93.75
node_cpu_seconds_total{cpu="11",mode="steal"} 0
node_cpu_seconds_total{cpu="11",mode="system"} 4400.65
node_cpu_seconds_total{cpu="11",mode="user"} 10785.25
node_cpu_seconds_total{cpu="12",mode="idle"} 246444.56
node_cpu_seconds_total{cpu="12",mode="iowait"} 50.44
node_cpu_seconds_total{cpu="12",mode="irq"} 190.82
node_cpu_seconds_total{cpu="12",mode="nice"} 0.9
node_cpu_seconds_total{cpu="12",mode="softirq"} 240.29
node_cpu_seconds_total{cpu="12",mode="steal"} 0
node_cpu_seconds_total{cpu="12",mode="system"} 4143.21
node_cpu_seconds_total{cpu="12",mode="user"} 9210.16
node_cpu_seconds_total{cpu="13",mode="idle"} 247098.04
node_cpu_seconds_total{cpu="13",mode="iowait"} 54.16
node_cpu_seconds_total{cpu="13",mode="irq"} 200.44
node_cpu_seconds_total{cpu="13",mode="nice"} 0.95
node_cpu_seconds_total{cpu="13",mode="softirq"} 103.39
node_cpu_seconds_total{cpu="13",mode="steal"} 0
node_cpu_seconds_total{cpu="13",mode="system"} 4511.82
node_cpu_seconds_total{cpu="13",mode="user"} 8275.62
node_cpu_seconds_total{cpu="14",mode="idle"} 245579.55
node_cpu_seconds_total{cpu="14",mode="iowait"} 52.32
node_cpu_seconds_total{cpu="14",mode="irq"} 195.64
node_cpu_seconds_total{cpu="14",mode="nice"} 1.17
node_cpu_seconds_total{cpu="14",mode="softirq"} 105.59
node_cpu_seconds_total{cpu="14",mode="steal"} 0
node_cpu_seconds_total{cpu="14",mode="system"} 4294.84
node_cpu_seconds_total{cpu="14",mode="user"} 9909.95
node_cpu_seconds_total{cpu="15",mode="idle"} 245325.72
node_cpu_seconds_total{cpu="15",mode="iowait"} 56.37
node_cpu_seconds_total{cpu="15",mode="irq"} 169.45
node_cpu_seconds_total{cpu="15",mode="nice"} 1.28
node_cpu_seconds_total{cpu="15",mode="softirq"} 97.97
node_cpu_seconds_total{cpu="15",mode="steal"} 0
node_cpu_seconds_total{cpu="15",mode="system"} 4117.76
node_cpu_seconds_total{cpu="15",mode="user"} 10421.02
node_cpu_seconds_total{cpu="2",mode="idle"} 245360.24
node_cpu_seconds_total{cpu="2",mode="iowait"} 49.43
node_cpu_seconds_total{cpu="2",mode="irq"} 162.17
node_cpu_seconds_total{cpu="2",mode="nice"} 1.44
node_cpu_seconds_total{cpu="2",mode="softirq"} 144.41
node_cpu_seconds_total{cpu="2",mode="steal"} 0
node_cpu_seconds_total{cpu="2",mode="system"} 4319.94
node_cpu_seconds_total{cpu="2",mode="user"} 10177.32
node_cpu_seconds_total{cpu="3",mode="idle"} 244799.66
node_cpu_seconds_total{cpu="3",mode="iowait"} 47.9
node_cpu_seconds_total{cpu="3",mode="irq"} 182.69
node_cpu_seconds_total{cpu="3",mode="nice"} 2.26
node_cpu_seconds_total{cpu="3",mode="softirq"} 119.42
node_cpu_seconds_total{cpu="3",mode="steal"} 0
node_cpu_seconds_total{cpu="3",mode="system"} 4312.7
node_cpu_seconds_total{cpu="3",mode="user"} 10768.94
node_cpu_seconds_total{cpu="4",mode="idle"} 244491.3
node_cpu_seconds_total{cpu="4",mode="iowait"} 51.34
node_cpu_seconds_total{cpu="4",mode="irq"} 217.8
node_cpu_seconds_total{cpu="4",mode="nice"} 0.86
node_cpu_seconds_total{cpu="4",mode="softirq"} 123.34
node_cpu_seconds_total{cpu="4",mode="steal"} 0
node_cpu_seconds_total{cpu="4",mode="system"} 4492.26
node_cpu_seconds_total{cpu="4",mode="user"} 10861.93
node_cpu_seconds_total{cpu="5",mode="idle"} 243983.55
node_cpu_seconds_total{cpu="5",mode="iowait"} 53.44
node_cpu_seconds_total{cpu="5",mode="irq"} 236.85
node_cpu_seconds_total{cpu="5",mode="nice"} 1.54
node_cpu_seconds_total{cpu="5",mode="softirq"} 116.01
node_cpu_seconds_total{cpu="5",mode="steal"} 0
node_cpu_seconds_total{cpu="5",mode="system"} 4817.84
node_cpu_seconds_total{cpu="5",mode="user"} 11119.25
node_cpu_seconds_total{cpu="6",mode="idle"} 244988.68
node_cpu_seconds_total{cpu="6",mode="iowait"} 51.25
node_cpu_seconds_total{cpu="6",mode="irq"} 203.93
node_cpu_seconds_total{cpu="6",mode="nice"} 0.64
node_cpu_seconds_total{cpu="6",mode="softirq"} 111.42
node_cpu_seconds_total{cpu="6",mode="steal"} 0
node_cpu_seconds_total{cpu="6",mode="system"} 4599.82
node_cpu_seconds_total{cpu="6",mode="user"} 10228.18
node_cpu_seconds_total{cpu="7",mode="idle"} 245477.44
node_cpu_seconds_total{cpu="7",mode="iowait"} 51.24
node_cpu_seconds_total{cpu="7",mode="irq"} 169.4
node_cpu_seconds_total{cpu="7",mode="nice"} 0.98
node_cpu_seconds_total{cpu="7",mode="softirq"} 102.42
node_cpu_seconds_total{cpu="7",mode="steal"} 0
node_cpu_seconds_total{cpu="7",mode="system"} 4131.65
node_cpu_seconds_total{cpu="7",mode="user"} 10253.42
node_cpu_seconds_total{cpu="8",mode="idle"} 244211.43
node_cpu_seconds_total{cpu="8",mode="iowait"} 43.28
node_cpu_seconds_total{cpu="8",mode="irq"} 166.15
node_cpu_seconds_total{cpu="8",mode="nice"} 0.91
node_cpu_seconds_total{cpu="8",mode="softirq"} 94.97
node_cpu_seconds_total{cpu="8",mode="steal"} 0
node_cpu_seconds_total{cpu="8",mode="system"} 4311.34
node_cpu_seconds_total{cpu="8",mode="user"} 11435.74
node_cpu_seconds_total{cpu="9",mode="idle"} 244594.67
node_cpu_seconds_total{cpu="9",mode="iowait"} 46
node_cpu_seconds_total{cpu="9",mode="irq"} 164.95
node_cpu_seconds_total{cpu="9",mode="nice"} 1.35
node_cpu_seconds_total{cpu="9",mode="softirq"} 99.35
node_cpu_seconds_total{cpu="9",mode="steal"} 0
node_cpu_seconds_total{cpu="9",mode="system"} 4361.85
node_cpu_seconds_total{cpu="9",mode="user"} 10946.5
# HELP node_disk_discard_time_seconds_total This is the total number of seconds spent by all discards.
# TYPE node_disk_discard_time_seconds_total counter
node_disk_discard_time_seconds_total{device="dm-0"} 0
node_disk_discard_time_seconds_total{device="dm-1"} 0
node_disk_discard_time_seconds_total{device="dm-2"} 0
node_disk_discard_time_seconds_total{device="nvme0n1"} 0
# HELP node_disk_discarded_sectors_total The total number of sectors discarded successfully.
# TYPE node_disk_discarded_sectors_total counter
node_disk_discarded_sectors_total{device="dm-0"} 0
node_disk_discarded_sectors_total{device="dm-1"} 0
node_disk_discarded_sectors_total{device="dm-2"} 0
node_disk_discarded_sectors_total{device="nvme0n1"} 0
# HELP node_disk_discards_completed_total The total number of discards completed successfully.
# TYPE node_disk_discards_completed_total counter
node_disk_discards_completed_total{device="dm-0"} 0
node_disk_discards_completed_total{device="dm-1"} 0
node_disk_discards_completed_total{device="dm-2"} 0
node_disk_discards_completed_total{device="nvme0n1"} 0
# HELP node_disk_discards_merged_total The total number of discards merged.
# TYPE node_disk_discards_merged_total counter
node_disk_discards_merged_total{device="dm-0"} 0
node_disk_discards_merged_total{device="dm-1"} 0
node_disk_discards_merged_total{device="dm-2"} 0
node_disk_discards_merged_total{device="nvme0n1"} 0
# HELP node_disk_io_now The number of I/Os currently in progress.
# TYPE node_disk_io_now gauge
node_disk_io_now{device="dm-0"} 0
node_disk_io_now{device="dm-1"} 0
node_disk_io_now{device="dm-2"} 0
node_disk_io_now{device="nvme0n1"} 0
# HELP node_disk_io_time_seconds_total Total seconds spent doing I/Os.
# TYPE node_disk_io_time_seconds_total counter
node_disk_io_time_seconds_total{device="dm-0"} 443.183
node_disk_io_time_seconds_total{device="dm-1"} 61.794000000000004
node_disk_io_time_seconds_total{device="dm-2"} 621.394
node_disk_io_time_seconds_total{device="nvme0n1"} 1068.534
# HELP node_disk_io_time_weighted_seconds_total The weighted # of seconds spent doing I/Os.
# TYPE node_disk_io_time_weighted_seconds_total counter
node_disk_io_time_weighted_seconds_total{device="dm-0"} 6223.402
node_disk_io_time_weighted_seconds_total{device="dm-1"} 2050.091
node_disk_io_time_weighted_seconds_total{device="dm-2"} 4108.752
node_disk_io_time_weighted_seconds_total{device="nvme0n1"} 11520.691
# HELP node_disk_read_bytes_total The total number of bytes read successfully.
# TYPE node_disk_read_bytes_total counter
node_disk_read_bytes_total{device="dm-0"} 1.43212059136e+11
node_disk_read_bytes_total{device="dm-1"} 1.62844672e+09
node_disk_read_bytes_total{device="dm-2"} 3.6360531968e+10
node_disk_read_bytes_total{device="nvme0n1"} 1.8126767872e+11
# HELP node_disk_read_time_seconds_total The total number of seconds spent by all reads.
# TYPE node_disk_read_time_seconds_total counter
node_disk_read_time_seconds_total{device="dm-0"} 5492.32
node_disk_read_time_seconds_total{device="dm-1"} 158.862
node_disk_read_time_seconds_total{device="dm-2"} 1996.285
node_disk_read_time_seconds_total{device="nvme0n1"} 8769.603000000001
# HELP node_disk_reads_completed_total The total number of reads completed successfully.
# TYPE node_disk_reads_completed_total counter
node_disk_reads_completed_total{device="dm-0"} 805325
node_disk_reads_completed_total{device="dm-1"} 397113
node_disk_reads_completed_total{device="dm-2"} 286808
node_disk_reads_completed_total{device="nvme0n1"} 1.439913e+06
# HELP node_disk_reads_merged_total The total number of reads merged.
# TYPE node_disk_reads_merged_total counter
node_disk_reads_merged_total{device="dm-0"} 0
node_disk_reads_merged_total{device="dm-1"} 0
node_disk_reads_merged_total{device="dm-2"} 0
node_disk_reads_merged_total{device="nvme0n1"} 202883
# HELP node_disk_write_time_seconds_total This is the total number of seconds spent by all writes.
# TYPE node_disk_write_time_seconds_total counter
node_disk_write_time_seconds_total{device="dm-0"} 731.082
node_disk_write_time_seconds_total{device="dm-1"} 1891.229
node_disk_write_time_seconds_total{device="dm-2"} 2112.467
node_disk_write_time_seconds_total{device="nvme0n1"} 2751.088
# HELP node_disk_writes_completed_total The total number of writes completed successfully.
# TYPE node_disk_writes_completed_total counter
node_disk_writes_completed_total{device="dm-0"} 113957
node_disk_writes_completed_total{device="dm-1"} 942140
node_disk_writes_completed_total{device="dm-2"} 633235
node_disk_writes_completed_total{device="nvme0n1"} 977350
# HELP node_disk_writes_merged_total The number of writes merged.
# TYPE node_disk_writes_merged_total counter
node_disk_writes_merged_total{device="dm-0"} 0
node_disk_writes_merged_total{device="dm-1"} 0
node_disk_writes_merged_total{device="dm-2"} 0
node_disk_writes_merged_total{device="nvme0n1"} 738789
# HELP node_disk_written_bytes_total The total number of bytes written successfully.
# TYPE node_disk_written_bytes_total counter
node_disk_written_bytes_total{device="dm-0"} 1.821488128e+09
node_disk_written_bytes_total{device="dm-1"} 5.95206144e+09
node_disk_written_bytes_total{device="dm-2"} 4.1596535296e+10
node_disk_written_bytes_total{device="nvme0n1"} 4.8958386688e+10
# HELP node_entropy_available_bits Bits of available entropy.
# TYPE node_entropy_available_bits gauge
node_entropy_available_bits 3963
# HELP node_entropy_pool_size_bits Bits of entropy pool.
# TYPE node_entropy_pool_size_bits gauge
node_entropy_pool_size_bits 4096
# HELP node_exporter_build_info A metric with a constant '1' value labeled by version, revision, branch, and goversion from which node_exporter was built.
# TYPE node_exporter_build_info gauge
node_exporter_build_info{branch="",goversion="go1.17",revision="",version=""} 1
# HELP node_filefd_allocated File descriptor statistics: allocated.
# TYPE node_filefd_allocated gauge
node_filefd_allocated 19520
# HELP node_filefd_maximum File descriptor statistics: maximum.
# TYPE node_filefd_maximum gauge
node_filefd_maximum 3.247138e+06
# HELP node_filesystem_avail_bytes Filesystem space available to non-root users in bytes.
# TYPE node_filesystem_avail_bytes gauge
node_filesystem_avail_bytes{device="/dev/fuse",fstype="fuse",mountpoint="/run/user/1000/doc"} 0
node_filesystem_avail_bytes{device="/dev/mapper/cl-home",fstype="xfs",mountpoint="/home"} 2.96452349952e+11
node_filesystem_avail_bytes{device="/dev/mapper/cl-root",fstype="xfs",mountpoint="/"} 3.8826491904e+10
node_filesystem_avail_bytes{device="/dev/nvme0n1p1",fstype="vfat",mountpoint="/boot/efi"} 6.20314624e+08
node_filesystem_avail_bytes{device="/dev/nvme0n1p2",fstype="ext4",mountpoint="/boot"} 5.08178432e+08
node_filesystem_avail_bytes{device="gvfsd-fuse",fstype="fuse.gvfsd-fuse",mountpoint="/run/user/1000/gvfs"} 0
node_filesystem_avail_bytes{device="tmpfs",fstype="tmpfs",mountpoint="/run"} 1.62147328e+10
node_filesystem_avail_bytes{device="tmpfs",fstype="tmpfs",mountpoint="/run/user/1000"} 3.340566528e+09
# HELP node_filesystem_device_error Whether an error occurred while getting statistics for the given device.
# TYPE node_filesystem_device_error gauge
node_filesystem_device_error{device="/dev/fuse",fstype="fuse",mountpoint="/run/user/1000/doc"} 0
node_filesystem_device_error{device="/dev/mapper/cl-home",fstype="xfs",mountpoint="/home"} 0
node_filesystem_device_error{device="/dev/mapper/cl-root",fstype="xfs",mountpoint="/"} 0
node_filesystem_device_error{device="/dev/nvme0n1p1",fstype="vfat",mountpoint="/boot/efi"} 0
node_filesystem_device_error{device="/dev/nvme0n1p2",fstype="ext4",mountpoint="/boot"} 0
node_filesystem_device_error{device="gvfsd-fuse",fstype="fuse.gvfsd-fuse",mountpoint="/run/user/1000/gvfs"} 0
node_filesystem_device_error{device="tmpfs",fstype="tmpfs",mountpoint="/run"} 0
node_filesystem_device_error{device="tmpfs",fstype="tmpfs",mountpoint="/run/user/1000"} 0
# HELP node_filesystem_files Filesystem total file nodes.
# TYPE node_filesystem_files gauge
node_filesystem_files{device="/dev/fuse",fstype="fuse",mountpoint="/run/user/1000/doc"} 0
node_filesystem_files{device="/dev/mapper/cl-home",fstype="xfs",mountpoint="/home"} 2.2089728e+08
node_filesystem_files{device="/dev/mapper/cl-root",fstype="xfs",mountpoint="/"} 2.62144e+07
node_filesystem_files{device="/dev/nvme0n1p1",fstype="vfat",mountpoint="/boot/efi"} 0
node_filesystem_files{device="/dev/nvme0n1p2",fstype="ext4",mountpoint="/boot"} 65536
node_filesystem_files{device="gvfsd-fuse",fstype="fuse.gvfsd-fuse",mountpoint="/run/user/1000/gvfs"} 0
node_filesystem_files{device="tmpfs",fstype="tmpfs",mountpoint="/run"} 4.077947e+06
node_filesystem_files{device="tmpfs",fstype="tmpfs",mountpoint="/run/user/1000"} 4.077947e+06
# HELP node_filesystem_files_free Filesystem total free file nodes.
# TYPE node_filesystem_files_free gauge
node_filesystem_files_free{device="/dev/fuse",fstype="fuse",mountpoint="/run/user/1000/doc"} 0
node_filesystem_files_free{device="/dev/mapper/cl-home",fstype="xfs",mountpoint="/home"} 2.18645287e+08
node_filesystem_files_free{device="/dev/mapper/cl-root",fstype="xfs",mountpoint="/"} 2.5958645e+07
node_filesystem_files_free{device="/dev/nvme0n1p1",fstype="vfat",mountpoint="/boot/efi"} 0
node_filesystem_files_free{device="/dev/nvme0n1p2",fstype="ext4",mountpoint="/boot"} 65491
node_filesystem_files_free{device="gvfsd-fuse",fstype="fuse.gvfsd-fuse",mountpoint="/run/user/1000/gvfs"} 0
node_filesystem_files_free{device="tmpfs",fstype="tmpfs",mountpoint="/run"} 4.07687e+06
node_filesystem_files_free{device="tmpfs",fstype="tmpfs",mountpoint="/run/user/1000"} 4.077864e+06
# HELP node_filesystem_free_bytes Filesystem free space in bytes.
# TYPE node_filesystem_free_bytes gauge
node_filesystem_free_bytes{device="/dev/fuse",fstype="fuse",mountpoint="/run/user/1000/doc"} 0
node_filesystem_free_bytes{device="/dev/mapper/cl-home",fstype="xfs",mountpoint="/home"} 2.96452349952e+11
node_filesystem_free_bytes{device="/dev/mapper/cl-root",fstype="xfs",mountpoint="/"} 3.8826491904e+10
node_filesystem_free_bytes{device="/dev/nvme0n1p1",fstype="vfat",mountpoint="/boot/efi"} 6.20314624e+08
node_filesystem_free_bytes{device="/dev/nvme0n1p2",fstype="ext4",mountpoint="/boot"} 5.7864192e+08
node_filesystem_free_bytes{device="gvfsd-fuse",fstype="fuse.gvfsd-fuse",mountpoint="/run/user/1000/gvfs"} 0
node_filesystem_free_bytes{device="tmpfs",fstype="tmpfs",mountpoint="/run"} 1.62147328e+10
node_filesystem_free_bytes{device="tmpfs",fstype="tmpfs",mountpoint="/run/user/1000"} 3.340566528e+09
# HELP node_filesystem_readonly Filesystem read-only status.
# TYPE node_filesystem_readonly gauge
node_filesystem_readonly{device="/dev/fuse",fstype="fuse",mountpoint="/run/user/1000/doc"} 0
node_filesystem_readonly{device="/dev/mapper/cl-home",fstype="xfs",mountpoint="/home"} 0
node_filesystem_readonly{device="/dev/mapper/cl-root",fstype="xfs",mountpoint="/"} 0
node_filesystem_readonly{device="/dev/nvme0n1p1",fstype="vfat",mountpoint="/boot/efi"} 0
node_filesystem_readonly{device="/dev/nvme0n1p2",fstype="ext4",mountpoint="/boot"} 0
node_filesystem_readonly{device="gvfsd-fuse",fstype="fuse.gvfsd-fuse",mountpoint="/run/user/1000/gvfs"} 0
node_filesystem_readonly{device="tmpfs",fstype="tmpfs",mountpoint="/run"} 0
node_filesystem_readonly{device="tmpfs",fstype="tmpfs",mountpoint="/run/user/1000"} 0
# HELP node_filesystem_size_bytes Filesystem size in bytes.
# TYPE node_filesystem_size_bytes gauge
node_filesystem_size_bytes{device="/dev/fuse",fstype="fuse",mountpoint="/run/user/1000/doc"} 0
node_filesystem_size_bytes{device="/dev/mapper/cl-home",fstype="xfs",mountpoint="/home"} 4.5217673216e+11
node_filesystem_size_bytes{device="/dev/mapper/cl-root",fstype="xfs",mountpoint="/"} 5.36608768e+10
node_filesystem_size_bytes{device="/dev/nvme0n1p1",fstype="vfat",mountpoint="/boot/efi"} 6.27900416e+08
node_filesystem_size_bytes{device="/dev/nvme0n1p2",fstype="ext4",mountpoint="/boot"} 1.02330368e+09
node_filesystem_size_bytes{device="gvfsd-fuse",fstype="fuse.gvfsd-fuse",mountpoint="/run/user/1000/gvfs"} 0
node_filesystem_size_bytes{device="tmpfs",fstype="tmpfs",mountpoint="/run"} 1.6703270912e+10
node_filesystem_size_bytes{device="tmpfs",fstype="tmpfs",mountpoint="/run/user/1000"} 3.340652544e+09
# HELP node_forks_total Total number of forks.
# TYPE node_forks_total counter
node_forks_total 2.140686e+07
# HELP node_hwmon_chip_names Annotation metric for human-readable chip names
# TYPE node_hwmon_chip_names gauge
node_hwmon_chip_names{chip="0000:00:03_1_0000:09:00_0",chip_name="nouveau"} 1
node_hwmon_chip_names{chip="pci0000:00_0000:00:18_3",chip_name="k10temp"} 1
node_hwmon_chip_names{chip="platform_eeepc_wmi",chip_name="asus"} 1
# HELP node_hwmon_fan_rpm Hardware monitor for fan revolutions per minute (input)
# TYPE node_hwmon_fan_rpm gauge
node_hwmon_fan_rpm{chip="0000:00:03_1_0000:09:00_0",sensor="fan1"} 0
node_hwmon_fan_rpm{chip="platform_eeepc_wmi",sensor="fan1"} 0
# HELP node_hwmon_pwm Hardware monitor pwm element
# TYPE node_hwmon_pwm gauge
node_hwmon_pwm{chip="0000:00:03_1_0000:09:00_0",sensor="pwm1"} 100
# HELP node_hwmon_pwm_enable Hardware monitor pwm element enable
# TYPE node_hwmon_pwm_enable gauge
node_hwmon_pwm_enable{chip="0000:00:03_1_0000:09:00_0",sensor="pwm1"} -1
node_hwmon_pwm_enable{chip="platform_eeepc_wmi",sensor="pwm1"} 2
# HELP node_hwmon_pwm_max Hardware monitor pwm element max
# TYPE node_hwmon_pwm_max gauge
node_hwmon_pwm_max{chip="0000:00:03_1_0000:09:00_0",sensor="pwm1"} 100
# HELP node_hwmon_pwm_min Hardware monitor pwm element min
# TYPE node_hwmon_pwm_min gauge
node_hwmon_pwm_min{chip="0000:00:03_1_0000:09:00_0",sensor="pwm1"} 30
# HELP node_hwmon_sensor_label Label for given chip and sensor
# TYPE node_hwmon_sensor_label gauge
node_hwmon_sensor_label{chip="pci0000:00_0000:00:18_3",label="tctl",sensor="temp1"} 1
node_hwmon_sensor_label{chip="pci0000:00_0000:00:18_3",label="tdie",sensor="temp2"} 1
node_hwmon_sensor_label{chip="platform_eeepc_wmi",label="cpu_fan",sensor="fan1"} 1
# HELP node_hwmon_temp_auto_point1_pwm_celsius Hardware monitor for temperature (auto_point1_pwm)
# TYPE node_hwmon_temp_auto_point1_pwm_celsius gauge
node_hwmon_temp_auto_point1_pwm_celsius{chip="0000:00:03_1_0000:09:00_0",sensor="temp1"} 0.1
# HELP node_hwmon_temp_auto_point1_temp_celsius Hardware monitor for temperature (auto_point1_temp)
# TYPE node_hwmon_temp_auto_point1_temp_celsius gauge
node_hwmon_temp_auto_point1_temp_celsius{chip="0000:00:03_1_0000:09:00_0",sensor="temp1"} 90
# HELP node_hwmon_temp_auto_point1_temp_hyst_celsius Hardware monitor for temperature (auto_point1_temp_hyst)
# TYPE node_hwmon_temp_auto_point1_temp_hyst_celsius gauge
node_hwmon_temp_auto_point1_temp_hyst_celsius{chip="0000:00:03_1_0000:09:00_0",sensor="temp1"} 3
# HELP node_hwmon_temp_celsius Hardware monitor for temperature (input)
# TYPE node_hwmon_temp_celsius gauge
node_hwmon_temp_celsius{chip="0000:00:03_1_0000:09:00_0",sensor="temp1"} 38
node_hwmon_temp_celsius{chip="pci0000:00_0000:00:18_3",sensor="temp1"} 54.875
node_hwmon_temp_celsius{chip="pci0000:00_0000:00:18_3",sensor="temp2"} 44.875
# HELP node_hwmon_temp_crit_celsius Hardware monitor for temperature (crit)
# TYPE node_hwmon_temp_crit_celsius gauge
node_hwmon_temp_crit_celsius{chip="0000:00:03_1_0000:09:00_0",sensor="temp1"} 105
# HELP node_hwmon_temp_crit_hyst_celsius Hardware monitor for temperature (crit_hyst)
# TYPE node_hwmon_temp_crit_hyst_celsius gauge
node_hwmon_temp_crit_hyst_celsius{chip="0000:00:03_1_0000:09:00_0",sensor="temp1"} 5
# HELP node_hwmon_temp_emergency_celsius Hardware monitor for temperature (emergency)
# TYPE node_hwmon_temp_emergency_celsius gauge
node_hwmon_temp_emergency_celsius{chip="0000:00:03_1_0000:09:00_0",sensor="temp1"} 135
# HELP node_hwmon_temp_emergency_hyst_celsius Hardware monitor for temperature (emergency_hyst)
# TYPE node_hwmon_temp_emergency_hyst_celsius gauge
node_hwmon_temp_emergency_hyst_celsius{chip="0000:00:03_1_0000:09:00_0",sensor="temp1"} 5
# HELP node_hwmon_temp_max_celsius Hardware monitor for temperature (max)
# TYPE node_hwmon_temp_max_celsius gauge
node_hwmon_temp_max_celsius{chip="0000:00:03_1_0000:09:00_0",sensor="temp1"} 95
# HELP node_hwmon_temp_max_hyst_celsius Hardware monitor for temperature (max_hyst)
# TYPE node_hwmon_temp_max_hyst_celsius gauge
node_hwmon_temp_max_hyst_celsius{chip="0000:00:03_1_0000:09:00_0",sensor="temp1"} 3
# HELP node_hwmon_update_interval_seconds Hardware monitor update interval
# TYPE node_hwmon_update_interval_seconds gauge
node_hwmon_update_interval_seconds{chip="0000:00:03_1_0000:09:00_0",sensor="update_interval0"} 1
# HELP node_intr_total Total number of interrupts serviced.
# TYPE node_intr_total counter
node_intr_total 5.334582138e+09
# HELP node_load1 1m load average.
# TYPE node_load1 gauge
node_load1 8.54
# HELP node_load15 15m load average.
# TYPE node_load15 gauge
node_load15 3.53
# HELP node_load5 5m load average.
# TYPE node_load5 gauge
node_load5 5.06
# HELP node_memory_Active_anon_bytes Memory information field Active_anon_bytes.
# TYPE node_memory_Active_anon_bytes gauge
node_memory_Active_anon_bytes 4.83127296e+09
# HELP node_memory_Active_bytes Memory information field Active_bytes.
# TYPE node_memory_Active_bytes gauge
node_memory_Active_bytes 6.614925312e+09
# HELP node_memory_Active_file_bytes Memory information field Active_file_bytes.
# TYPE node_memory_Active_file_bytes gauge
node_memory_Active_file_bytes 1.783652352e+09
# HELP node_memory_AnonHugePages_bytes Memory information field AnonHugePages_bytes.
# TYPE node_memory_AnonHugePages_bytes gauge
node_memory_AnonHugePages_bytes 7.704936448e+09
# HELP node_memory_AnonPages_bytes Memory information field AnonPages_bytes.
# TYPE node_memory_AnonPages_bytes gauge
node_memory_AnonPages_bytes 1.0612457472e+10
# HELP node_memory_Bounce_bytes Memory information field Bounce_bytes.
# TYPE node_memory_Bounce_bytes gauge
node_memory_Bounce_bytes 0
# HELP node_memory_Buffers_bytes Memory information field Buffers_bytes.
# TYPE node_memory_Buffers_bytes gauge
node_memory_Buffers_bytes 36864
# HELP node_memory_Cached_bytes Memory information field Cached_bytes.
# TYPE node_memory_Cached_bytes gauge
node_memory_Cached_bytes 5.468053504e+09
# HELP node_memory_CommitLimit_bytes Memory information field CommitLimit_bytes.
# TYPE node_memory_CommitLimit_bytes gauge
node_memory_CommitLimit_bytes 2.0994039808e+10
# HELP node_memory_Committed_AS_bytes Memory information field Committed_AS_bytes.
# TYPE node_memory_Committed_AS_bytes gauge
node_memory_Committed_AS_bytes 3.2676655104e+10
# HELP node_memory_DirectMap1G_bytes Memory information field DirectMap1G_bytes.
# TYPE node_memory_DirectMap1G_bytes gauge
node_memory_DirectMap1G_bytes 2.147483648e+09
# HELP node_memory_DirectMap2M_bytes Memory information field DirectMap2M_bytes.
# TYPE node_memory_DirectMap2M_bytes gauge
node_memory_DirectMap2M_bytes 3.1392268288e+10
# HELP node_memory_DirectMap4k_bytes Memory information field DirectMap4k_bytes.
# TYPE node_memory_DirectMap4k_bytes gauge
node_memory_DirectMap4k_bytes 7.20539648e+08
# HELP node_memory_Dirty_bytes Memory information field Dirty_bytes.
# TYPE node_memory_Dirty_bytes gauge
node_memory_Dirty_bytes 2.2142976e+07
# HELP node_memory_FileHugePages_bytes Memory information field FileHugePages_bytes.
# TYPE node_memory_FileHugePages_bytes gauge
node_memory_FileHugePages_bytes 0
# HELP node_memory_FilePmdMapped_bytes Memory information field FilePmdMapped_bytes.
# TYPE node_memory_FilePmdMapped_bytes gauge
node_memory_FilePmdMapped_bytes 0
# HELP node_memory_HardwareCorrupted_bytes Memory information field HardwareCorrupted_bytes.
# TYPE node_memory_HardwareCorrupted_bytes gauge
node_memory_HardwareCorrupted_bytes 0
# HELP node_memory_HugePages_Free Memory information field HugePages_Free.
# TYPE node_memory_HugePages_Free gauge
node_memory_HugePages_Free 0
# HELP node_memory_HugePages_Rsvd Memory information field HugePages_Rsvd.
# TYPE node_memory_HugePages_Rsvd gauge
node_memory_HugePages_Rsvd 0
# HELP node_memory_HugePages_Surp Memory information field HugePages_Surp.
# TYPE node_memory_HugePages_Surp gauge
node_memory_HugePages_Surp 0
# HELP node_memory_HugePages_Total Memory information field HugePages_Total.
# TYPE node_memory_HugePages_Total gauge
node_memory_HugePages_Total 0
# HELP node_memory_Hugepagesize_bytes Memory information field Hugepagesize_bytes.
# TYPE node_memory_Hugepagesize_bytes gauge
node_memory_Hugepagesize_bytes 2.097152e+06
# HELP node_memory_Hugetlb_bytes Memory information field Hugetlb_bytes.
# TYPE node_memory_Hugetlb_bytes gauge
node_memory_Hugetlb_bytes 0
# HELP node_memory_Inactive_anon_bytes Memory information field Inactive_anon_bytes.
# TYPE node_memory_Inactive_anon_bytes gauge
node_memory_Inactive_anon_bytes 6.529441792e+09
# HELP node_memory_Inactive_bytes Memory information field Inactive_bytes.
# TYPE node_memory_Inactive_bytes gauge
node_memory_Inactive_bytes 9.724928e+09
# HELP node_memory_Inactive_file_bytes Memory information field Inactive_file_bytes.
# TYPE node_memory_Inactive_file_bytes gauge
node_memory_Inactive_file_bytes 3.195486208e+09
# HELP node_memory_KReclaimable_bytes Memory information field KReclaimable_bytes.
# TYPE node_memory_KReclaimable_bytes gauge
node_memory_KReclaimable_bytes 1.34076416e+09
# HELP node_memory_KernelStack_bytes Memory information field KernelStack_bytes.
# TYPE node_memory_KernelStack_bytes gauge
node_memory_KernelStack_bytes 3.3931264e+07
# HELP node_memory_Mapped_bytes Memory information field Mapped_bytes.
# TYPE node_memory_Mapped_bytes gauge
node_memory_Mapped_bytes 1.020248064e+09
# HELP node_memory_MemAvailable_bytes Memory information field MemAvailable_bytes.
# TYPE node_memory_MemAvailable_bytes gauge
node_memory_MemAvailable_bytes 1.987110912e+10
# HELP node_memory_MemFree_bytes Memory information field MemFree_bytes.
# TYPE node_memory_MemFree_bytes gauge
node_memory_MemFree_bytes 1.402710016e+10
# HELP node_memory_MemTotal_bytes Memory information field MemTotal_bytes.
# TYPE node_memory_MemTotal_bytes gauge
node_memory_MemTotal_bytes 3.3406541824e+10
# HELP node_memory_Mlocked_bytes Memory information field Mlocked_bytes.
# TYPE node_memory_Mlocked_bytes gauge
node_memory_Mlocked_bytes 49152
# HELP node_memory_NFS_Unstable_bytes Memory information field NFS_Unstable_bytes.
# TYPE node_memory_NFS_Unstable_bytes gauge
node_memory_NFS_Unstable_bytes 0
# HELP node_memory_PageTables_bytes Memory information field PageTables_bytes.
# TYPE node_memory_PageTables_bytes gauge
node_memory_PageTables_bytes 1.5577088e+08
# HELP node_memory_Percpu_bytes Memory information field Percpu_bytes.
# TYPE node_memory_Percpu_bytes gauge
node_memory_Percpu_bytes 4.4957696e+07
# HELP node_memory_SReclaimable_bytes Memory information field SReclaimable_bytes.
# TYPE node_memory_SReclaimable_bytes gauge
node_memory_SReclaimable_bytes 1.34076416e+09
# HELP node_memory_SUnreclaim_bytes Memory information field SUnreclaim_bytes.
# TYPE node_memory_SUnreclaim_bytes gauge
node_memory_SUnreclaim_bytes 5.10304256e+08
# HELP node_memory_ShmemHugePages_bytes Memory information field ShmemHugePages_bytes.
# TYPE node_memory_ShmemHugePages_bytes gauge
node_memory_ShmemHugePages_bytes 0
# HELP node_memory_ShmemPmdMapped_bytes Memory information field ShmemPmdMapped_bytes.
# TYPE node_memory_ShmemPmdMapped_bytes gauge
node_memory_ShmemPmdMapped_bytes 0
# HELP node_memory_Shmem_bytes Memory information field Shmem_bytes.
# TYPE node_memory_Shmem_bytes gauge
node_memory_Shmem_bytes 4.88968192e+08
# HELP node_memory_Slab_bytes Memory information field Slab_bytes.
# TYPE node_memory_Slab_bytes gauge
node_memory_Slab_bytes 1.851068416e+09
# HELP node_memory_SwapCached_bytes Memory information field SwapCached_bytes.
# TYPE node_memory_SwapCached_bytes gauge
node_memory_SwapCached_bytes 2.1106688e+08
# HELP node_memory_SwapFree_bytes Memory information field SwapFree_bytes.
# TYPE node_memory_SwapFree_bytes gauge
node_memory_SwapFree_bytes 1.187201024e+09
# HELP node_memory_SwapTotal_bytes Memory information field SwapTotal_bytes.
# TYPE node_memory_SwapTotal_bytes gauge
node_memory_SwapTotal_bytes 4.290768896e+09
# HELP node_memory_Unevictable_bytes Memory information field Unevictable_bytes.
# TYPE node_memory_Unevictable_bytes gauge
node_memory_Unevictable_bytes 49152
# HELP node_memory_VmallocChunk_bytes Memory information field VmallocChunk_bytes.
# TYPE node_memory_VmallocChunk_bytes gauge
node_memory_VmallocChunk_bytes 0
# HELP node_memory_VmallocTotal_bytes Memory information field VmallocTotal_bytes.
# TYPE node_memory_VmallocTotal_bytes gauge
node_memory_VmallocTotal_bytes 3.5184372087808e+13
# HELP node_memory_VmallocUsed_bytes Memory information field VmallocUsed_bytes.
# TYPE node_memory_VmallocUsed_bytes gauge
node_memory_VmallocUsed_bytes 0
# HELP node_memory_WritebackTmp_bytes Memory information field WritebackTmp_bytes.
# TYPE node_memory_WritebackTmp_bytes gauge
node_memory_WritebackTmp_bytes 0
# HELP node_memory_Writeback_bytes Memory information field Writeback_bytes.
# TYPE node_memory_Writeback_bytes gauge
node_memory_Writeback_bytes 0
# HELP node_netstat_Icmp6_InErrors Statistic Icmp6InErrors.
# TYPE node_netstat_Icmp6_InErrors untyped
node_netstat_Icmp6_InErrors 217
# HELP node_netstat_Icmp6_InMsgs Statistic Icmp6InMsgs.
# TYPE node_netstat_Icmp6_InMsgs untyped
node_netstat_Icmp6_InMsgs 386
# HELP node_netstat_Icmp6_OutMsgs Statistic Icmp6OutMsgs.
# TYPE node_netstat_Icmp6_OutMsgs untyped
node_netstat_Icmp6_OutMsgs 807
# HELP node_netstat_Icmp_InErrors Statistic IcmpInErrors.
# TYPE node_netstat_Icmp_InErrors untyped
node_netstat_Icmp_InErrors 8
# HELP node_netstat_Icmp_InMsgs Statistic IcmpInMsgs.
# TYPE node_netstat_Icmp_InMsgs untyped
node_netstat_Icmp_InMsgs 26036
# HELP node_netstat_Icmp_OutMsgs Statistic IcmpOutMsgs.
# TYPE node_netstat_Icmp_OutMsgs untyped
node_netstat_Icmp_OutMsgs 26051
# HELP node_netstat_Ip6_InOctets Statistic Ip6InOctets.
# TYPE node_netstat_Ip6_InOctets untyped
node_netstat_Ip6_InOctets 742490
# HELP node_netstat_Ip6_OutOctets Statistic Ip6OutOctets.
# TYPE node_netstat_Ip6_OutOctets untyped
node_netstat_Ip6_OutOctets 656202
# HELP node_netstat_IpExt_InOctets Statistic IpExtInOctets.
# TYPE node_netstat_IpExt_InOctets untyped
node_netstat_IpExt_InOctets 5.89563694e+08
# HELP node_netstat_IpExt_OutOctets Statistic IpExtOutOctets.
# TYPE node_netstat_IpExt_OutOctets untyped
node_netstat_IpExt_OutOctets 2.4967182e+07
# HELP node_netstat_Ip_Forwarding Statistic IpForwarding.
# TYPE node_netstat_Ip_Forwarding untyped
node_netstat_Ip_Forwarding 1
# HELP node_netstat_TcpExt_ListenDrops Statistic TcpExtListenDrops.
# TYPE node_netstat_TcpExt_ListenDrops untyped
node_netstat_TcpExt_ListenDrops 0
# HELP node_netstat_TcpExt_ListenOverflows Statistic TcpExtListenOverflows.
# TYPE node_netstat_TcpExt_ListenOverflows untyped
node_netstat_TcpExt_ListenOverflows 0
# HELP node_netstat_TcpExt_SyncookiesFailed Statistic TcpExtSyncookiesFailed.
# TYPE node_netstat_TcpExt_SyncookiesFailed untyped
node_netstat_TcpExt_SyncookiesFailed 0
# HELP node_netstat_TcpExt_SyncookiesRecv Statistic TcpExtSyncookiesRecv.
# TYPE node_netstat_TcpExt_SyncookiesRecv untyped
node_netstat_TcpExt_SyncookiesRecv 0
# HELP node_netstat_TcpExt_SyncookiesSent Statistic TcpExtSyncookiesSent.
# TYPE node_netstat_TcpExt_SyncookiesSent untyped
node_netstat_TcpExt_SyncookiesSent 0
# HELP node_netstat_TcpExt_TCPSynRetrans Statistic TcpExtTCPSynRetrans.
# TYPE node_netstat_TcpExt_TCPSynRetrans untyped
node_netstat_TcpExt_TCPSynRetrans 6059
# HELP node_netstat_Tcp_ActiveOpens Statistic TcpActiveOpens.
# TYPE node_netstat_Tcp_ActiveOpens untyped
node_netstat_Tcp_ActiveOpens 10229
# HELP node_netstat_Tcp_CurrEstab Statistic TcpCurrEstab.
# TYPE node_netstat_Tcp_CurrEstab untyped
node_netstat_Tcp_CurrEstab 2
# HELP node_netstat_Tcp_InErrs Statistic TcpInErrs.
# TYPE node_netstat_Tcp_InErrs untyped
node_netstat_Tcp_InErrs 92
# HELP node_netstat_Tcp_InSegs Statistic TcpInSegs.
# TYPE node_netstat_Tcp_InSegs untyped
node_netstat_Tcp_InSegs 284742
# HELP node_netstat_Tcp_OutRsts Statistic TcpOutRsts.
# TYPE node_netstat_Tcp_OutRsts untyped
node_netstat_Tcp_OutRsts 7576
# HELP node_netstat_Tcp_OutSegs Statistic TcpOutSegs.
# TYPE node_netstat_Tcp_OutSegs untyped
node_netstat_Tcp_OutSegs 212547
# HELP node_netstat_Tcp_PassiveOpens Statistic TcpPassiveOpens.
# TYPE node_netstat_Tcp_PassiveOpens untyped
node_netstat_Tcp_PassiveOpens 50
# HELP node_netstat_Tcp_RetransSegs Statistic TcpRetransSegs.
# TYPE node_netstat_Tcp_RetransSegs untyped
node_netstat_Tcp_RetransSegs 7783
# HELP node_netstat_Udp6_InDatagrams Statistic Udp6InDatagrams.
# TYPE node_netstat_Udp6_InDatagrams untyped
node_netstat_Udp6_InDatagrams 6
# HELP node_netstat_Udp6_InErrors Statistic Udp6InErrors.
# TYPE node_netstat_Udp6_InErrors untyped
node_netstat_Udp6_InErrors 0
# HELP node_netstat_Udp6_NoPorts Statistic Udp6NoPorts.
# TYPE node_netstat_Udp6_NoPorts untyped
node_netstat_Udp6_NoPorts 0
# HELP node_netstat_Udp6_OutDatagrams Statistic Udp6OutDatagrams.
# TYPE node_netstat_Udp6_OutDatagrams untyped
node_netstat_Udp6_OutDatagrams 112
# HELP node_netstat_Udp6_RcvbufErrors Statistic Udp6RcvbufErrors.
# TYPE node_netstat_Udp6_RcvbufErrors untyped
node_netstat_Udp6_RcvbufErrors 0
# HELP node_netstat_Udp6_SndbufErrors Statistic Udp6SndbufErrors.
# TYPE node_netstat_Udp6_SndbufErrors untyped
node_netstat_Udp6_SndbufErrors 0
# HELP node_netstat_UdpLite6_InErrors Statistic UdpLite6InErrors.
# TYPE node_netstat_UdpLite6_InErrors untyped
node_netstat_UdpLite6_InErrors 0
# HELP node_netstat_UdpLite_InErrors Statistic UdpLiteInErrors.
# TYPE node_netstat_UdpLite_InErrors untyped
node_netstat_UdpLite_InErrors 0
# HELP node_netstat_Udp_InDatagrams Statistic UdpInDatagrams.
# TYPE node_netstat_Udp_InDatagrams untyped
node_netstat_Udp_InDatagrams 4837
# HELP node_netstat_Udp_InErrors Statistic UdpInErrors.
# TYPE node_netstat_Udp_InErrors untyped
node_netstat_Udp_InErrors 0
# HELP node_netstat_Udp_NoPorts Statistic UdpNoPorts.
# TYPE node_netstat_Udp_NoPorts untyped
node_netstat_Udp_NoPorts 25928
# HELP node_netstat_Udp_OutDatagrams Statistic UdpOutDatagrams.
# TYPE node_netstat_Udp_OutDatagrams untyped
node_netstat_Udp_OutDatagrams 40262
# HELP node_netstat_Udp_RcvbufErrors Statistic UdpRcvbufErrors.
# TYPE node_netstat_Udp_RcvbufErrors untyped
node_netstat_Udp_RcvbufErrors 0
# HELP node_netstat_Udp_SndbufErrors Statistic UdpSndbufErrors.
# TYPE node_netstat_Udp_SndbufErrors untyped
node_netstat_Udp_SndbufErrors 0
# HELP node_network_address_assign_type address_assign_type value of /sys/class/net/<iface>.
# TYPE node_network_address_assign_type gauge
node_network_address_assign_type{device="docker0"} 3
node_network_address_assign_type{device="enp4s0"} 0
node_network_address_assign_type{device="lo"} 0
node_network_address_assign_type{device="veth7b69587"} 1
node_network_address_assign_type{device="virbr0"} 3
node_network_address_assign_type{device="virbr0-nic"} 3
node_network_address_assign_type{device="virbr1"} 3
node_network_address_assign_type{device="virbr1-nic"} 3
node_network_address_assign_type{device="wlp5s0"} 0
# HELP node_network_carrier carrier value of /sys/class/net/<iface>.
# TYPE node_network_carrier gauge
node_network_carrier{device="docker0"} 1
node_network_carrier{device="enp4s0"} 0
node_network_carrier{device="lo"} 1
node_network_carrier{device="veth7b69587"} 1
node_network_carrier{device="virbr0"} 0
node_network_carrier{device="virbr1"} 0
node_network_carrier{device="wlp5s0"} 0
# HELP node_network_carrier_changes_total carrier_changes_total value of /sys/class/net/<iface>.
# TYPE node_network_carrier_changes_total counter
node_network_carrier_changes_total{device="docker0"} 62
node_network_carrier_changes_total{device="enp4s0"} 1
node_network_carrier_changes_total{device="lo"} 0
node_network_carrier_changes_total{device="veth7b69587"} 2
node_network_carrier_changes_total{device="virbr0"} 1
node_network_carrier_changes_total{device="virbr0-nic"} 1
node_network_carrier_changes_total{device="virbr1"} 1
node_network_carrier_changes_total{device="virbr1-nic"} 1
node_network_carrier_changes_total{device="wlp5s0"} 143
# HELP node_network_carrier_down_changes_total carrier_down_changes_total value of /sys/class/net/<iface>.
# TYPE node_network_carrier_down_changes_total counter
node_network_carrier_down_changes_total{device="docker0"} 31
node_network_carrier_down_changes_total{device="enp4s0"} 1
node_network_carrier_down_changes_total{device="lo"} 0
node_network_carrier_down_changes_total{device="veth7b69587"} 1
node_network_carrier_down_changes_total{device="virbr0"} 1
node_network_carrier_down_changes_total{device="virbr0-nic"} 1
node_network_carrier_down_changes_total{device="virbr1"} 1
node_network_carrier_down_changes_total{device="virbr1-nic"} 1
node_network_carrier_down_changes_total{device="wlp5s0"} 72
# HELP node_network_carrier_up_changes_total carrier_up_changes_total value of /sys/class/net/<iface>.
# TYPE node_network_carrier_up_changes_total counter
node_network_carrier_up_changes_total{device="docker0"} 31
node_network_carrier_up_changes_total{device="enp4s0"} 0
node_network_carrier_up_changes_total{device="lo"} 0
node_network_carrier_up_changes_total{device="veth7b69587"} 1
node_network_carrier_up_changes_total{device="virbr0"} 0
node_network_carrier_up_changes_total{device="virbr0-nic"} 0
node_network_carrier_up_changes_total{device="virbr1"} 0
node_network_carrier_up_changes_total{device="virbr1-nic"} 0
node_network_carrier_up_changes_total{device="wlp5s0"} 71
# HELP node_network_device_id device_id value of /sys/class/net/<iface>.
# TYPE node_network_device_id gauge
node_network_device_id{device="docker0"} 0
node_network_device_id{device="enp4s0"} 0
node_network_device_id{device="lo"} 0
node_network_device_id{device="veth7b69587"} 0
node_network_device_id{device="virbr0"} 0
node_network_device_id{device="virbr0-nic"} 0
node_network_device_id{device="virbr1"} 0
node_network_device_id{device="virbr1-nic"} 0
node_network_device_id{device="wlp5s0"} 0
# HELP node_network_dormant dormant value of /sys/class/net/<iface>.
# TYPE node_network_dormant gauge
node_network_dormant{device="docker0"} 0
node_network_dormant{device="enp4s0"} 0
node_network_dormant{device="lo"} 0
node_network_dormant{device="veth7b69587"} 0
node_network_dormant{device="virbr0"} 0
node_network_dormant{device="virbr1"} 0
node_network_dormant{device="wlp5s0"} 0
# HELP node_network_flags flags value of /sys/class/net/<iface>.
# TYPE node_network_flags gauge
node_network_flags{device="docker0"} 4099
node_network_flags{device="enp4s0"} 4099
node_network_flags{device="lo"} 9
node_network_flags{device="veth7b69587"} 4867
node_network_flags{device="virbr0"} 4099
node_network_flags{device="virbr0-nic"} 4866
node_network_flags{device="virbr1"} 4099
node_network_flags{device="virbr1-nic"} 4866
node_network_flags{device="wlp5s0"} 4099
# HELP node_network_iface_id iface_id value of /sys/class/net/<iface>.
# TYPE node_network_iface_id gauge
node_network_iface_id{device="docker0"} 8
node_network_iface_id{device="enp4s0"} 2
node_network_iface_id{device="lo"} 1
node_network_iface_id{device="veth7b69587"} 42
node_network_iface_id{device="virbr0"} 6
node_network_iface_id{device="virbr0-nic"} 7
node_network_iface_id{device="virbr1"} 4
node_network_iface_id{device="virbr1-nic"} 5
node_network_iface_id{device="wlp5s0"} 3
# HELP node_network_iface_link iface_link value of /sys/class/net/<iface>.
# TYPE node_network_iface_link gauge
node_network_iface_link{device="docker0"} 8
node_network_iface_link{device="enp4s0"} 2
node_network_iface_link{device="lo"} 1
node_network_iface_link{device="veth7b69587"} 41
node_network_iface_link{device="virbr0"} 6
node_network_iface_link{device="virbr0-nic"} 7
node_network_iface_link{device="virbr1"} 4
node_network_iface_link{device="virbr1-nic"} 5
node_network_iface_link{device="wlp5s0"} 3
# HELP node_network_iface_link_mode iface_link_mode value of /sys/class/net/<iface>.
# TYPE node_network_iface_link_mode gauge
node_network_iface_link_mode{device="docker0"} 0
node_network_iface_link_mode{device="enp4s0"} 0
node_network_iface_link_mode{device="lo"} 0
node_network_iface_link_mode{device="veth7b69587"} 0
node_network_iface_link_mode{device="virbr0"} 0
node_network_iface_link_mode{device="virbr0-nic"} 0
node_network_iface_link_mode{device="virbr1"} 0
node_network_iface_link_mode{device="virbr1-nic"} 0
node_network_iface_link_mode{device="wlp5s0"} 1
# HELP node_network_info Non-numeric data from /sys/class/net/<iface>, value is always 1.
# TYPE node_network_info gauge
node_network_info{address="00:00:00:00:00:00",broadcast="00:00:00:00:00:00",device="lo",duplex="",ifalias="",operstate="unknown"} 1
node_network_info{address="02:42:5c:e6:8e:6b",broadcast="ff:ff:ff:ff:ff:ff",device="docker0",duplex="",ifalias="",operstate="up"} 1
node_network_info{address="4c:ed:fb:77:77:64",broadcast="ff:ff:ff:ff:ff:ff",device="enp4s0",duplex="unknown",ifalias="",operstate="down"} 1
node_network_info{address="52:54:00:87:33:89",broadcast="ff:ff:ff:ff:ff:ff",device="virbr1",duplex="",ifalias="",operstate="down"} 1
node_network_info{address="52:54:00:87:33:89",broadcast="ff:ff:ff:ff:ff:ff",device="virbr1-nic",duplex="",ifalias="",operstate="down"} 1
node_network_info{address="52:54:00:8b:df:53",broadcast="ff:ff:ff:ff:ff:ff",device="virbr0",duplex="",ifalias="",operstate="down"} 1
node_network_info{address="52:54:00:8b:df:53",broadcast="ff:ff:ff:ff:ff:ff",device="virbr0-nic",duplex="",ifalias="",operstate="down"} 1
node_network_info{address="80:c5:f2:64:0b:b1",broadcast="ff:ff:ff:ff:ff:ff",device="wlp5s0",duplex="",ifalias="",operstate="down"} 1
node_network_info{address="f6:8a:57:51:79:11",broadcast="ff:ff:ff:ff:ff:ff",device="veth7b69587",duplex="full",ifalias="",operstate="up"} 1
# HELP node_network_mtu_bytes mtu_bytes value of /sys/class/net/<iface>.
# TYPE node_network_mtu_bytes gauge
node_network_mtu_bytes{device="docker0"} 1500
node_network_mtu_bytes{device="enp4s0"} 1500
node_network_mtu_bytes{device="lo"} 65536
node_network_mtu_bytes{device="veth7b69587"} 1500
node_network_mtu_bytes{device="virbr0"} 1500
node_network_mtu_bytes{device="virbr0-nic"} 1500
node_network_mtu_bytes{device="virbr1"} 1500
node_network_mtu_bytes{device="virbr1-nic"} 1500
node_network_mtu_bytes{device="wlp5s0"} 1500
# HELP node_network_name_assign_type name_assign_type value of /sys/class/net/<iface>.
# TYPE node_network_name_assign_type gauge
node_network_name_assign_type{device="docker0"} 3
node_network_name_assign_type{device="enp4s0"} 4
node_network_name_assign_type{device="veth7b69587"} 3
node_network_name_assign_type{device="virbr0"} 3
node_network_name_assign_type{device="virbr1"} 3
node_network_name_assign_type{device="wlp5s0"} 4
# HELP node_network_net_dev_group net_dev_group value of /sys/class/net/<iface>.
# TYPE node_network_net_dev_group gauge
node_network_net_dev_group{device="docker0"} 0
node_network_net_dev_group{device="enp4s0"} 0
node_network_net_dev_group{device="lo"} 0
node_network_net_dev_group{device="veth7b69587"} 0
node_network_net_dev_group{device="virbr0"} 0
node_network_net_dev_group{device="virbr0-nic"} 0
node_network_net_dev_group{device="virbr1"} 0
node_network_net_dev_group{device="virbr1-nic"} 0
node_network_net_dev_group{device="wlp5s0"} 0
# HELP node_network_protocol_type protocol_type value of /sys/class/net/<iface>.
# TYPE node_network_protocol_type gauge
node_network_protocol_type{device="docker0"} 1
node_network_protocol_type{device="enp4s0"} 1
node_network_protocol_type{device="lo"} 772
node_network_protocol_type{device="veth7b69587"} 1
node_network_protocol_type{device="virbr0"} 1
node_network_protocol_type{device="virbr0-nic"} 1
node_network_protocol_type{device="virbr1"} 1
node_network_protocol_type{device="virbr1-nic"} 1
node_network_protocol_type{device="wlp5s0"} 1
# HELP node_network_receive_bytes_total Network device statistic receive_bytes.
# TYPE node_network_receive_bytes_total counter
node_network_receive_bytes_total{device="docker0"} 128119
node_network_receive_bytes_total{device="enp4s0"} 0
node_network_receive_bytes_total{device="lo"} 5.121228e+06
node_network_receive_bytes_total{device="veth7b69587"} 121516
node_network_receive_bytes_total{device="virbr0"} 0
node_network_receive_bytes_total{device="virbr0-nic"} 0
node_network_receive_bytes_total{device="virbr1"} 0
node_network_receive_bytes_total{device="virbr1-nic"} 0
node_network_receive_bytes_total{device="wlp5s0"} 5.99587819e+08
# HELP node_network_receive_compressed_total Network device statistic receive_compressed.
# TYPE node_network_receive_compressed_total counter
node_network_receive_compressed_total{device="docker0"} 0
node_network_receive_compressed_total{device="enp4s0"} 0
node_network_receive_compressed_total{device="lo"} 0
node_network_receive_compressed_total{device="veth7b69587"} 0
node_network_receive_compressed_total{device="virbr0"} 0
node_network_receive_compressed_total{device="virbr0-nic"} 0
node_network_receive_compressed_total{device="virbr1"} 0
node_network_receive_compressed_total{device="virbr1-nic"} 0
node_network_receive_compressed_total{device="wlp5s0"} 0
# HELP node_network_receive_drop_total Network device statistic receive_drop.
# TYPE node_network_receive_drop_total counter
node_network_receive_drop_total{device="docker0"} 0
node_network_receive_drop_total{device="enp4s0"} 0
node_network_receive_drop_total{device="lo"} 0
node_network_receive_drop_total{device="veth7b69587"} 0
node_network_receive_drop_total{device="virbr0"} 0
node_network_receive_drop_total{device="virbr0-nic"} 0
node_network_receive_drop_total{device="virbr1"} 0
node_network_receive_drop_total{device="virbr1-nic"} 0
node_network_receive_drop_total{device="wlp5s0"} 0
# HELP node_network_receive_errs_total Network device statistic receive_errs.
# TYPE node_network_receive_errs_total counter
node_network_receive_errs_total{device="docker0"} 0
node_network_receive_errs_total{device="enp4s0"} 0
node_network_receive_errs_total{device="lo"} 0
node_network_receive_errs_total{device="veth7b69587"} 0
node_network_receive_errs_total{device="virbr0"} 0
node_network_receive_errs_total{device="virbr0-nic"} 0
node_network_receive_errs_total{device="virbr1"} 0
node_network_receive_errs_total{device="virbr1-nic"} 0
node_network_receive_errs_total{device="wlp5s0"} 0
# HELP node_network_receive_fifo_total Network device statistic receive_fifo.
# TYPE node_network_receive_fifo_total counter
node_network_receive_fifo_total{device="docker0"} 0
node_network_receive_fifo_total{device="enp4s0"} 0
node_network_receive_fifo_total{device="lo"} 0
node_network_receive_fifo_total{device="veth7b69587"} 0
node_network_receive_fifo_total{device="virbr0"} 0
node_network_receive_fifo_total{device="virbr0-nic"} 0
node_network_receive_fifo_total{device="virbr1"} 0
node_network_receive_fifo_total{device="virbr1-nic"} 0
node_network_receive_fifo_total{device="wlp5s0"} 0
# HELP node_network_receive_frame_total Network device statistic receive_frame.
# TYPE node_network_receive_frame_total counter
node_network_receive_frame_total{device="docker0"} 0
node_network_receive_frame_total{device="enp4s0"} 0
node_network_receive_frame_total{device="lo"} 0
node_network_receive_frame_total{device="veth7b69587"} 0
node_network_receive_frame_total{device="virbr0"} 0
node_network_receive_frame_total{device="virbr0-nic"} 0
node_network_receive_frame_total{device="virbr1"} 0
node_network_receive_frame_total{device="virbr1-nic"} 0
node_network_receive_frame_total{device="wlp5s0"} 0
# HELP node_network_receive_multicast_total Network device statistic receive_multicast.
# TYPE node_network_receive_multicast_total counter
node_network_receive_multicast_total{device="docker0"} 0
node_network_receive_multicast_total{device="enp4s0"} 0
node_network_receive_multicast_total{device="lo"} 0
node_network_receive_multicast_total{device="veth7b69587"} 0
node_network_receive_multicast_total{device="virbr0"} 0
node_network_receive_multicast_total{device="virbr0-nic"} 0
node_network_receive_multicast_total{device="virbr1"} 0
node_network_receive_multicast_total{device="virbr1-nic"} 0
node_network_receive_multicast_total{device="wlp5s0"} 0
# HELP node_network_receive_packets_total Network device statistic receive_packets.
# TYPE node_network_receive_packets_total counter
node_network_receive_packets_total{device="docker0"} 145
node_network_receive_packets_total{device="enp4s0"} 0
node_network_receive_packets_total{device="lo"} 65125
node_network_receive_packets_total{device="veth7b69587"} 62
node_network_receive_packets_total{device="virbr0"} 0
node_network_receive_packets_total{device="virbr0-nic"} 0
node_network_receive_packets_total{device="virbr1"} 0
node_network_receive_packets_total{device="virbr1-nic"} 0
node_network_receive_packets_total{device="wlp5s0"} 468377
# HELP node_network_speed_bytes speed_bytes value of /sys/class/net/<iface>.
# TYPE node_network_speed_bytes gauge
node_network_speed_bytes{device="enp4s0"} -125000
node_network_speed_bytes{device="veth7b69587"} 1.25e+09
# HELP node_network_transmit_bytes_total Network device statistic transmit_bytes.
# TYPE node_network_transmit_bytes_total counter
node_network_transmit_bytes_total{device="docker0"} 1.795261e+06
node_network_transmit_bytes_total{device="enp4s0"} 0
node_network_transmit_bytes_total{device="lo"} 5.121228e+06
node_network_transmit_bytes_total{device="veth7b69587"} 1.757594e+06
node_network_transmit_bytes_total{device="virbr0"} 0
node_network_transmit_bytes_total{device="virbr0-nic"} 0
node_network_transmit_bytes_total{device="virbr1"} 0
node_network_transmit_bytes_total{device="virbr1-nic"} 0
node_network_transmit_bytes_total{device="wlp5s0"} 2.792947e+07
# HELP node_network_transmit_carrier_total Network device statistic transmit_carrier.
# TYPE node_network_transmit_carrier_total counter
node_network_transmit_carrier_total{device="docker0"} 0
node_network_transmit_carrier_total{device="enp4s0"} 0
node_network_transmit_carrier_total{device="lo"} 0
node_network_transmit_carrier_total{device="veth7b69587"} 0
node_network_transmit_carrier_total{device="virbr0"} 0
node_network_transmit_carrier_total{device="virbr0-nic"} 0
node_network_transmit_carrier_total{device="virbr1"} 0
node_network_transmit_carrier_total{device="virbr1-nic"} 0
node_network_transmit_carrier_total{device="wlp5s0"} 0
# HELP node_network_transmit_colls_total Network device statistic transmit_colls.
# TYPE node_network_transmit_colls_total counter
node_network_transmit_colls_total{device="docker0"} 0
node_network_transmit_colls_total{device="enp4s0"} 0
node_network_transmit_colls_total{device="lo"} 0
node_network_transmit_colls_total{device="veth7b69587"} 0
node_network_transmit_colls_total{device="virbr0"} 0
node_network_transmit_colls_total{device="virbr0-nic"} 0
node_network_transmit_colls_total{device="virbr1"} 0
node_network_transmit_colls_total{device="virbr1-nic"} 0
node_network_transmit_colls_total{device="wlp5s0"} 0
# HELP node_network_transmit_compressed_total Network device statistic transmit_compressed.
# TYPE node_network_transmit_compressed_total counter
node_network_transmit_compressed_total{device="docker0"} 0
node_network_transmit_compressed_total{device="enp4s0"} 0
node_network_transmit_compressed_total{device="lo"} 0
node_network_transmit_compressed_total{device="veth7b69587"} 0
node_network_transmit_compressed_total{device="virbr0"} 0
node_network_transmit_compressed_total{device="virbr0-nic"} 0
node_network_transmit_compressed_total{device="virbr1"} 0
node_network_transmit_compressed_total{device="virbr1-nic"} 0
node_network_transmit_compressed_total{device="wlp5s0"} 0
# HELP node_network_transmit_drop_total Network device statistic transmit_drop.
# TYPE node_network_transmit_drop_total counter
node_network_transmit_drop_total{device="docker0"} 0
node_network_transmit_drop_total{device="enp4s0"} 0
node_network_transmit_drop_total{device="lo"} 0
node_network_transmit_drop_total{device="veth7b69587"} 0
node_network_transmit_drop_total{device="virbr0"} 0
node_network_transmit_drop_total{device="virbr0-nic"} 0
node_network_transmit_drop_total{device="virbr1"} 0
node_network_transmit_drop_total{device="virbr1-nic"} 0
node_network_transmit_drop_total{device="wlp5s0"} 0
# HELP node_network_transmit_errs_total Network device statistic transmit_errs.
# TYPE node_network_transmit_errs_total counter
node_network_transmit_errs_total{device="docker0"} 0
node_network_transmit_errs_total{device="enp4s0"} 0
node_network_transmit_errs_total{device="lo"} 0
node_network_transmit_errs_total{device="veth7b69587"} 0
node_network_transmit_errs_total{device="virbr0"} 0
node_network_transmit_errs_total{device="virbr0-nic"} 0
node_network_transmit_errs_total{device="virbr1"} 0
node_network_transmit_errs_total{device="virbr1-nic"} 0
node_network_transmit_errs_total{device="wlp5s0"} 0
# HELP node_network_transmit_fifo_total Network device statistic transmit_fifo.
# TYPE node_network_transmit_fifo_total counter
node_network_transmit_fifo_total{device="docker0"} 0
node_network_transmit_fifo_total{device="enp4s0"} 0
node_network_transmit_fifo_total{device="lo"} 0
node_network_transmit_fifo_total{device="veth7b69587"} 0
node_network_transmit_fifo_total{device="virbr0"} 0
node_network_transmit_fifo_total{device="virbr0-nic"} 0
node_network_transmit_fifo_total{device="virbr1"} 0
node_network_transmit_fifo_total{device="virbr1-nic"} 0
node_network_transmit_fifo_total{device="wlp5s0"} 0
# HELP node_network_transmit_packets_total Network device statistic transmit_packets.
# TYPE node_network_transmit_packets_total counter
node_network_transmit_packets_total{device="docker0"} 8642
node_network_transmit_packets_total{device="enp4s0"} 0
node_network_transmit_packets_total{device="lo"} 65125
node_network_transmit_packets_total{device="veth7b69587"} 8424
node_network_transmit_packets_total{device="virbr0"} 0
node_network_transmit_packets_total{device="virbr0-nic"} 0
node_network_transmit_packets_total{device="virbr1"} 0
node_network_transmit_packets_total{device="virbr1-nic"} 0
node_network_transmit_packets_total{device="wlp5s0"} 212429
# HELP node_network_transmit_queue_length transmit_queue_length value of /sys/class/net/<iface>.
# TYPE node_network_transmit_queue_length gauge
node_network_transmit_queue_length{device="docker0"} 0
node_network_transmit_queue_length{device="enp4s0"} 1000
node_network_transmit_queue_length{device="lo"} 1000
node_network_transmit_queue_length{device="veth7b69587"} 0
node_network_transmit_queue_length{device="virbr0"} 1000
node_network_transmit_queue_length{device="virbr0-nic"} 1000
node_network_transmit_queue_length{device="virbr1"} 1000
node_network_transmit_queue_length{device="virbr1-nic"} 1000
node_network_transmit_queue_length{device="wlp5s0"} 1000
# HELP node_network_up Value is 1 if operstate is 'up', 0 otherwise.
# TYPE node_network_up gauge
node_network_up{device="docker0"} 1
node_network_up{device="enp4s0"} 0
node_network_up{device="lo"} 0
node_network_up{device="veth7b69587"} 1
node_network_up{device="virbr0"} 0
node_network_up{device="virbr0-nic"} 0
node_network_up{device="virbr1"} 0
node_network_up{device="virbr1-nic"} 0
node_network_up{device="wlp5s0"} 0
# HELP node_nf_conntrack_entries Number of currently allocated flow entries for connection tracking.
# TYPE node_nf_conntrack_entries gauge
node_nf_conntrack_entries 84
# HELP node_nf_conntrack_entries_limit Maximum size of connection tracking table.
# TYPE node_nf_conntrack_entries_limit gauge
node_nf_conntrack_entries_limit 262144
# HELP node_nf_conntrack_stat_drop Number of packets dropped due to conntrack failure.
# TYPE node_nf_conntrack_stat_drop gauge
node_nf_conntrack_stat_drop 0
# HELP node_nf_conntrack_stat_early_drop Number of dropped conntrack entries to make room for new ones, if maximum table size was reached.
# TYPE node_nf_conntrack_stat_early_drop gauge
node_nf_conntrack_stat_early_drop 0
# HELP node_nf_conntrack_stat_found Number of searched entries which were successful.
# TYPE node_nf_conntrack_stat_found gauge
node_nf_conntrack_stat_found 0
# HELP node_nf_conntrack_stat_ignore Number of packets seen which are already connected to a conntrack entry.
# TYPE node_nf_conntrack_stat_ignore gauge
node_nf_conntrack_stat_ignore 0
# HELP node_nf_conntrack_stat_insert Number of entries inserted into the list.
# TYPE node_nf_conntrack_stat_insert gauge
node_nf_conntrack_stat_insert 0
# HELP node_nf_conntrack_stat_insert_failed Number of entries for which list insertion was attempted but failed.
# TYPE node_nf_conntrack_stat_insert_failed gauge
node_nf_conntrack_stat_insert_failed 0
# HELP node_nf_conntrack_stat_invalid Number of packets seen which can not be tracked.
# TYPE node_nf_conntrack_stat_invalid gauge
node_nf_conntrack_stat_invalid 42
# HELP node_nf_conntrack_stat_search_restart Number of conntrack table lookups which had to be restarted due to hashtable resizes.
# TYPE node_nf_conntrack_stat_search_restart gauge
node_nf_conntrack_stat_search_restart 20
# HELP node_nvme_info Non-numeric data from /sys/class/nvme/<device>, value is always 1.
# TYPE node_nvme_info gauge
node_nvme_info{device="nvme0",firmware_revision="1B2QEXP7",model="Samsung SSD 970 PRO 512GB",serial="S463NF0M126227F",state="live"} 1
# HELP node_os_info A metric with a constant '1' value labeled by build_id, id, id_like, image_id, image_version, name, pretty_name, variant, variant_id, version, version_codename, version_id.
# TYPE node_os_info gauge
node_os_info{build_id="",id="centos",id_like="rhel fedora",image_id="",image_version="",name="CentOS Stream",pretty_name="CentOS Stream 8",variant="",variant_id="",version="8",version_codename="",version_id="8"} 1
# HELP node_os_version Metric containing the major.minor part of the OS version.
# TYPE node_os_version gauge
node_os_version{id="centos",id_like="rhel fedora",name="CentOS Stream"} 8
# HELP node_power_supply_info info of /sys/class/power_supply/<power_supply>.
# TYPE node_power_supply_info gauge
node_power_supply_info{capacity_level="Full",manufacturer="Logitech",model_name="Wireless Mouse MX Master 3",power_supply="hidpp_battery_0",scope="Device",serial_number="4082-0b-a3-3c-3d",status="Discharging",type="Battery"} 1
# HELP node_power_supply_online online value of /sys/class/power_supply/<power_supply>.
# TYPE node_power_supply_online gauge
node_power_supply_online{power_supply="hidpp_battery_0"} 1
# HELP node_procs_blocked Number of processes blocked waiting for I/O to complete.
# TYPE node_procs_blocked gauge
node_procs_blocked 0
# HELP node_procs_running Number of processes in runnable state.
# TYPE node_procs_running gauge
node_procs_running 17
# HELP node_rapl_core_joules_total Current RAPL core value in joules
# TYPE node_rapl_core_joules_total counter
node_rapl_core_joules_total{index="0"} 18055.835581
# HELP node_rapl_package_joules_total Current RAPL package value in joules
# TYPE node_rapl_package_joules_total counter
node_rapl_package_joules_total{index="0"} 38354.045387
# HELP node_schedstat_running_seconds_total Number of seconds CPU spent running a process.
# TYPE node_schedstat_running_seconds_total counter
node_schedstat_running_seconds_total{cpu="0"} 16089.943239656
node_schedstat_running_seconds_total{cpu="1"} 16634.132721461
node_schedstat_running_seconds_total{cpu="10"} 16645.924410221
node_schedstat_running_seconds_total{cpu="11"} 16337.271146471
node_schedstat_running_seconds_total{cpu="12"} 14530.495091473
node_schedstat_running_seconds_total{cpu="13"} 13870.549601242
node_schedstat_running_seconds_total{cpu="14"} 15400.629199933
node_schedstat_running_seconds_total{cpu="15"} 15695.310698722
node_schedstat_running_seconds_total{cpu="2"} 15627.934247068
node_schedstat_running_seconds_total{cpu="3"} 16186.707067924
node_schedstat_running_seconds_total{cpu="4"} 16444.835429933
node_schedstat_running_seconds_total{cpu="5"} 16953.051074156
node_schedstat_running_seconds_total{cpu="6"} 15981.563157782
node_schedstat_running_seconds_total{cpu="7"} 15547.739385785
node_schedstat_running_seconds_total{cpu="8"} 16835.137652115
node_schedstat_running_seconds_total{cpu="9"} 16444.994696967
# HELP node_schedstat_timeslices_total Number of timeslices executed by CPU.
# TYPE node_schedstat_timeslices_total counter
node_schedstat_timeslices_total{cpu="0"} 2.17455604e+08
node_schedstat_timeslices_total{cpu="1"} 3.0867184e+08
node_schedstat_timeslices_total{cpu="10"} 2.54515634e+08
node_schedstat_timeslices_total{cpu="11"} 2.65435914e+08
node_schedstat_timeslices_total{cpu="12"} 1.96223022e+08
node_schedstat_timeslices_total{cpu="13"} 2.48111692e+08
node_schedstat_timeslices_total{cpu="14"} 2.45909358e+08
node_schedstat_timeslices_total{cpu="15"} 2.31805973e+08
node_schedstat_timeslices_total{cpu="2"} 2.27593082e+08
node_schedstat_timeslices_total{cpu="3"} 2.46201381e+08
node_schedstat_timeslices_total{cpu="4"} 2.58191107e+08
node_schedstat_timeslices_total{cpu="5"} 2.6442866e+08
node_schedstat_timeslices_total{cpu="6"} 2.52569908e+08
node_schedstat_timeslices_total{cpu="7"} 2.33846236e+08
node_schedstat_timeslices_total{cpu="8"} 2.20612605e+08
node_schedstat_timeslices_total{cpu="9"} 2.15667221e+08
# HELP node_schedstat_waiting_seconds_total Number of seconds spent by processing waiting for this CPU.
# TYPE node_schedstat_waiting_seconds_total counter
node_schedstat_waiting_seconds_total{cpu="0"} 1126.53601952
node_schedstat_waiting_seconds_total{cpu="1"} 1156.254864609
node_schedstat_waiting_seconds_total{cpu="10"} 1311.816583186
node_schedstat_waiting_seconds_total{cpu="11"} 1215.578328449
node_schedstat_waiting_seconds_total{cpu="12"} 1118.228987357
node_schedstat_waiting_seconds_total{cpu="13"} 1362.943741814
node_schedstat_waiting_seconds_total{cpu="14"} 1182.106502636
node_schedstat_waiting_seconds_total{cpu="15"} 1092.942495264
node_schedstat_waiting_seconds_total{cpu="2"} 1176.692986927
node_schedstat_waiting_seconds_total{cpu="3"} 1131.573713549
node_schedstat_waiting_seconds_total{cpu="4"} 1105.298848192
node_schedstat_waiting_seconds_total{cpu="5"} 1148.380913183
node_schedstat_waiting_seconds_total{cpu="6"} 1142.378436564
node_schedstat_waiting_seconds_total{cpu="7"} 1092.694198739
node_schedstat_waiting_seconds_total{cpu="8"} 1160.274312659
node_schedstat_waiting_seconds_total{cpu="9"} 1218.530015667
# HELP node_scrape_collector_duration_seconds node_exporter: Duration of a collector scrape.
# TYPE node_scrape_collector_duration_seconds gauge
node_scrape_collector_duration_seconds{collector="arp"} 0.001251949
node_scrape_collector_duration_seconds{collector="bcache"} 1.777e-05
node_scrape_collector_duration_seconds{collector="bonding"} 3.5931e-05
node_scrape_collector_duration_seconds{collector="btrfs"} 1.298e-05
node_scrape_collector_duration_seconds{collector="conntrack"} 0.000298142
node_scrape_collector_duration_seconds{collector="cpu"} 0.003596498
node_scrape_collector_duration_seconds{collector="cpufreq"} 0.003007063
node_scrape_collector_duration_seconds{collector="diskstats"} 0.000180301
node_scrape_collector_duration_seconds{collector="edac"} 0.00132263
node_scrape_collector_duration_seconds{collector="entropy"} 0.00137847
node_scrape_collector_duration_seconds{collector="fibrechannel"} 0.001085368
node_scrape_collector_duration_seconds{collector="filefd"} 7.2561e-05
node_scrape_collector_duration_seconds{collector="filesystem"} 0.004909238
node_scrape_collector_duration_seconds{collector="hwmon"} 0.254004811
node_scrape_collector_duration_seconds{collector="infiniband"} 1.922e-05
node_scrape_collector_duration_seconds{collector="ipvs"} 4.775e-05
node_scrape_collector_duration_seconds{collector="loadavg"} 5.3521e-05
node_scrape_collector_duration_seconds{collector="mdadm"} 8.6861e-05
node_scrape_collector_duration_seconds{collector="meminfo"} 0.000194251
node_scrape_collector_duration_seconds{collector="netclass"} 0.007597499
node_scrape_collector_duration_seconds{collector="netdev"} 0.000567914
node_scrape_collector_duration_seconds{collector="netstat"} 0.003147374
node_scrape_collector_duration_seconds{collector="nfs"} 0.000108071
node_scrape_collector_duration_seconds{collector="nfsd"} 1.789e-05
node_scrape_collector_duration_seconds{collector="nvme"} 0.000130181
node_scrape_collector_duration_seconds{collector="os"} 0.001227999
node_scrape_collector_duration_seconds{collector="powersupplyclass"} 0.000464293
node_scrape_collector_duration_seconds{collector="pressure"} 3.86e-05
node_scrape_collector_duration_seconds{collector="rapl"} 0.000260882
node_scrape_collector_duration_seconds{collector="schedstat"} 0.002929763
node_scrape_collector_duration_seconds{collector="sockstat"} 0.0013373
node_scrape_collector_duration_seconds{collector="softnet"} 0.002761251
node_scrape_collector_duration_seconds{collector="stat"} 0.002027796
node_scrape_collector_duration_seconds{collector="tapestats"} 0.001592972
node_scrape_collector_duration_seconds{collector="textfile"} 2.572e-05
node_scrape_collector_duration_seconds{collector="thermal_zone"} 0.0013323
node_scrape_collector_duration_seconds{collector="time"} 9.4611e-05
node_scrape_collector_duration_seconds{collector="timex"} 1.713e-05
node_scrape_collector_duration_seconds{collector="udp_queues"} 0.00136594
node_scrape_collector_duration_seconds{collector="uname"} 1.079e-05
node_scrape_collector_duration_seconds{collector="vmstat"} 0.001786284
node_scrape_collector_duration_seconds{collector="xfs"} 0.003115454
node_scrape_collector_duration_seconds{collector="zfs"} 3.375e-05
# HELP node_scrape_collector_success node_exporter: Whether a collector succeeded.
# TYPE node_scrape_collector_success gauge
node_scrape_collector_success{collector="arp"} 1
node_scrape_collector_success{collector="bcache"} 1
node_scrape_collector_success{collector="bonding"} 0
node_scrape_collector_success{collector="btrfs"} 1
node_scrape_collector_success{collector="conntrack"} 1
node_scrape_collector_success{collector="cpu"} 1
node_scrape_collector_success{collector="cpufreq"} 1
node_scrape_collector_success{collector="diskstats"} 1
node_scrape_collector_success{collector="edac"} 1
node_scrape_collector_success{collector="entropy"} 1
node_scrape_collector_success{collector="fibrechannel"} 0
node_scrape_collector_success{collector="filefd"} 1
node_scrape_collector_success{collector="filesystem"} 1
node_scrape_collector_success{collector="hwmon"} 1
node_scrape_collector_success{collector="infiniband"} 0
node_scrape_collector_success{collector="ipvs"} 0
node_scrape_collector_success{collector="loadavg"} 1
node_scrape_collector_success{collector="mdadm"} 1
node_scrape_collector_success{collector="meminfo"} 1
node_scrape_collector_success{collector="netclass"} 1
node_scrape_collector_success{collector="netdev"} 1
node_scrape_collector_success{collector="netstat"} 1
node_scrape_collector_success{collector="nfs"} 0
node_scrape_collector_success{collector="nfsd"} 0
node_scrape_collector_success{collector="nvme"} 1
node_scrape_collector_success{collector="os"} 1
node_scrape_collector_success{collector="powersupplyclass"} 1
node_scrape_collector_success{collector="pressure"} 0
node_scrape_collector_success{collector="rapl"} 1
node_scrape_collector_success{collector="schedstat"} 1
node_scrape_collector_success{collector="sockstat"} 1
node_scrape_collector_success{collector="softnet"} 1
node_scrape_collector_success{collector="stat"} 1
node_scrape_collector_success{collector="tapestats"} 0
node_scrape_collector_success{collector="textfile"} 1
node_scrape_collector_success{collector="thermal_zone"} 1
node_scrape_collector_success{collector="time"} 1
node_scrape_collector_success{collector="timex"} 1
node_scrape_collector_success{collector="udp_queues"} 1
node_scrape_collector_success{collector="uname"} 1
node_scrape_collector_success{collector="vmstat"} 1
node_scrape_collector_success{collector="xfs"} 1
node_scrape_collector_success{collector="zfs"} 0
# HELP node_sockstat_FRAG6_inuse Number of FRAG6 sockets in state inuse.
# TYPE node_sockstat_FRAG6_inuse gauge
node_sockstat_FRAG6_inuse 0
# HELP node_sockstat_FRAG6_memory Number of FRAG6 sockets in state memory.
# TYPE node_sockstat_FRAG6_memory gauge
node_sockstat_FRAG6_memory 0
# HELP node_sockstat_FRAG_inuse Number of FRAG sockets in state inuse.
# TYPE node_sockstat_FRAG_inuse gauge
node_sockstat_FRAG_inuse 0
# HELP node_sockstat_FRAG_memory Number of FRAG sockets in state memory.
# TYPE node_sockstat_FRAG_memory gauge
node_sockstat_FRAG_memory 0
# HELP node_sockstat_RAW6_inuse Number of RAW6 sockets in state inuse.
# TYPE node_sockstat_RAW6_inuse gauge
node_sockstat_RAW6_inuse 0
# HELP node_sockstat_RAW_inuse Number of RAW sockets in state inuse.
# TYPE node_sockstat_RAW_inuse gauge
node_sockstat_RAW_inuse 0
# HELP node_sockstat_TCP6_inuse Number of TCP6 sockets in state inuse.
# TYPE node_sockstat_TCP6_inuse gauge
node_sockstat_TCP6_inuse 11
# HELP node_sockstat_TCP_alloc Number of TCP sockets in state alloc.
# TYPE node_sockstat_TCP_alloc gauge
node_sockstat_TCP_alloc 18
# HELP node_sockstat_TCP_inuse Number of TCP sockets in state inuse.
# TYPE node_sockstat_TCP_inuse gauge
node_sockstat_TCP_inuse 6
# HELP node_sockstat_TCP_mem Number of TCP sockets in state mem.
# TYPE node_sockstat_TCP_mem gauge
node_sockstat_TCP_mem 3
# HELP node_sockstat_TCP_mem_bytes Number of TCP sockets in state mem_bytes.
# TYPE node_sockstat_TCP_mem_bytes gauge
node_sockstat_TCP_mem_bytes 12288
# HELP node_sockstat_TCP_orphan Number of TCP sockets in state orphan.
# TYPE node_sockstat_TCP_orphan gauge
node_sockstat_TCP_orphan 0
# HELP node_sockstat_TCP_tw Number of TCP sockets in state tw.
# TYPE node_sockstat_TCP_tw gauge
node_sockstat_TCP_tw 0
# HELP node_sockstat_UDP6_inuse Number of UDP6 sockets in state inuse.
# TYPE node_sockstat_UDP6_inuse gauge
node_sockstat_UDP6_inuse 4
# HELP node_sockstat_UDPLITE6_inuse Number of UDPLITE6 sockets in state inuse.
# TYPE node_sockstat_UDPLITE6_inuse gauge
node_sockstat_UDPLITE6_inuse 0
# HELP node_sockstat_UDPLITE_inuse Number of UDPLITE sockets in state inuse.
# TYPE node_sockstat_UDPLITE_inuse gauge
node_sockstat_UDPLITE_inuse 0
# HELP node_sockstat_UDP_inuse Number of UDP sockets in state inuse.
# TYPE node_sockstat_UDP_inuse gauge
node_sockstat_UDP_inuse 8
# HELP node_sockstat_UDP_mem Number of UDP sockets in state mem.
# TYPE node_sockstat_UDP_mem gauge
node_sockstat_UDP_mem 3
# HELP node_sockstat_UDP_mem_bytes Number of UDP sockets in state mem_bytes.
# TYPE node_sockstat_UDP_mem_bytes gauge
node_sockstat_UDP_mem_bytes 12288
# HELP node_sockstat_sockets_used Number of IPv4 sockets in use.
# TYPE node_sockstat_sockets_used gauge
node_sockstat_sockets_used 1763
# HELP node_softnet_dropped_total Number of dropped packets
# TYPE node_softnet_dropped_total counter
node_softnet_dropped_total{cpu="0"} 0
node_softnet_dropped_total{cpu="1"} 0
node_softnet_dropped_total{cpu="10"} 0
node_softnet_dropped_total{cpu="11"} 0
node_softnet_dropped_total{cpu="12"} 0
node_softnet_dropped_total{cpu="13"} 0
node_softnet_dropped_total{cpu="14"} 0
node_softnet_dropped_total{cpu="15"} 0
node_softnet_dropped_total{cpu="2"} 0
node_softnet_dropped_total{cpu="3"} 0
node_softnet_dropped_total{cpu="4"} 0
node_softnet_dropped_total{cpu="5"} 0
node_softnet_dropped_total{cpu="6"} 0
node_softnet_dropped_total{cpu="7"} 0
node_softnet_dropped_total{cpu="8"} 0
node_softnet_dropped_total{cpu="9"} 0
# HELP node_softnet_processed_total Number of processed packets
# TYPE node_softnet_processed_total counter
node_softnet_processed_total{cpu="0"} 4014
node_softnet_processed_total{cpu="1"} 4171
node_softnet_processed_total{cpu="10"} 3446
node_softnet_processed_total{cpu="11"} 3812
node_softnet_processed_total{cpu="12"} 285112
node_softnet_processed_total{cpu="13"} 3957
node_softnet_processed_total{cpu="14"} 5550
node_softnet_processed_total{cpu="15"} 5591
node_softnet_processed_total{cpu="2"} 4092
node_softnet_processed_total{cpu="3"} 3740
node_softnet_processed_total{cpu="4"} 5326
node_softnet_processed_total{cpu="5"} 5572
node_softnet_processed_total{cpu="6"} 5727
node_softnet_processed_total{cpu="7"} 6132
node_softnet_processed_total{cpu="8"} 3755
node_softnet_processed_total{cpu="9"} 5014
# HELP node_softnet_times_squeezed_total Number of times processing packets ran out of quota
# TYPE node_softnet_times_squeezed_total counter
node_softnet_times_squeezed_total{cpu="0"} 0
node_softnet_times_squeezed_total{cpu="1"} 0
node_softnet_times_squeezed_total{cpu="10"} 0
node_softnet_times_squeezed_total{cpu="11"} 0
node_softnet_times_squeezed_total{cpu="12"} 1
node_softnet_times_squeezed_total{cpu="13"} 0
node_softnet_times_squeezed_total{cpu="14"} 0
node_softnet_times_squeezed_total{cpu="15"} 0
node_softnet_times_squeezed_total{cpu="2"} 0
node_softnet_times_squeezed_total{cpu="3"} 0
node_softnet_times_squeezed_total{cpu="4"} 0
node_softnet_times_squeezed_total{cpu="5"} 0
node_softnet_times_squeezed_total{cpu="6"} 0
node_softnet_times_squeezed_total{cpu="7"} 0
node_softnet_times_squeezed_total{cpu="8"} 0
node_softnet_times_squeezed_total{cpu="9"} 0
# HELP node_textfile_scrape_error 1 if there was an error opening or reading a file, 0 otherwise
# TYPE node_textfile_scrape_error gauge
node_textfile_scrape_error 0
# HELP node_time_seconds System time in seconds since epoch (1970).
# TYPE node_time_seconds gauge
node_time_seconds 1.6347843276190393e+09
# HELP node_time_zone_offset_seconds System time zone offset in seconds.
# TYPE node_time_zone_offset_seconds gauge
node_time_zone_offset_seconds{time_zone="CST"} 28800
# HELP node_timex_estimated_error_seconds Estimated error in seconds.
# TYPE node_timex_estimated_error_seconds gauge
node_timex_estimated_error_seconds 0.010755
# HELP node_timex_frequency_adjustment_ratio Local clock frequency adjustment.
# TYPE node_timex_frequency_adjustment_ratio gauge
node_timex_frequency_adjustment_ratio 1.0000072019195556
# HELP node_timex_loop_time_constant Phase-locked loop time constant.
# TYPE node_timex_loop_time_constant gauge
node_timex_loop_time_constant 2
# HELP node_timex_maxerror_seconds Maximum error in seconds.
# TYPE node_timex_maxerror_seconds gauge
node_timex_maxerror_seconds 16
# HELP node_timex_offset_seconds Time offset in between local system and reference clock.
# TYPE node_timex_offset_seconds gauge
node_timex_offset_seconds 0
# HELP node_timex_pps_calibration_total Pulse per second count of calibration intervals.
# TYPE node_timex_pps_calibration_total counter
node_timex_pps_calibration_total 0
# HELP node_timex_pps_error_total Pulse per second count of calibration errors.
# TYPE node_timex_pps_error_total counter
node_timex_pps_error_total 0
# HELP node_timex_pps_frequency_hertz Pulse per second frequency.
# TYPE node_timex_pps_frequency_hertz gauge
node_timex_pps_frequency_hertz 0
# HELP node_timex_pps_jitter_seconds Pulse per second jitter.
# TYPE node_timex_pps_jitter_seconds gauge
node_timex_pps_jitter_seconds 0
# HELP node_timex_pps_jitter_total Pulse per second count of jitter limit exceeded events.
# TYPE node_timex_pps_jitter_total counter
node_timex_pps_jitter_total 0
# HELP node_timex_pps_shift_seconds Pulse per second interval duration.
# TYPE node_timex_pps_shift_seconds gauge
node_timex_pps_shift_seconds 0
# HELP node_timex_pps_stability_exceeded_total Pulse per second count of stability limit exceeded events.
# TYPE node_timex_pps_stability_exceeded_total counter
node_timex_pps_stability_exceeded_total 0
# HELP node_timex_pps_stability_hertz Pulse per second stability, average of recent frequency changes.
# TYPE node_timex_pps_stability_hertz gauge
node_timex_pps_stability_hertz 0
# HELP node_timex_status Value of the status array bits.
# TYPE node_timex_status gauge
node_timex_status 8256
# HELP node_timex_sync_status Is clock synchronized to a reliable server (1 = yes, 0 = no).
# TYPE node_timex_sync_status gauge
node_timex_sync_status 0
# HELP node_timex_tai_offset_seconds International Atomic Time (TAI) offset.
# TYPE node_timex_tai_offset_seconds gauge
node_timex_tai_offset_seconds 37
# HELP node_timex_tick_seconds Seconds between clock ticks.
# TYPE node_timex_tick_seconds gauge
node_timex_tick_seconds 0.01
# HELP node_udp_queues Number of allocated memory in the kernel for UDP datagrams in bytes.
# TYPE node_udp_queues gauge
node_udp_queues{ip="v4",queue="rx"} 0
node_udp_queues{ip="v4",queue="tx"} 0
node_udp_queues{ip="v6",queue="rx"} 0
node_udp_queues{ip="v6",queue="tx"} 0
# HELP node_uname_info Labeled system information as provided by the uname system call.
# TYPE node_uname_info gauge
node_uname_info{domainname="(none)",machine="x86_64",nodename="localhost.localdomain",release="4.18.0-338.el8.x86_64",sysname="Linux",version="#1 SMP Fri Aug 27 17:32:14 UTC 2021"} 1
# HELP node_vmstat_oom_kill /proc/vmstat information field oom_kill.
# TYPE node_vmstat_oom_kill untyped
node_vmstat_oom_kill 1
# HELP node_vmstat_pgfault /proc/vmstat information field pgfault.
# TYPE node_vmstat_pgfault untyped
node_vmstat_pgfault 1.1126746086e+10
# HELP node_vmstat_pgmajfault /proc/vmstat information field pgmajfault.
# TYPE node_vmstat_pgmajfault untyped
node_vmstat_pgmajfault 296097
# HELP node_vmstat_pgpgin /proc/vmstat information field pgpgin.
# TYPE node_vmstat_pgpgin untyped
node_vmstat_pgpgin 1.77019217e+08
# HELP node_vmstat_pgpgout /proc/vmstat information field pgpgout.
# TYPE node_vmstat_pgpgout untyped
node_vmstat_pgpgout 4.7810924e+07
# HELP node_vmstat_pswpin /proc/vmstat information field pswpin.
# TYPE node_vmstat_pswpin untyped
node_vmstat_pswpin 397015
# HELP node_vmstat_pswpout /proc/vmstat information field pswpout.
# TYPE node_vmstat_pswpout untyped
node_vmstat_pswpout 1.45314e+06
# HELP node_xfs_allocation_btree_compares_total Number of allocation B-tree compares for a filesystem.
# TYPE node_xfs_allocation_btree_compares_total counter
node_xfs_allocation_btree_compares_total{device="dm-0"} 0
node_xfs_allocation_btree_compares_total{device="dm-2"} 0
# HELP node_xfs_allocation_btree_lookups_total Number of allocation B-tree lookups for a filesystem.
# TYPE node_xfs_allocation_btree_lookups_total counter
node_xfs_allocation_btree_lookups_total{device="dm-0"} 0
node_xfs_allocation_btree_lookups_total{device="dm-2"} 0
# HELP node_xfs_allocation_btree_records_deleted_total Number of allocation B-tree records deleted for a filesystem.
# TYPE node_xfs_allocation_btree_records_deleted_total counter
node_xfs_allocation_btree_records_deleted_total{device="dm-0"} 0
node_xfs_allocation_btree_records_deleted_total{device="dm-2"} 0
# HELP node_xfs_allocation_btree_records_inserted_total Number of allocation B-tree records inserted for a filesystem.
# TYPE node_xfs_allocation_btree_records_inserted_total counter
node_xfs_allocation_btree_records_inserted_total{device="dm-0"} 0
node_xfs_allocation_btree_records_inserted_total{device="dm-2"} 0
# HELP node_xfs_block_map_btree_compares_total Number of block map B-tree compares for a filesystem.
# TYPE node_xfs_block_map_btree_compares_total counter
node_xfs_block_map_btree_compares_total{device="dm-0"} 0
node_xfs_block_map_btree_compares_total{device="dm-2"} 0
# HELP node_xfs_block_map_btree_lookups_total Number of block map B-tree lookups for a filesystem.
# TYPE node_xfs_block_map_btree_lookups_total counter
node_xfs_block_map_btree_lookups_total{device="dm-0"} 0
node_xfs_block_map_btree_lookups_total{device="dm-2"} 0
# HELP node_xfs_block_map_btree_records_deleted_total Number of block map B-tree records deleted for a filesystem.
# TYPE node_xfs_block_map_btree_records_deleted_total counter
node_xfs_block_map_btree_records_deleted_total{device="dm-0"} 0
node_xfs_block_map_btree_records_deleted_total{device="dm-2"} 0
# HELP node_xfs_block_map_btree_records_inserted_total Number of block map B-tree records inserted for a filesystem.
# TYPE node_xfs_block_map_btree_records_inserted_total counter
node_xfs_block_map_btree_records_inserted_total{device="dm-0"} 0
node_xfs_block_map_btree_records_inserted_total{device="dm-2"} 0
# HELP node_xfs_block_mapping_extent_list_compares_total Number of extent list compares for a filesystem.
# TYPE node_xfs_block_mapping_extent_list_compares_total counter
node_xfs_block_mapping_extent_list_compares_total{device="dm-0"} 0
node_xfs_block_mapping_extent_list_compares_total{device="dm-2"} 0
# HELP node_xfs_block_mapping_extent_list_deletions_total Number of extent list deletions for a filesystem.
# TYPE node_xfs_block_mapping_extent_list_deletions_total counter
node_xfs_block_mapping_extent_list_deletions_total{device="dm-0"} 55742
node_xfs_block_mapping_extent_list_deletions_total{device="dm-2"} 71204
# HELP node_xfs_block_mapping_extent_list_insertions_total Number of extent list insertions for a filesystem.
# TYPE node_xfs_block_mapping_extent_list_insertions_total counter
node_xfs_block_mapping_extent_list_insertions_total{device="dm-0"} 21023
node_xfs_block_mapping_extent_list_insertions_total{device="dm-2"} 137576
# HELP node_xfs_block_mapping_extent_list_lookups_total Number of extent list lookups for a filesystem.
# TYPE node_xfs_block_mapping_extent_list_lookups_total counter
node_xfs_block_mapping_extent_list_lookups_total{device="dm-0"} 5.42566741e+08
node_xfs_block_mapping_extent_list_lookups_total{device="dm-2"} 2.2313968e+07
# HELP node_xfs_block_mapping_reads_total Number of block map for read operations for a filesystem.
# TYPE node_xfs_block_mapping_reads_total counter
node_xfs_block_mapping_reads_total{device="dm-0"} 5.41622137e+08
node_xfs_block_mapping_reads_total{device="dm-2"} 4.265772e+06
# HELP node_xfs_block_mapping_unmaps_total Number of block unmaps (deletes) for a filesystem.
# TYPE node_xfs_block_mapping_unmaps_total counter
node_xfs_block_mapping_unmaps_total{device="dm-0"} 111940
node_xfs_block_mapping_unmaps_total{device="dm-2"} 129556
# HELP node_xfs_block_mapping_writes_total Number of block map for write operations for a filesystem.
# TYPE node_xfs_block_mapping_writes_total counter
node_xfs_block_mapping_writes_total{device="dm-0"} 747504
node_xfs_block_mapping_writes_total{device="dm-2"} 1.735712e+07
# HELP node_xfs_directory_operation_create_total Number of times a new directory entry was created for a filesystem.
# TYPE node_xfs_directory_operation_create_total counter
node_xfs_directory_operation_create_total{device="dm-0"} 68262
node_xfs_directory_operation_create_total{device="dm-2"} 118353
# HELP node_xfs_directory_operation_getdents_total Number of times the directory getdents operation was performed for a filesystem.
# TYPE node_xfs_directory_operation_getdents_total counter
node_xfs_directory_operation_getdents_total{device="dm-0"} 2.96095059e+08
node_xfs_directory_operation_getdents_total{device="dm-2"} 1.0654669e+07
# HELP node_xfs_directory_operation_lookup_total Number of file name directory lookups which miss the operating systems directory name lookup cache.
# TYPE node_xfs_directory_operation_lookup_total counter
node_xfs_directory_operation_lookup_total{device="dm-0"} 181489
node_xfs_directory_operation_lookup_total{device="dm-2"} 1.390528e+06
# HELP node_xfs_directory_operation_remove_total Number of times an existing directory entry was created for a filesystem.
# TYPE node_xfs_directory_operation_remove_total counter
node_xfs_directory_operation_remove_total{device="dm-0"} 60516
node_xfs_directory_operation_remove_total{device="dm-2"} 87149
# HELP node_xfs_extent_allocation_blocks_allocated_total Number of blocks allocated for a filesystem.
# TYPE node_xfs_extent_allocation_blocks_allocated_total counter
node_xfs_extent_allocation_blocks_allocated_total{device="dm-0"} 394660
node_xfs_extent_allocation_blocks_allocated_total{device="dm-2"} 6.847057e+06
# HELP node_xfs_extent_allocation_blocks_freed_total Number of blocks freed for a filesystem.
# TYPE node_xfs_extent_allocation_blocks_freed_total counter
node_xfs_extent_allocation_blocks_freed_total{device="dm-0"} 201797
node_xfs_extent_allocation_blocks_freed_total{device="dm-2"} 4.792731e+06
# HELP node_xfs_extent_allocation_extents_allocated_total Number of extents allocated for a filesystem.
# TYPE node_xfs_extent_allocation_extents_allocated_total counter
node_xfs_extent_allocation_extents_allocated_total{device="dm-0"} 9635
node_xfs_extent_allocation_extents_allocated_total{device="dm-2"} 67234
# HELP node_xfs_extent_allocation_extents_freed_total Number of extents freed for a filesystem.
# TYPE node_xfs_extent_allocation_extents_freed_total counter
node_xfs_extent_allocation_extents_freed_total{device="dm-0"} 4188
node_xfs_extent_allocation_extents_freed_total{device="dm-2"} 42639
# HELP node_xfs_inode_operation_attempts_total Number of times the OS looked for an XFS inode in the inode cache.
# TYPE node_xfs_inode_operation_attempts_total counter
node_xfs_inode_operation_attempts_total{device="dm-0"} 154976
node_xfs_inode_operation_attempts_total{device="dm-2"} 1.29795e+06
# HELP node_xfs_inode_operation_attribute_changes_total Number of times the OS explicitly changed the attributes of an XFS inode.
# TYPE node_xfs_inode_operation_attribute_changes_total counter
node_xfs_inode_operation_attribute_changes_total{device="dm-0"} 29187
node_xfs_inode_operation_attribute_changes_total{device="dm-2"} 75337
# HELP node_xfs_inode_operation_duplicates_total Number of times the OS tried to add a missing XFS inode to the inode cache, but found it had already been added by another process.
# TYPE node_xfs_inode_operation_duplicates_total counter
node_xfs_inode_operation_duplicates_total{device="dm-0"} 0
node_xfs_inode_operation_duplicates_total{device="dm-2"} 0
# HELP node_xfs_inode_operation_found_total Number of times the OS looked for and found an XFS inode in the inode cache.
# TYPE node_xfs_inode_operation_found_total counter
node_xfs_inode_operation_found_total{device="dm-0"} 83400
node_xfs_inode_operation_found_total{device="dm-2"} 65654
# HELP node_xfs_inode_operation_missed_total Number of times the OS looked for an XFS inode in the cache, but did not find it.
# TYPE node_xfs_inode_operation_missed_total counter
node_xfs_inode_operation_missed_total{device="dm-0"} 71576
node_xfs_inode_operation_missed_total{device="dm-2"} 1.232296e+06
# HELP node_xfs_inode_operation_reclaims_total Number of times the OS reclaimed an XFS inode from the inode cache to free memory for another purpose.
# TYPE node_xfs_inode_operation_reclaims_total counter
node_xfs_inode_operation_reclaims_total{device="dm-0"} 64751
node_xfs_inode_operation_reclaims_total{device="dm-2"} 315253
# HELP node_xfs_inode_operation_recycled_total Number of times the OS found an XFS inode in the cache, but could not use it as it was being recycled.
# TYPE node_xfs_inode_operation_recycled_total counter
node_xfs_inode_operation_recycled_total{device="dm-0"} 1
node_xfs_inode_operation_recycled_total{device="dm-2"} 0
# HELP node_xfs_read_calls_total Number of read(2) system calls made to files in a filesystem.
# TYPE node_xfs_read_calls_total counter
node_xfs_read_calls_total{device="dm-0"} 1.240916917e+09
node_xfs_read_calls_total{device="dm-2"} 4.1618242e+07
# HELP node_xfs_vnode_active_total Number of vnodes not on free lists for a filesystem.
# TYPE node_xfs_vnode_active_total counter
node_xfs_vnode_active_total{device="dm-0"} 6825
node_xfs_vnode_active_total{device="dm-2"} 917043
# HELP node_xfs_vnode_allocate_total Number of times vn_alloc called for a filesystem.
# TYPE node_xfs_vnode_allocate_total counter
node_xfs_vnode_allocate_total{device="dm-0"} 0
node_xfs_vnode_allocate_total{device="dm-2"} 0
# HELP node_xfs_vnode_get_total Number of times vn_get called for a filesystem.
# TYPE node_xfs_vnode_get_total counter
node_xfs_vnode_get_total{device="dm-0"} 0
node_xfs_vnode_get_total{device="dm-2"} 0
# HELP node_xfs_vnode_hold_total Number of times vn_hold called for a filesystem.
# TYPE node_xfs_vnode_hold_total counter
node_xfs_vnode_hold_total{device="dm-0"} 0
node_xfs_vnode_hold_total{device="dm-2"} 0
# HELP node_xfs_vnode_reclaim_total Number of times vn_reclaim called for a filesystem.
# TYPE node_xfs_vnode_reclaim_total counter
node_xfs_vnode_reclaim_total{device="dm-0"} 127473
node_xfs_vnode_reclaim_total{device="dm-2"} 344623
# HELP node_xfs_vnode_release_total Number of times vn_rele called for a filesystem.
# TYPE node_xfs_vnode_release_total counter
node_xfs_vnode_release_total{device="dm-0"} 127473
node_xfs_vnode_release_total{device="dm-2"} 344623
# HELP node_xfs_vnode_remove_total Number of times vn_remove called for a filesystem.
# TYPE node_xfs_vnode_remove_total counter
node_xfs_vnode_remove_total{device="dm-0"} 127473
node_xfs_vnode_remove_total{device="dm-2"} 344623
# HELP node_xfs_write_calls_total Number of write(2) system calls made to files in a filesystem.
# TYPE node_xfs_write_calls_total counter
node_xfs_write_calls_total{device="dm-0"} 652775
node_xfs_write_calls_total{device="dm-2"} 1.6660521e+07
# HELP process_cpu_seconds_total Total user and system CPU time spent in seconds.
# TYPE process_cpu_seconds_total counter
process_cpu_seconds_total 0
# HELP process_max_fds Maximum number of open file descriptors.
# TYPE process_max_fds gauge
process_max_fds 262144
# HELP process_open_fds Number of open file descriptors.
# TYPE process_open_fds gauge
process_open_fds 10
# HELP process_resident_memory_bytes Resident memory size in bytes.
# TYPE process_resident_memory_bytes gauge
process_resident_memory_bytes 1.605632e+07
# HELP process_start_time_seconds Start time of the process since unix epoch in seconds.
# TYPE process_start_time_seconds gauge
process_start_time_seconds 1.6347843174e+09
# HELP process_virtual_memory_bytes Virtual memory size in bytes.
# TYPE process_virtual_memory_bytes gauge
process_virtual_memory_bytes 1.12881664e+09
# HELP process_virtual_memory_max_bytes Maximum amount of virtual memory available in bytes.
# TYPE process_virtual_memory_max_bytes gauge
process_virtual_memory_max_bytes 1.8446744073709552e+19
# HELP promhttp_metric_handler_errors_total Total number of internal errors encountered by the promhttp metric handler.
# TYPE promhttp_metric_handler_errors_total counter
promhttp_metric_handler_errors_total{cause="encoding"} 0
promhttp_metric_handler_errors_total{cause="gathering"} 0
# HELP promhttp_metric_handler_requests_in_flight Current number of scrapes being served.
# TYPE promhttp_metric_handler_requests_in_flight gauge
promhttp_metric_handler_requests_in_flight 1
# HELP promhttp_metric_handler_requests_total Total number of scrapes by HTTP status code.
# TYPE promhttp_metric_handler_requests_total counter
promhttp_metric_handler_requests_total{code="200"} 0
promhttp_metric_handler_requests_total{code="500"} 0
promhttp_metric_handler_requests_total{code="503"} 0"##;

fn bench_parse_text(c: &mut Criterion) {
    let mut group = c.benchmark_group("parse");
    group.measurement_time(std::time::Duration::from_secs(10));

    group.bench_function("1", |b| {
        b.iter(|| {
            prometheus::parse_text(INPUT).unwrap();
        })
    });

    group.finish();
}

fn bench_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("throughput");
    group.measurement_time(std::time::Duration::from_secs(10));
    group.noise_threshold(0.03);

    group.throughput(Throughput::Bytes(INPUT.len() as u64));
    group.bench_with_input("1", INPUT, |b, input| {
        b.iter(|| {
            prometheus::parse_text(input).unwrap();
        })
    });

    group.finish();
}

criterion_group!(
    name = benches;
    config = Criterion::default().with_profiler(PProfProfiler::new(100, Output::Protobuf));
    targets = bench_parse_text, bench_throughput
);
criterion_main!(benches);
