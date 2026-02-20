mod protocol;

use bytes::Buf;
use configurable::{Configurable, configurable_component};
use event::{Metric, tags};
use framework::config::{OutputType, SourceConfig, SourceContext};
use framework::config::{default_interval, default_true};
use framework::http::{Auth, HttpClient, HttpError};
use framework::{Pipeline, ShutdownSignal, Source};
use http_body_util::{BodyExt, Full};
use serde::{Deserialize, Serialize};
use std::hash::{Hash, Hasher};
use std::ops::{Add, Sub};
use std::sync::Arc;
use std::time::Duration;
use tokio::task::JoinSet;
use tokio::time::Instant;
use url::Url;

use protocol::chassis::{Chassis, Power, Thermal};
use protocol::system::{
    Memory, MemoryMetrics, NetworkInterface, SimpleStorage, Storage, StorageDevice, System,
};
use protocol::{List, Root};

#[derive(Configurable, Clone, Debug, Deserialize, Serialize)]
struct CollectConfig {
    // Chassis
    #[serde(default = "default_true")]
    thermal: bool,
    #[serde(default = "default_true")]
    power: bool,

    // System
    #[serde(default = "default_true")]
    memory: bool,
    #[serde(default)]
    network: bool,
    #[serde(default = "default_true")]
    simple_storage: bool,
    #[serde(default = "default_true")]
    storage: bool,
}

impl Default for CollectConfig {
    fn default() -> Self {
        CollectConfig {
            thermal: true,
            power: true,
            storage: true,
            memory: true,
            network: true,
            simple_storage: true,
        }
    }
}

#[configurable_component(source, name = "redfish")]
struct Config {
    #[configurable(example = "http://localhost:8000")]
    targets: Vec<Url>,

    auth: Option<Auth>,

    /// The interval between fetch config.
    #[serde(default = "default_interval", with = "humanize::duration::serde")]
    interval: Duration,

    /// Configure which resource metric to collect
    #[serde(default)]
    collector: CollectConfig,
}

#[async_trait::async_trait]
#[typetag::serde(name = "redfish")]
impl SourceConfig for Config {
    async fn build(&self, cx: SourceContext) -> crate::Result<Source> {
        let client = HttpClient::new(None, &cx.proxy)?;

        Ok(Box::pin(run(
            client,
            self.targets.clone(),
            self.auth.clone(),
            self.interval,
            self.collector.clone(),
            cx.output,
            cx.shutdown,
        )))
    }

    fn outputs(&self) -> Vec<OutputType> {
        vec![OutputType::metric()]
    }
}

async fn run(
    client: HttpClient,
    targets: Vec<Url>,
    auth: Option<Auth>,
    interval: Duration,
    config: CollectConfig,
    output: Pipeline,
    shutdown: ShutdownSignal,
) -> Result<(), ()> {
    let mut tasks = JoinSet::new();

    for target in targets {
        let client = Client {
            client: client.clone(),
            target: target.to_string(),
            auth: auth.clone(),
        };

        tasks.spawn(scrape_loop(
            client,
            interval,
            config.clone(),
            output.clone(),
            shutdown.clone(),
        ));
    }

    while tasks.join_next().await.is_some() {}

    Ok(())
}

#[inline]
fn target_hash<H: Hash>(target: H) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::default();
    target.hash(&mut hasher);
    hasher.finish()
}

async fn scrape_loop(
    client: Client,
    interval: Duration,
    config: CollectConfig,
    mut output: Pipeline,
    mut shutdown: ShutdownSignal,
) {
    let client = Arc::new(client);
    let ttl = Duration::from_secs(10 * 60);

    let last_cached = Instant::now().sub(2 * ttl); // make sure metadata will be updated
    let mut chassis = vec![];
    let mut systems = vec![];

    // get hash and set jitter
    let offset = target_hash(&client.target) % interval.as_nanos() as u64;
    let offset = Duration::from_nanos(offset);
    let mut ticker = tokio::time::interval_at(Instant::now().add(offset), interval);

    loop {
        tokio::select! {
            _ = ticker.tick() => {}
            _ = &mut shutdown => break,
        }

        let start = Instant::now();
        if start.sub(last_cached) > ttl {
            // update
            match fetch_all_endpoints(&client, &config).await {
                Ok((c, s)) => {
                    chassis = c;
                    systems = s;
                }
                Err(err) => {
                    warn!(
                        message = "fetch resource endpoints failed",
                        %err,
                        target = &client.target,
                    );

                    continue;
                }
            };
        }

        let result = gather(Arc::clone(&client), &config, &chassis, &systems).await;
        let elapsed = start.elapsed();
        let up = result.is_ok();

        let mut metrics = result.unwrap_or_else(|err| {
            warn!(
                message = "scrape redfish metrics failed",
                target = &client.target,
                %err,
            );
            Vec::with_capacity(2)
        });

        metrics.extend([
            Metric::gauge("redfish_up", "", up),
            Metric::gauge("redfish_scrape_duration_seconds", "", elapsed),
        ]);

        // set instance
        metrics.iter_mut().for_each(|m| {
            m.tags.insert("instance", client.target.clone());
        });

        if let Err(_err) = output.send(metrics).await {
            warn!(message = "send metrics failed", target = &client.target);

            break;
        }
    }
}

#[derive(Debug)]
struct SystemResources {
    system: System,

    // logs: Vec<String>,
    memories: Vec<String>,
    networks: Vec<String>,
    simple_storages: Vec<String>,
    storages: Vec<String>,
}

#[derive(Debug)]
struct ChassisResources {
    chassis: Chassis,

    thermal: Option<String>,
    power: Option<String>,
}

struct Client {
    client: HttpClient,
    target: String,
    auth: Option<Auth>,
}

impl Client {
    async fn fetch<T: for<'a> Deserialize<'a>>(&self, path: &str) -> Result<T, HttpError> {
        let mut req = http::Request::get(format!("{}{}", self.target, path))
            .header("accept", "application/json")
            .body(Full::default())?;
        if let Some(auth) = &self.auth {
            auth.apply(&mut req);
        }

        let resp = self.client.send(req).await?;

        let (parts, incoming) = resp.into_parts();
        if !parts.status.is_success() {
            return Err(HttpError::UnexpectedStatus(parts.status));
        }

        let body = incoming.collect().await?.to_bytes();

        serde_json::from_reader(body.reader()).map_err(Into::into)
    }
}

async fn gather(
    client: Arc<Client>,
    config: &CollectConfig,
    chassis: &[ChassisResources],
    systems: &[SystemResources],
) -> crate::Result<Vec<Metric>> {
    let mut metrics = vec![];
    let mut tasks = JoinSet::new();

    for SystemResources {
        system,
        memories,
        networks,
        simple_storages,
        storages,
    } in systems
    {
        metrics.push(Metric::gauge_with_tags(
            "redfish_system_info",
            "Information of the system",
            1,
            tags!(
                "system" => system.id.clone(),
                "name" => system.name.clone(),
                // extra info
                "system_type" => system.system_type.clone(),
                "manufacturer" => system.manufacturer.clone(),
                "model" => system.model.clone(),
                "sku" => system.sku.clone(),
                "serial_number" => system.serial_number.clone(),
                "part_number" => system.part_number.clone(),
                "bios_version" => system.bios_version.clone(),
                "uuid" => system.uuid.clone(),
                "hostname" => system.host_name.clone(),
                "asset_tag" => system.asset_tag.clone(),
            ),
        ));

        if let Some(status) = &system.status {
            metrics.push(Metric::gauge_with_tags(
                "redfish_system_power_state",
                "Power state of the system",
                matches!(&system.power_state, Some(state) if state == "On"),
                tags!("system" => system.id.clone(), "name" => system.name.clone()),
            ));

            if let Some(health) = &status.health {
                metrics.push(Metric::gauge_with_tags(
                    "redfish_system_health",
                    "Health status of the system",
                    health == "OK",
                    tags!("system" => system.id.clone(), "name" => system.name.clone()),
                ));
            }
        }

        if config.memory {
            for path in memories {
                let client = Arc::clone(&client);

                tasks.spawn(collect_memory(client, path.to_string(), system.id.clone()));
            }
        }

        if config.network {
            for path in networks {
                let client = Arc::clone(&client);

                tasks.spawn(collect_network(client, path.to_string(), system.id.clone()));
            }
        }

        if config.simple_storage {
            for path in simple_storages {
                let client = Arc::clone(&client);

                tasks.spawn(collect_simple_storage(
                    client,
                    path.to_string(),
                    system.id.clone(),
                ));
            }
        }

        if config.storage {
            for path in storages {
                let client = Arc::clone(&client);

                tasks.spawn(collect_storage(client, path.to_string(), system.id.clone()));
            }
        }
    }

    for ChassisResources {
        chassis,
        thermal,
        power,
    } in chassis
    {
        metrics.extend([
            Metric::gauge_with_tags(
                "redfish_chassis_info",
                "The information of the chassis",
                1,
                tags!(
                    "id" => chassis.id.clone(),
                    "name" => chassis.name.clone(),
                    "chassis_type" => chassis.chassis_type.clone().unwrap_or_default(),
                    "asset_tag" => chassis.asset_tag.clone().unwrap_or_default(),
                    "manufacturer" => chassis.manufacturer.clone().unwrap_or_default(),
                    "model" => chassis.model.clone().unwrap_or_default(),
                    "sku" => chassis.sku.clone().unwrap_or_default(),
                    "serial_number" => chassis.serial_number.clone().unwrap_or_default(),
                    "part_number" => chassis.part_number.clone().unwrap_or_default(),
                ),
            ),
            Metric::gauge_with_tags(
                "redfish_chassis_power_state",
                "Power state of the chassis",
                matches!(&chassis.power_state, Some(state) if state == "On"),
                tags!(
                    "id" => chassis.id.clone(),
                    "name" => chassis.name.clone(),
                ),
            ),
        ]);

        if let Some(status) = &chassis.status {
            metrics.push(Metric::gauge_with_tags(
                "redfish_chassis_state",
                "state of the chassis",
                status.state == "Enabled",
                tags!(
                    "id" => chassis.id.clone(),
                    "name" => chassis.name.clone(),
                ),
            ));

            if let Some(health) = &status.health {
                metrics.push(Metric::gauge_with_tags(
                    "redfish_chassis_health",
                    "Health status of the chassis",
                    health == "OK",
                    tags!(
                        "id" => chassis.id.clone(),
                        "name" => chassis.name.clone(),
                    ),
                ))
            }
        }

        if config.thermal
            && let Some(path) = thermal
        {
            let client = Arc::clone(&client);

            tasks.spawn(collect_thermal(
                client,
                path.to_string(),
                chassis.id.clone(),
            ));
        }

        if config.power
            && let Some(path) = power
        {
            let client = Arc::clone(&client);

            tasks.spawn(collect_power(client, path.to_string(), chassis.id.clone()));
        }
    }

    while let Some(Ok(result)) = tasks.join_next().await {
        match result {
            Ok(partial) => metrics.extend(partial),
            Err(err) => {
                warn!(
                    message = "collect redfish metrics failed",
                    %err,
                    target = &client.target
                )
            }
        }
    }

    Ok(metrics)
}

async fn collect_memory(
    client: Arc<Client>,
    path: String,
    system: String,
) -> crate::Result<Vec<Metric>> {
    let memory = client.fetch::<Memory>(&path).await?;

    let mut info_tags = tags!(
        // system id
        "system" => system.clone(),
        // misc
        "id" => &memory.id,
        "name" => &memory.name,
        "rank_count" => memory.rank_count,
        "capacity_mib" => memory.capacity_mib,
        "data_width_bits" => memory.data_width_bits,
        "bus_width_bits" => memory.bus_width_bits,
        "error_correction" => memory.error_correction,
        "memory_type" => memory.memory_type,
        "memory_device_type" => memory.memory_device_type,
        "base_module_type" => memory.base_module_type,
    );
    if let Some(memory_location) = memory.memory_location {
        info_tags.insert("socket", memory_location.socket);
        info_tags.insert("memory_controller", memory_location.memory_controller);
        info_tags.insert("channel", memory_location.channel);
        info_tags.insert("slot", memory_location.slot);
    }

    let mut metrics = vec![
        Metric::gauge_with_tags(
            "redfish_system_memory_info",
            "Memory information of the system",
            1,
            info_tags,
        ),
        Metric::gauge_with_tags(
            "redfish_system_memory_state",
            "Memory state, 1 for enabled",
            memory.status.state == "Enabled",
            tags!(
                "system" => system.clone(),

                "id" => &memory.id,
                "name" => &memory.name,
            ),
        ),
        Metric::gauge_with_tags(
            "redfish_system_memory_capacity",
            "Memory capacity in bytes",
            memory.capacity_mib * 1024 * 1024,
            tags!(
                "system" => system.clone(),

                "id" => &memory.id,
                "name" => &memory.name,
            ),
        ),
        Metric::gauge_with_tags(
            "redfish_system_memory_operating_speed",
            "Memory speed in MHz",
            memory.operating_speed_mhz,
            tags!(
                "system" => system.clone(),

                "id" => &memory.id,
                "name" => &memory.name,
            ),
        ),
    ];

    if let Some(health) = &memory.status.health {
        metrics.push(Metric::gauge_with_tags(
            "redfish_system_memory_health",
            "Memory health, 1 for ok",
            health == "OK",
            tags!(
                "system" => system.clone(),

                "id" => &memory.id,
                "name" => &memory.name,
            ),
        ));
    }

    if let Some(link) = memory.metrics {
        let memory_metrics = client.fetch::<MemoryMetrics>(link.path()).await?;

        metrics.extend([
            Metric::gauge_with_tags(
                "redfish_system_memory_data_loss",
                "Memory data loss",
                memory_metrics.health_data.data_loss_detected,
                tags!(
                    "system" => system.clone(),

                    "id" => &memory.id,
                    "name" => &memory.name,
                ),
            ),
            Metric::gauge_with_tags(
                "redfish_system_memory_correctable_ecc_error",
                "Correctable error threshold crossing alarm trip",
                memory_metrics.health_data.alarm_trips.correctable_ecc_error,
                tags!(
                    "system" => system.clone(),

                    "id" => &memory.id,
                    "name" => &memory.name,
                ),
            ),
            Metric::gauge_with_tags(
                "redfish_system_memory_uncorrectable_ecc_error",
                "Uncorrectable error threshold crossing alarm trip",
                memory_metrics
                    .health_data
                    .alarm_trips
                    .uncorrectable_ecc_error,
                tags!(
                    "system" => system,

                    "id" => memory.id,
                    "name" => memory.name,
                ),
            ),
        ]);
    }

    Ok(metrics)
}

async fn collect_network(
    client: Arc<Client>,
    path: String,
    system: String,
) -> crate::Result<Vec<Metric>> {
    let network = client.fetch::<NetworkInterface>(&path).await?;

    let mut metrics = vec![
        Metric::gauge_with_tags(
            "redfish_system_network_interface_info",
            "Network interface info",
            1,
            tags!(
                "system" => system.clone(),
                "id" => &network.id,
                "name" => &network.name,
            ),
        ),
        Metric::gauge_with_tags(
            "redfish_system_network_interface_state",
            "Network interface state",
            network.status.state == "Enabled",
            tags!(
                "system" => system.clone(),
                "id" => &network.id,
                "name" => &network.name,
            ),
        ),
    ];

    if let Some(health) = &network.status.health {
        metrics.push(Metric::gauge_with_tags(
            "redfish_system_network_interface_health",
            "Network interface health",
            health == "OK",
            tags!(
                "system" => system.clone(),
                "id" => network.id,
                "name" => network.name,
            ),
        ));
    }

    Ok(metrics)
}

async fn collect_storage(
    client: Arc<Client>,
    path: String,
    system: String,
) -> crate::Result<Vec<Metric>> {
    let storage = client.fetch::<Storage>(&path).await?;

    let mut metrics = vec![];
    for controller in storage.storage_controllers {
        metrics.extend([
            Metric::gauge_with_tags(
                "redfish_system_storage_controller_info",
                "Storage controller info",
                1,
                tags!(
                    "system" => system.clone(),

                    "storage" => &storage.id,

                    "member_id" => &controller.member_id,
                    "controller" => &controller.name,

                    "manufacturer" => controller.manufacturer.unwrap_or_default(),
                    "model" => controller.model.unwrap_or_default(),
                    "serial_number" => controller.serial_number.unwrap_or_default(),
                    "part_number" => controller.part_number.unwrap_or_default(),
                    "speed_gbps" => controller.speed_gbps.unwrap_or_default(),
                    "firmware_version" => controller.firmware_version.unwrap_or_default(),
                ),
            ),
            Metric::gauge_with_tags(
                "redfish_system_storage_controller_state",
                "Storage controller state",
                controller.status.state == "Enabled",
                tags!(
                    "system" => system.clone(),

                    "storage" => &storage.id,

                    "member_id" => controller.member_id,
                    "controller" => controller.name,
                ),
            ),
        ]);
    }

    for link in storage.drives {
        let drive = client.fetch::<StorageDevice>(link.path()).await?;

        if drive.status.state == "Absent" {
            continue;
        }

        metrics.push(Metric::gauge_with_tags(
            "redfish_system_storage_drive_info",
            "Storage drive info",
            1,
            tags!(
                "system" => system.clone(),

                "storage" => &storage.id,

                "id" => &drive.id,
                "name" => &drive.name,
                "model" => &drive.model.unwrap_or_default(),
                "revision" => &drive.revision.unwrap_or_default(),
                "protocol" => &drive.protocol.unwrap_or_default(),
                "media_type" => drive.media_type.unwrap_or_default(),
                "manufacturer" => drive.manufacturer.unwrap_or_default(),
                "serial_number" => drive.serial_number.unwrap_or_default(),
                "part_number" => drive.part_number.unwrap_or_default(),
            ),
        ));

        if drive.status.state == "Absent" {
            continue;
        }

        if let Some(value) = drive.capacity_bytes {
            metrics.push(Metric::gauge_with_tags(
                "redfish_system_storage_drive_capacity_bytes",
                "The capacity of storage drive in bytes",
                value,
                tags!(
                    "system" => system.clone(),
                    "storage" => &storage.id,

                    "drive" => &drive.id,
                ),
            ));
        }

        if let Some(value) = drive.capable_speed_gbs {
            metrics.push(Metric::gauge_with_tags(
                "redfish_system_storage_drive_capable_speed_gbs",
                "The capable of speed of this drive",
                value,
                tags!(
                    "system" => system.clone(),
                    "storage" => &storage.id,

                    "drive" => &drive.id,
                ),
            ));
        }

        if let Some(value) = drive.negotiated_speed_gbs {
            metrics.push(Metric::gauge_with_tags(
                "redfish_system_storage_drive_negotiated_speed_gbs",
                "The negotiated speed of this drive",
                value,
                tags!(
                    "system" => system.clone(),
                    "storage" => &storage.id,

                    "drive" => &drive.id,
                ),
            ));
        }
    }

    Ok(metrics)
}

async fn collect_simple_storage(
    client: Arc<Client>,
    path: String,
    system: String,
) -> crate::Result<Vec<Metric>> {
    let simple_storage = client.fetch::<SimpleStorage>(&path).await?;

    let mut metrics = vec![
        Metric::gauge_with_tags(
            "redfish_system_simple_storage_info",
            "Information about this simple storage",
            1,
            tags!(
                "system" => system.clone(),

                "id" => &simple_storage.id,
                "name" => &simple_storage.name,
            ),
        ),
        Metric::gauge_with_tags(
            "redfish_system_simple_storage_state",
            "Simple storage state",
            simple_storage.status.state == "Enabled",
            tags!(
                "system" => system.clone(),

                "id" => &simple_storage.id,
                "name" => &simple_storage.name,
            ),
        ),
    ];

    if simple_storage.status.state == "Absent" {
        return Ok(metrics);
    }

    metrics.push(Metric::gauge_with_tags(
        "redfish_system_simple_storage_health",
        "",
        simple_storage
            .status
            .health
            .map(|s| s == "OK")
            .unwrap_or(false),
        tags!(
            "system" => system.clone(),

            "id" => &simple_storage.id,
            "name" => &simple_storage.name,
        ),
    ));

    for (index, device) in simple_storage.devices.into_iter().enumerate() {
        metrics.push(Metric::gauge_with_tags(
            "redfish_system_simple_storage_device_info",
            "Device info",
            1,
            tags!(
                "system" => system.clone(),

                "index" => index,
                "name" => &device.name,

                "manufacturer" => device.manufacturer.clone().unwrap_or_default(),
                "model" => device.model.clone().unwrap_or_default(),
            ),
        ));

        metrics.push(Metric::gauge_with_tags(
            "redfish_system_simple_storage_device_state",
            "simple storage device state",
            device.status.state == "Enabled",
            tags!(
                "system" => system.clone(),

                "name" => &device.name,
                "index" => index,
            ),
        ));

        if device.status.state == "Absent" {
            continue;
        }

        if let Some(health) = device.status.health {
            metrics.push(Metric::gauge_with_tags(
                "redfish_system_simple_storage_device_health",
                "",
                health == "OK",
                tags!(
                    "system" => system.clone(),

                    "name" => &device.name,
                    "index" => index,
                ),
            ));
        }

        if let Some(value) = device.capacity_bytes {
            metrics.push(Metric::gauge_with_tags(
                "redfish_system_simple_storage_drive_capacity_bytes",
                "The simple storage device size, in bytes, of the storage device.",
                value,
                tags!(
                    "system" => system.clone(),
                    "name" => &device.name,
                ),
            ));
        }
    }

    Ok(metrics)
}

async fn collect_thermal(
    client: Arc<Client>,
    path: String,
    chassis: String,
) -> crate::Result<Vec<Metric>> {
    let thermal = client.fetch::<Thermal>(&path).await?;

    let mut metrics = vec![];
    for fan in thermal.fans {
        metrics.extend([
            Metric::gauge_with_tags(
                "redfish_system_thermal_fan_info",
                "",
                1,
                tags! {
                    "chassis" => chassis.clone(),
                    "name" => fan.name.clone(),
                    "member_id" => fan.member_id.clone(),

                    "manufacturer" => fan.manufacturer.clone().unwrap_or_default(),
                    "hot_pluggable" => fan.hot_pluggable.unwrap_or_default(),
                },
            ),
            Metric::gauge_with_tags(
                "redfish_chassis_fan_state",
                "fan status",
                fan.status.state == "Enabled",
                tags!(
                    "chassis" => chassis.clone(),
                    "name" => fan.name.clone(),
                    "member_id" => fan.member_id.clone(),
                ),
            ),
        ]);

        if fan.status.state == "Absent" {
            continue;
        }

        if let Some(health) = fan.status.health {
            metrics.push(Metric::gauge_with_tags(
                "redfish_chassis_fan_health",
                "fan health",
                health == "OK",
                tags!(
                    "chassis" => chassis.clone(),
                    "name" => fan.name.clone(),
                    "member_id" => fan.member_id.clone(),
                ),
            ));
        }

        if let Some(value) = fan.reading {
            metrics.push(Metric::gauge_with_tags(
                "redfish_chassis_fan_speed",
                "fan speed in RPM",
                value,
                tags!(
                    "chassis" => chassis.clone(),
                    "name" => fan.name.clone(),
                    "member_id" => fan.member_id.clone(),
                ),
            ));
        }

        if let Some(value) = fan.lower_threshold_critical {
            metrics.push(Metric::gauge_with_tags(
                "redfish_chassis_fan_lower_threshold_critical",
                "",
                value,
                tags!(
                    "chassis" => chassis.clone(),
                    "name" => fan.name.clone(),
                    "member_id" => fan.member_id.clone(),
                ),
            ))
        }

        if let Some(value) = fan.lower_threshold_fatal {
            metrics.push(Metric::gauge_with_tags(
                "redfish_chassis_fan_lower_threshold_fatal",
                "",
                value,
                tags!(
                    "chassis" => chassis.clone(),
                    "name" => fan.name.clone(),
                    "member_id" => fan.member_id.clone(),
                ),
            ));
        }

        if let Some(value) = fan.lower_threshold_non_critical {
            metrics.push(Metric::gauge_with_tags(
                "redfish_chassis_fan_lower_threshold_non_critical",
                "",
                value,
                tags!(
                    "chassis" => chassis.clone(),
                    "name" => fan.name.clone(),
                    "member_id" => fan.member_id.clone(),
                ),
            ));
        }
    }

    for temp in thermal.temperatures {
        if let Some(health) = temp.status.health {
            metrics.push(Metric::gauge_with_tags(
                "redfish_chassis_temperature_health",
                "Temperature sensor health state",
                health == "OK",
                tags!(
                    "chassis" => chassis.clone(),

                    "name" => temp.name.clone(),
                    "member_id" => temp.member_id.clone(),
                ),
            ));
        }

        metrics.extend([Metric::gauge_with_tags(
            "redfish_chassis_temperature_state",
            "Temperature sensor state",
            temp.status.state == "Enabled",
            tags!(
                "chassis" => chassis.clone(),

                "name" => temp.name.clone(),
                "member_id" => temp.member_id.clone(),
            ),
        )]);

        if temp.status.state == "Absent" {
            continue;
        }

        if let Some(value) = temp.upper_threshold_non_critical {
            metrics.push(Metric::gauge_with_tags(
                "redfish_chassis_temperature_upper_threshold_non_critical",
                "",
                value,
                tags!(
                    "chassis" => chassis.clone(),

                    "name" => temp.name.clone(),
                    "member_id" => temp.member_id.clone(),
                ),
            ));
        }

        if let Some(value) = temp.upper_threshold_critical {
            metrics.push(Metric::gauge_with_tags(
                "redfish_chassis_temperature_upper_threshold_critical",
                "",
                value,
                tags!(
                    "chassis" => chassis.clone(),

                    "name" => temp.name.clone(),
                    "member_id" => temp.member_id.clone(),
                ),
            ));
        }

        if let Some(value) = temp.upper_threshold_fatal {
            metrics.push(Metric::gauge_with_tags(
                "redfish_chassis_temperature_upper_threshold_fatal",
                "",
                value,
                tags!(
                    "chassis" => chassis.clone(),

                    "name" => temp.name.clone(),
                    "member_id" => temp.member_id.clone(),
                ),
            ));
        }

        if let Some(value) = temp.reading_celsius {
            metrics.push(Metric::gauge_with_tags(
                "redfish_chassis_temperature_celsius",
                "Temperature the sensors reported",
                value,
                tags!(
                    "chassis" => chassis.clone(),

                    "name" => temp.name.clone(),
                    "member_id" => temp.member_id.clone(),
                ),
            ));
        }
    }

    Ok(metrics)
}

async fn collect_power(
    client: Arc<Client>,
    path: String,
    chassis: String,
) -> crate::Result<Vec<Metric>> {
    let power = client.fetch::<Power>(&path).await?;

    let mut metrics = vec![];
    for (index, power_supply) in power.power_supplies.into_iter().enumerate() {
        metrics.extend([
            Metric::gauge_with_tags(
                "redfish_chassis_power_supply_info",
                "Power supply information",
                1,
                tags!(
                    "chassis" => chassis.clone(),

                    "name" => power_supply.name.clone(),
                    "model" => power_supply.model.unwrap_or_default(),
                    "manufacturer" => power_supply.manufacturer.unwrap_or_default(),
                    "firmware_version" => power_supply.firmware_version.unwrap_or_default(),
                    "serial_number" => power_supply.serial_number.unwrap_or_default(),
                    "part_number" => power_supply.part_number.unwrap_or_default(),
                    "spare_part_number" => power_supply.spare_part_number.unwrap_or_default(),
                ),
            ),
            Metric::gauge_with_tags(
                "redfish_chassis_power_supply_state",
                "1(Enabled), 2(Disabled), 3(StandbyOffline), 4(StandbySpare), 5(InTest), 6(Starting), 7(Absent), 8(UnavailableOffline), 9(Deferring), 10(Quiesced), 11(Updating)",
                power_supply.status.state == "Enabled",
                tags!(
                    "chassis" => chassis.clone(),

                    "name" => power_supply.name.clone(),
                    "member_id" => power_supply.member_id.clone().unwrap_or_else(|| index.to_string()),
                )
            )
        ]);

        if power_supply.status.state == "Absent" {
            continue;
        }

        if let Some(health) = power_supply.status.health {
            metrics.push(Metric::gauge_with_tags(
                "redfish_chassis_power_supply_health",
                "power supply health of chassis",
                health == "OK",
                tags!(
                    "chassis" => chassis.clone(),
                    "name" => power_supply.name.clone(),
                    "member_id" => power_supply.member_id.clone().unwrap_or_else(|| index.to_string()),
                )
            ));
        }

        if let Some(value) = power_supply.power_input_watts {
            metrics.push(Metric::gauge_with_tags(
                "redfish_chassis_power_input_watts",
                "measured input power, in Watts, of power supply on this chassis",
                value,
                tags!(
                    "chassis" => chassis.clone(),
                    "name" => power_supply.name.clone(),
                    "member_id" => power_supply.member_id.clone().unwrap_or_else(|| index.to_string()),
                )
            ));
        }

        if let Some(value) = power_supply.power_output_watts {
            metrics.push(Metric::gauge_with_tags(
                "redfish_chassis_power_output_watts",
                "measured output power, in Watts, of power supply on this chassis",
                value,
                tags!(
                    "chassis" => chassis.clone(),
                    "name" => power_supply.name.clone(),
                    "member_id" => power_supply.member_id.clone().unwrap_or_else(|| index.to_string()),
                )
            ));
        }

        if let Some(value) = power_supply.power_capacity_watts {
            metrics.push(Metric::gauge_with_tags(
                "redfish_chassis_power_capacity_watts",
                "Power capacity in Watts of this chassis",
                value,
                tags!(
                    "chassis" => chassis.clone(),
                    "name" => power_supply.name.clone(),
                    "member_id" => power_supply.member_id.clone().unwrap_or_else(|| index.to_string()),
                )
            ));
        }

        if let Some(value) = power_supply.efficiency_percent {
            metrics.push(Metric::gauge_with_tags(
                "redfish_chassis_power_efficiency_percent",
                "rated efficiency, as a percentage, of the associated power supply",
                value,
                tags!(
                    "chassis" => chassis.clone(),
                    "name" => power_supply.name.clone(),
                    "member_id" => power_supply.member_id.clone().unwrap_or_else(|| index.to_string()),
                )
            ));
        }
    }

    for (index, voltage) in power.voltages.into_iter().enumerate() {
        let member_id = voltage.member_id.unwrap_or_else(|| index.to_string());

        if let Some(value) = voltage.reading_volts {
            metrics.push(Metric::gauge_with_tags(
                "redfish_chassis_power_voltage",
                "power supply input voltage",
                value,
                tags!(
                    "chassis" => chassis.clone(),

                    "member_id" => member_id.clone(),
                    "name" => voltage.name.clone(),
                ),
            ));
        }

        if let Some(value) = voltage.upper_threshold_non_critical {
            metrics.push(Metric::gauge_with_tags(
                "redfish_chassis_power_voltage_upper_threshold_non_critical",
                "power supply input voltage, above normal range",
                value,
                tags!(
                    "chassis" => chassis.clone(),

                    "member_id" => member_id.clone(),
                    "name" => voltage.name.clone(),
                ),
            ));
        }

        if let Some(value) = voltage.upper_threshold_critical {
            metrics.push(Metric::gauge_with_tags(
                "redfish_chassis_power_voltage_upper_threshold_critical",
                "power supply input voltage, critical but not yet fatal",
                value,
                tags!(
                    "chassis" => chassis.clone(),

                    "member_id" => member_id.clone(),
                    "name" => voltage.name.clone(),
                ),
            ))
        }

        if let Some(value) = voltage.upper_threshold_fatal {
            metrics.push(Metric::gauge_with_tags(
                "redfish_chassis_power_voltage_upper_threshold_fatal",
                "power supply input voltage, above normal range",
                value,
                tags!(
                    "chassis" => chassis.clone(),

                    "member_id" => member_id.clone(),
                    "name" => voltage.name.clone(),
                ),
            ))
        }

        if let Some(value) = voltage.lower_threshold_non_critical {
            metrics.push(Metric::gauge_with_tags(
                "redfish_chassis_power_voltage_lower_threshold_non_critical",
                "",
                value,
                tags!(
                    "chassis" => chassis.clone(),

                    "member_id" => member_id.clone(),
                    "name" => voltage.name.clone(),
                ),
            ))
        }

        if let Some(value) = voltage.lower_threshold_critical {
            metrics.push(Metric::gauge_with_tags(
                "redfish_chassis_power_voltage_lower_threshold_critical",
                "",
                value,
                tags!(
                    "chassis" => chassis.clone(),

                    "member_id" => member_id.clone(),
                    "name" => voltage.name.clone(),
                ),
            ))
        }

        if let Some(value) = voltage.lower_threshold_fatal {
            metrics.push(Metric::gauge_with_tags(
                "redfish_chassis_power_voltage_lower_threshold_fatal",
                "",
                value,
                tags!(
                    "chassis" => chassis.clone(),

                    "member_id" => member_id.clone(),
                    "name" => voltage.name.clone(),
                ),
            ))
        }
    }

    Ok(metrics)
}

async fn fetch_all_endpoints(
    client: &Client,
    config: &CollectConfig,
) -> crate::Result<(Vec<ChassisResources>, Vec<SystemResources>)> {
    let root = client.fetch::<Root>("/redfish/v1").await?;

    let systems = match root.systems {
        Some(systems) => {
            let list = client.fetch::<List>(systems.path()).await?;

            let mut resources = vec![];
            for link in list.members {
                let system = client.fetch::<System>(link.path()).await?;

                let memories = match (config.memory, &system.memory) {
                    (true, Some(link)) => client
                        .fetch::<List>(link.path())
                        .await?
                        .members
                        .into_iter()
                        .map(|link| link.path().to_string())
                        .collect::<Vec<_>>(),
                    _ => vec![],
                };
                let networks = match (config.network, &system.network_interfaces) {
                    (true, Some(link)) => client
                        .fetch::<List>(link.path())
                        .await?
                        .members
                        .into_iter()
                        .map(|link| link.path().to_string())
                        .collect::<Vec<_>>(),
                    _ => vec![],
                };
                let storages = match (config.storage, &system.storage) {
                    (true, Some(link)) => client
                        .fetch::<List>(link.path())
                        .await?
                        .members
                        .into_iter()
                        .map(|link| link.path().to_string())
                        .collect::<Vec<_>>(),
                    _ => vec![],
                };
                let simple_storages = match (config.simple_storage, &system.simple_storage) {
                    (true, Some(link)) => client
                        .fetch::<List>(link.path())
                        .await?
                        .members
                        .into_iter()
                        .map(|link| link.path().to_string())
                        .collect::<Vec<_>>(),
                    _ => vec![],
                };

                resources.push(SystemResources {
                    system,
                    memories,
                    networks,
                    simple_storages,
                    storages,
                })
            }

            resources
        }
        None => vec![],
    };

    let chassis = match root.chassis {
        Some(chassis) => {
            let list = client.fetch::<List>(chassis.path()).await?;

            let mut resources = vec![];
            for item in list.members {
                let chassis = client.fetch::<Chassis>(item.path()).await?;

                let thermal = chassis.thermal.as_ref().map(|link| link.path().to_string());
                let power = chassis.power.as_ref().map(|link| link.path().to_string());

                resources.push(ChassisResources {
                    chassis,
                    thermal,
                    power,
                })
            }

            resources
        }
        None => vec![],
    };

    Ok((chassis, systems))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_config() {
        crate::testing::generate_config::<Config>();
    }
}

#[cfg(all(test, feature = "redfish-integration-tests"))]
mod integration_tests {
    use super::*;

    use std::net::SocketAddr;
    use std::path::PathBuf;

    use bytes::Bytes;
    use framework::config::ProxyConfig;
    use http::{Method, Request, Response, StatusCode};
    use http_body_util::Full;
    use hyper::body::Incoming;
    use hyper::server::conn::http1;
    use hyper::service::service_fn;
    use hyper_util::rt::TokioIo;
    use tokio::net::TcpListener;

    /// HTTP status code 404
    fn not_found() -> Response<Full<Bytes>> {
        Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Full::new(Bytes::from_static(b"Not Found")))
            .unwrap()
    }

    async fn mock_server(root: PathBuf, listen: SocketAddr) {
        let listener = TcpListener::bind(listen).await.unwrap();

        loop {
            let (stream, _) = listener.accept().await.unwrap();

            let root = root.clone();
            tokio::spawn(async move {
                let service = service_fn(|req: Request<Incoming>| {
                    let root = root.clone();

                    async move {
                        if req.method() != Method::GET {
                            return Ok(not_found());
                        }

                        if let Some(path) = req.uri().path().strip_prefix("/redfish/v1") {
                            let path = match path.strip_prefix("/") {
                                Some(path) => root.join(path).join("index.json"),
                                None => root.join(path).join("index.json"),
                            };

                            println!("url: {}, read {:?}", req.uri().path(), path);

                            match std::fs::read(&path) {
                                Ok(data) => Response::builder()
                                    .status(StatusCode::OK)
                                    .header("Content-Type", "application/json")
                                    .body(Full::new(data.into())),
                                Err(err) => {
                                    println!("Failed to read file {path:?}, {err}");
                                    Response::builder()
                                        .status(StatusCode::INTERNAL_SERVER_ERROR)
                                        .body(Full::new(err.to_string().into()))
                                }
                            }
                        } else {
                            Ok(not_found())
                        }
                    }
                });

                if let Err(err) = http1::Builder::new()
                    .serve_connection(TokioIo::new(stream), service)
                    .await
                {
                    panic!("failed to serve connection: {err}")
                }
            });
        }
    }

    #[tokio::test]
    async fn dsp() {
        let root = PathBuf::from("tests/redfish/DSP2043_2024.3");
        let config = CollectConfig {
            thermal: true,
            power: true,
            memory: true,
            network: true,
            simple_storage: true,
            storage: true,
        };

        let dirs = std::fs::read_dir(&root).unwrap();

        let mut targets = vec![];
        for entry in dirs.flatten() {
            if entry.path().is_file() {
                continue;
            }

            if !entry
                .path()
                .file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with("public")
            {
                continue;
            }

            let addr = testify::next_addr();
            targets.push(addr);

            tokio::spawn(mock_server(entry.path(), addr));
        }

        tokio::time::sleep(Duration::from_secs(1)).await;

        for target in targets {
            let client = Client {
                client: HttpClient::new(None, &ProxyConfig::default()).unwrap(),
                target: format!("http://{target}"),
                auth: None,
            };

            let (chassis, systems) = fetch_all_endpoints(&client, &config).await.unwrap();

            let _metrics = gather(Arc::new(client), &config, &chassis, &systems)
                .await
                .unwrap();
        }
    }
}
