#!/bin/bash

case $1 in
  "--list-gpus")
  echo "GPU 0: NVIDIA GeForce RTX 2080 SUPER (UUID: GPU-df6e7a7c-7314-46f8-abc4-b88b36dcf3aa)"
  ;;

  "--help-query-gpu")
  cat << EOF
List of valid properties to query for the switch "--query-gpu=":

"timestamp"
The timestamp of when the query was made in format "YYYY/MM/DD HH:MM:SS.msec".

"driver_version"
The version of the installed NVIDIA display driver. This is an alphanumeric string.

"count"
The number of NVIDIA GPUs in the system.

"name" or "gpu_name"
The official product name of the GPU. This is an alphanumeric string. For all products.

"serial" or "gpu_serial"
This number matches the serial number physically printed on each board. It is a globally unique immutable alphanumeric value.

"uuid" or "gpu_uuid"
This value is the globally unique immutable alphanumeric identifier of the GPU. It does not correspond to any physical label on the board.

"pci.bus_id" or "gpu_bus_id"
PCI bus id as "domain:bus:device.function", in hex.

"pci.domain"
PCI domain number, in hex.

"pci.bus"
PCI bus number, in hex.

"pci.device"
PCI device number, in hex.

"pci.device_id"
PCI vendor device id, in hex

"pci.sub_device_id"
PCI Sub System id, in hex

"pcie.link.gen.current"
The current PCI-E link generation. These may be reduced when the GPU is not in use.

"pcie.link.gen.max"
The maximum PCI-E link generation possible with this GPU and system configuration. For example, if the GPU supports a higher PCIe generation than the system supports then this reports the system PCIe generation.

"pcie.link.width.current"
The current PCI-E link width. These may be reduced when the GPU is not in use.

"pcie.link.width.max"
The maximum PCI-E link width possible with this GPU and system configuration. For example, if the GPU supports a higher PCIe generation than the system supports then this reports the system PCIe generation.

"index"
Zero based index of the GPU. Can change at each boot.

"display_mode"
A flag that indicates whether a physical display (e.g. monitor) is currently connected to any of the GPU's connectors. "Enabled" indicates an attached display. "Disabled" indicates otherwise.

"display_active"
A flag that indicates whether a display is initialized on the GPU's (e.g. memory is allocated on the device for display). Display can be active even when no monitor is physically attached. "Enabled" indicates an active display. "Disabled" indicates otherwise.

"persistence_mode"
A flag that indicates whether persistence mode is enabled for the GPU. Value is either "Enabled" or "Disabled". When persistence mode is enabled the NVIDIA driver remains loaded even when no active clients, such as X11 or nvidia-smi, exist. This minimizes the driver load latency associated with running dependent apps, such as CUDA programs. Linux only.

"accounting.mode"
A flag that indicates whether accounting mode is enabled for the GPU. Value is either "Enabled" or "Disabled". When accounting is enabled statistics are calculated for each compute process running on the GPU.Statistics can be queried during the lifetime or after termination of the process.The execution time of process is reported as 0 while the process is in running state and updated to actualexecution time after the process has terminated. See --help-query-accounted-apps for more info.

"accounting.buffer_size"
The size of the circular buffer that holds list of processes that can be queried for accounting stats. This is the maximum number of processes that accounting information will be stored for before information about oldest processes will get overwritten by information about new processes.

Section about driver_model properties
On Windows, the TCC and WDDM driver models are supported. The driver model can be changed with the (-dm) or (-fdm) flags. The TCC driver model is optimized for compute applications. I.E. kernel launch times will be quicker with TCC. The WDDM driver model is designed for graphics applications and is not recommended for compute applications. Linux does not support multiple driver models, and will always have the value of "N/A". Only for selected products. Please see feature matrix in NVML documentation.

"driver_model.current"
The driver model currently in use. Always "N/A" on Linux.

"driver_model.pending"
The driver model that will be used on the next reboot. Always "N/A" on Linux.

"vbios_version"
The BIOS of the GPU board.

Section about inforom properties
Version numbers for each object in the GPU board's inforom storage. The inforom is a small, persistent store of configuration and state data for the GPU. All inforom version fields are numerical. It can be useful to know these version numbers because some GPU features are only available with inforoms of a certain version or higher.

"inforom.img" or "inforom.image"
Global version of the infoROM image. Image version just like VBIOS version uniquely describes the exact version of the infoROM flashed on the board in contrast to infoROM object version which is only an indicator of supported features.

"inforom.oem"
Version for the OEM configuration data.

"inforom.ecc"
Version for the ECC recording data.

"inforom.pwr" or "inforom.power"
Version for the power management data.

Section about gom properties
GOM allows to reduce power usage and optimize GPU throughput by disabling GPU features. Each GOM is designed to meet specific user needs.
In "All On" mode everything is enabled and running at full speed.
The "Compute" mode is designed for running only compute tasks. Graphics operations are not allowed.
The "Low Double Precision" mode is designed for running graphics applications that don't require high bandwidth double precision.
GOM can be changed with the (--gom) flag.

"gom.current" or "gpu_operation_mode.current"
The GOM currently in use.

"gom.pending" or "gpu_operation_mode.pending"
The GOM that will be used on the next reboot.

"fan.speed"
The fan speed value is the percent of the product's maximum noise tolerance fan speed that the device's fan is currently intended to run at. This value may exceed 100% in certain cases. Note: The reported speed is the intended fan speed. If the fan is physically blocked and unable to spin, this output will not match the actual fan speed. Many parts do not report fan speeds because they rely on cooling via fans in the surrounding enclosure.

"pstate"
The current performance state for the GPU. States range from P0 (maximum performance) to P12 (minimum performance).

Section about clocks_throttle_reasons properties
Retrieves information about factors that are reducing the frequency of clocks. If all throttle reasons are returned as "Not Active" it means that clocks are running as high as possible.

"clocks_throttle_reasons.supported"
Bitmask of supported clock throttle reasons. See nvml.h for more details.

"clocks_throttle_reasons.active"
Bitmask of active clock throttle reasons. See nvml.h for more details.

"clocks_throttle_reasons.gpu_idle"
Nothing is running on the GPU and the clocks are dropping to Idle state. This limiter may be removed in a later release.

"clocks_throttle_reasons.applications_clocks_setting"
GPU clocks are limited by applications clocks setting. E.g. can be changed by nvidia-smi --applications-clocks=

"clocks_throttle_reasons.sw_power_cap"
SW Power Scaling algorithm is reducing the clocks below requested clocks because the GPU is consuming too much power. E.g. SW power cap limit can be changed with nvidia-smi --power-limit=

"clocks_throttle_reasons.hw_slowdown"
HW Slowdown (reducing the core clocks by a factor of 2 or more) is engaged. This is an indicator of:
 HW Thermal Slowdown: temperature being too high
 HW Power Brake Slowdown: External Power Brake Assertion is triggered (e.g. by the system power supply)
 * Power draw is too high and Fast Trigger protection is reducing the clocks
 * May be also reported during PState or clock change
 * This behavior may be removed in a later release

"clocks_throttle_reasons.hw_thermal_slowdown"
HW Thermal Slowdown (reducing the core clocks by a factor of 2 or more) is engaged. This is an indicator of temperature being too high

"clocks_throttle_reasons.hw_power_brake_slowdown"
HW Power Brake Slowdown (reducing the core clocks by a factor of 2 or more) is engaged. This is an indicator of External Power Brake Assertion being triggered (e.g. by the system power supply)

"clocks_throttle_reasons.sw_thermal_slowdown"
SW Thermal capping algorithm is reducing clocks below requested clocks because GPU temperature is higher than Max Operating Temp.

"clocks_throttle_reasons.sync_boost"
Sync Boost This GPU has been added to a Sync boost group with nvidia-smi or DCGM in
 * order to maximize performance per watt. All GPUs in the sync boost group
 * will boost to the minimum possible clocks across the entire group. Look at
 * the throttle reasons for other GPUs in the system to see why those GPUs are
 * holding this one at lower clocks.

Section about memory properties
On-board memory information. Reported total memory is affected by ECC state. If ECC is enabled the total available memory is decreased by several percent, due to the requisite parity bits. The driver may also reserve a small amount of memory for internal use, even without active work on the GPU.

"memory.total"
Total installed GPU memory.

"memory.used"
Total memory allocated by active contexts.

"memory.free"
Total free memory.

"compute_mode"
The compute mode flag indicates whether individual or multiple compute applications may run on the GPU.
"Default" means multiple contexts are allowed per device.
"Exclusive_Process" means only one context is allowed per device, usable from multiple threads at a time.
"Prohibited" means no contexts are allowed per device (no compute apps).

Section about utilization properties
Utilization rates report how busy each GPU is over time, and can be used to determine how much an application is using the GPUs in the system.

"utilization.gpu"
Percent of time over the past sample period during which one or more kernels was executing on the GPU.
The sample period may be between 1 second and 1/6 second depending on the product.

"utilization.memory"
Percent of time over the past sample period during which global (device) memory was being read or written.
The sample period may be between 1 second and 1/6 second depending on the product.

Section about encoder.stats properties
Encoder stats report number of encoder sessions, average FPS and average latency in ms for given GPUs in the system.

"encoder.stats.sessionCount"
Number of encoder sessions running on the GPU.

"encoder.stats.averageFps"
Average FPS of all sessions running on the GPU.

"encoder.stats.averageLatency"
Average latency in microseconds of all sessions running on the GPU.

Section about ecc.mode properties
A flag that indicates whether ECC support is enabled. May be either "Enabled" or "Disabled". Changes to ECC mode require a reboot. Requires Inforom ECC object version 1.0 or higher.

"ecc.mode.current"
The ECC mode that the GPU is currently operating under.

"ecc.mode.pending"
The ECC mode that the GPU will operate under after the next reboot.

Section about ecc.errors properties
NVIDIA GPUs can provide error counts for various types of ECC errors. Some ECC errors are either single or double bit, where single bit errors are corrected and double bit errors are uncorrectable. Texture memory errors may be correctable via resend or uncorrectable if the resend fails. These errors are available across two timescales (volatile and aggregate). Single bit ECC errors are automatically corrected by the HW and do not result in data corruption. Double bit errors are detected but not corrected. Please see the ECC documents on the web for information on compute application behavior when double bit errors occur. Volatile error counters track the number of errors detected since the last driver load. Aggregate error counts persist indefinitely and thus act as a lifetime counter.

"ecc.errors.corrected.volatile.device_memory"
Errors detected in global device memory.

"ecc.errors.corrected.volatile.dram"
Errors detected in global device memory.

"ecc.errors.corrected.volatile.register_file"
Errors detected in register file memory.

"ecc.errors.corrected.volatile.l1_cache"
Errors detected in the L1 cache.

"ecc.errors.corrected.volatile.l2_cache"
Errors detected in the L2 cache.

"ecc.errors.corrected.volatile.texture_memory"
Parity errors detected in texture memory.

"ecc.errors.corrected.volatile.cbu"
Parity errors detected in CBU.

"ecc.errors.corrected.volatile.sram"
Errors detected in global SRAMs.

"ecc.errors.corrected.volatile.total"
Total errors detected across entire chip.

"ecc.errors.corrected.aggregate.device_memory"
Errors detected in global device memory.

"ecc.errors.corrected.aggregate.dram"
Errors detected in global device memory.

"ecc.errors.corrected.aggregate.register_file"
Errors detected in register file memory.

"ecc.errors.corrected.aggregate.l1_cache"
Errors detected in the L1 cache.

"ecc.errors.corrected.aggregate.l2_cache"
Errors detected in the L2 cache.

"ecc.errors.corrected.aggregate.texture_memory"
Parity errors detected in texture memory.

"ecc.errors.corrected.aggregate.cbu"
Parity errors detected in CBU.

"ecc.errors.corrected.aggregate.sram"
Errors detected in global SRAMs.

"ecc.errors.corrected.aggregate.total"
Total errors detected across entire chip.

"ecc.errors.uncorrected.volatile.device_memory"
Errors detected in global device memory.

"ecc.errors.uncorrected.volatile.dram"
Errors detected in global device memory.

"ecc.errors.uncorrected.volatile.register_file"
Errors detected in register file memory.

"ecc.errors.uncorrected.volatile.l1_cache"
Errors detected in the L1 cache.

"ecc.errors.uncorrected.volatile.l2_cache"
Errors detected in the L2 cache.

"ecc.errors.uncorrected.volatile.texture_memory"
Parity errors detected in texture memory.

"ecc.errors.uncorrected.volatile.cbu"
Parity errors detected in CBU.

"ecc.errors.uncorrected.volatile.sram"
Errors detected in global SRAMs.

"ecc.errors.uncorrected.volatile.total"
Total errors detected across entire chip.

"ecc.errors.uncorrected.aggregate.device_memory"
Errors detected in global device memory.

"ecc.errors.uncorrected.aggregate.dram"
Errors detected in global device memory.

"ecc.errors.uncorrected.aggregate.register_file"
Errors detected in register file memory.

"ecc.errors.uncorrected.aggregate.l1_cache"
Errors detected in the L1 cache.

"ecc.errors.uncorrected.aggregate.l2_cache"
Errors detected in the L2 cache.

"ecc.errors.uncorrected.aggregate.texture_memory"
Parity errors detected in texture memory.

"ecc.errors.uncorrected.aggregate.cbu"
Parity errors detected in CBU.

"ecc.errors.uncorrected.aggregate.sram"
Errors detected in global SRAMs.

"ecc.errors.uncorrected.aggregate.total"
Total errors detected across entire chip.

Section about retired_pages properties
NVIDIA GPUs can retire pages of GPU device memory when they become unreliable. This can happen when multiple single bit ECC errors occur for the same page, or on a double bit ECC error. When a page is retired, the NVIDIA driver will hide it such that no driver, or application memory allocations can access it.

"retired_pages.single_bit_ecc.count" or "retired_pages.sbe"
The number of GPU device memory pages that have been retired due to multiple single bit ECC errors.

"retired_pages.double_bit.count" or "retired_pages.dbe"
The number of GPU device memory pages that have been retired due to a double bit ECC error.

"retired_pages.pending"
Checks if any GPU device memory pages are pending retirement on the next reboot. Pages that are pending retirement can still be allocated, and may cause further reliability issues.

"temperature.gpu"
 Core GPU temperature. in degrees C.

"temperature.memory"
 HBM memory temperature. in degrees C.

"power.management"
A flag that indicates whether power management is enabled. Either "Supported" or "[Not Supported]". Requires Inforom PWR object version 3.0 or higher or Kepler device.

"power.draw"
The last measured power draw for the entire board, in watts. Only available if power management is supported. This reading is accurate to within +/- 5 watts.

"power.limit"
The software power limit in watts. Set by software like nvidia-smi. On Kepler devices Power Limit can be adjusted using [-pl | --power-limit=] switches.

"enforced.power.limit"
The power management algorithm's power ceiling, in watts. Total board power draw is manipulated by the power management algorithm such that it stays under this value. This value is the minimum of various power limiters.

"power.default_limit"
The default power management algorithm's power ceiling, in watts. Power Limit will be set back to Default Power Limit after driver unload.

"power.min_limit"
The minimum value in watts that power limit can be set to.

"power.max_limit"
The maximum value in watts that power limit can be set to.

"clocks.current.graphics" or "clocks.gr"
Current frequency of graphics (shader) clock.

"clocks.current.sm" or "clocks.sm"
Current frequency of SM (Streaming Multiprocessor) clock.

"clocks.current.memory" or "clocks.mem"
Current frequency of memory clock.

"clocks.current.video" or "clocks.video"
Current frequency of video encoder/decoder clock.

Section about clocks.applications properties
User specified frequency at which applications will be running at. Can be changed with [-ac | --applications-clocks] switches.

"clocks.applications.graphics" or "clocks.applications.gr"
User specified frequency of graphics (shader) clock.

"clocks.applications.memory" or "clocks.applications.mem"
User specified frequency of memory clock.

Section about clocks.default_applications properties
Default frequency at which applications will be running at. Application clocks can be changed with [-ac | --applications-clocks] switches. Application clocks can be set to default using [-rac | --reset-applications-clocks] switches.

"clocks.default_applications.graphics" or "clocks.default_applications.gr"
Default frequency of applications graphics (shader) clock.

"clocks.default_applications.memory" or "clocks.default_applications.mem"
Default frequency of applications memory clock.

Section about clocks.max properties
Maximum frequency at which parts of the GPU are design to run.

"clocks.max.graphics" or "clocks.max.gr"
Maximum frequency of graphics (shader) clock.

"clocks.max.sm" or "clocks.max.sm"
Maximum frequency of SM (Streaming Multiprocessor) clock.

"clocks.max.memory" or "clocks.max.mem"
Maximum frequency of memory clock.

Section about mig.mode properties
A flag that indicates whether MIG mode is enabled. May be either "Enabled" or "Disabled". Changes to MIG mode require a GPU reset.

"mig.mode.current"
The MIG mode that the GPU is currently operating under.

"mig.mode.pending"
The MIG mode that the GPU will operate under after reset.
EOF
  ;;

  *)
  echo timestamp, driver_version, count, name, serial, uuid, pci.bus_id, pci.domain, pci.bus, pci.device, pci.device_id, pci.sub_device_id, pcie.link.gen.current, pcie.link.gen.max, pcie.link.width.current, pcie.link.width.max, index, display_mode, display_active, persistence_mode, accounting.mode, accounting.buffer_size, driver_model.current, driver_model.pending, vbios_version, inforom.img, inforom.oem, inforom.ecc, inforom.pwr, gom.current, gom.pending, fan.speed [%], pstate, clocks_throttle_reasons.supported, clocks_throttle_reasons.active, clocks_throttle_reasons.gpu_idle, clocks_throttle_reasons.applications_clocks_setting, clocks_throttle_reasons.sw_power_cap, clocks_throttle_reasons.hw_slowdown, clocks_throttle_reasons.hw_thermal_slowdown, clocks_throttle_reasons.hw_power_brake_slowdown, clocks_throttle_reasons.sw_thermal_slowdown, clocks_throttle_reasons.sync_boost, memory.total [MiB], memory.used [MiB], memory.free [MiB], compute_mode, utilization.gpu [%], utilization.memory [%], encoder.stats.sessionCount, encoder.stats.averageFps, encoder.stats.averageLatency, ecc.mode.current, ecc.mode.pending, ecc.errors.corrected.volatile.device_memory, ecc.errors.corrected.volatile.dram, ecc.errors.corrected.volatile.register_file, ecc.errors.corrected.volatile.l1_cache, ecc.errors.corrected.volatile.l2_cache, ecc.errors.corrected.volatile.texture_memory, ecc.errors.corrected.volatile.cbu, ecc.errors.corrected.volatile.sram, ecc.errors.corrected.volatile.total, ecc.errors.corrected.aggregate.device_memory, ecc.errors.corrected.aggregate.dram, ecc.errors.corrected.aggregate.register_file, ecc.errors.corrected.aggregate.l1_cache, ecc.errors.corrected.aggregate.l2_cache, ecc.errors.corrected.aggregate.texture_memory, ecc.errors.corrected.aggregate.cbu, ecc.errors.corrected.aggregate.sram, ecc.errors.corrected.aggregate.total, ecc.errors.uncorrected.volatile.device_memory, ecc.errors.uncorrected.volatile.dram, ecc.errors.uncorrected.volatile.register_file, ecc.errors.uncorrected.volatile.l1_cache, ecc.errors.uncorrected.volatile.l2_cache, ecc.errors.uncorrected.volatile.texture_memory, ecc.errors.uncorrected.volatile.cbu, ecc.errors.uncorrected.volatile.sram, ecc.errors.uncorrected.volatile.total, ecc.errors.uncorrected.aggregate.device_memory, ecc.errors.uncorrected.aggregate.dram, ecc.errors.uncorrected.aggregate.register_file, ecc.errors.uncorrected.aggregate.l1_cache, ecc.errors.uncorrected.aggregate.l2_cache, ecc.errors.uncorrected.aggregate.texture_memory, ecc.errors.uncorrected.aggregate.cbu, ecc.errors.uncorrected.aggregate.sram, ecc.errors.uncorrected.aggregate.total, retired_pages.single_bit_ecc.count, retired_pages.double_bit.count, retired_pages.pending, temperature.gpu, temperature.memory, power.management, power.draw [W], power.limit [W], enforced.power.limit [W], power.default_limit [W], power.min_limit [W], power.max_limit [W], clocks.current.graphics [MHz], clocks.current.sm [MHz], clocks.current.memory [MHz], clocks.current.video [MHz], clocks.applications.graphics [MHz], clocks.applications.memory [MHz], clocks.default_applications.graphics [MHz], clocks.default_applications.memory [MHz], clocks.max.graphics [MHz], clocks.max.sm [MHz], clocks.max.memory [MHz], mig.mode.current, mig.mode.pending
  echo 2021/06/10 00:55:54.492, 465.27, 1, NVIDIA GeForce RTX 2080 SUPER, [N/A], GPU-df6e7a7c-7314-46f8-abc4-b88b36dcf3aa, 00000000:0C:00.0, 0x0000, 0x0C, 0x00, 0x1E8110DE, 0x40051458, 1, 3, 16, 16, 0, Enabled, Enabled, Disabled, Disabled, 4000, [N/A], [N/A], 90.04.7A.40.73, G001.0000.02.04, 1.1, [N/A], [N/A], [N/A], [N/A], 38 %, P8, 0x00000000000001FF, 0x0000000000000001, Active, Not Active, Not Active, Not Active, Not Active, Not Active, Not Active, Not Active, 7979 MiB, 344 MiB, 7635 MiB, Default, 0 %, 10 %, 0, 0, 0, [N/A], [N/A], [N/A], [N/A], [N/A], [N/A], [N/A], [N/A], [N/A], [N/A], [N/A], [N/A], [N/A], [N/A], [N/A], [N/A], [N/A], [N/A], [N/A], [N/A], [N/A], [N/A], [N/A], [N/A], [N/A], [N/A], [N/A], [N/A], [N/A], [N/A], [N/A], [N/A], [N/A], [N/A], [N/A], [N/A], [N/A], [N/A], [N/A], [N/A], [N/A], 32, N/A, Enabled, 30.37 W, 250.00 W, 250.00 W, 250.00 W, 105.00 W, 350.00 W, 300 MHz, 300 MHz, 405 MHz, 540 MHz, [N/A], [N/A], [N/A], [N/A], 2145 MHz, 2145 MHz, 7751 MHz, [N/A], [N/A]
  ;;

esac
