use lazy_static::lazy_static;
use std::collections::HashMap;
use std::sync::RwLock;

pub struct DiskConfig {
    pub read_speed: f64,  // MB/s
    pub write_speed: f64, // MB/s
}

lazy_static! {
    pub static ref DISK_CONFIGS: RwLock<HashMap<&'static str, DiskConfig>> = RwLock::new({
        let mut configs = HashMap::new();
        configs.insert(
            "sata_hdd",
            DiskConfig {
                read_speed: 120.0,
                write_speed: 100.0,
            },
        );
        configs.insert(
            "sata_ssd",
            DiskConfig {
                read_speed: 300.0,
                write_speed: 250.0,
            },
        );
        configs.insert(
            "nvme",
            DiskConfig {
                read_speed: 1500.0,
                write_speed: 1200.0,
            },
        );
        configs
    });
}

pub fn get_disk_configs() -> &'static RwLock<HashMap<&'static str, DiskConfig>> {
    &DISK_CONFIGS
}
