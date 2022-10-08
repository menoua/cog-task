use std::fmt::{Debug, Formatter};
use sysinfo::{CpuExt, SystemExt};

pub struct SystemInfo {
    system: String,
    cpu_brand: String,
    cpu_cores: usize,
    memory_total: u64,
    memory_available: u64,
    pub(crate) renderer: String,
    pub(crate) hw_acceleration: String,
}

impl SystemInfo {
    pub fn new() -> Self {
        let mut sys = sysinfo::System::new_all();
        sys.refresh_memory();
        sys.refresh_cpu();

        Self {
            system: format!(
                "{} ({}) ({})",
                sys.name().unwrap_or("".to_owned()).trim(),
                sys.long_os_version().unwrap_or("?".to_owned()).trim(),
                sys.kernel_version().unwrap_or("?".to_owned()).trim(),
            ),
            cpu_brand: format!("{}", sys.global_cpu_info().brand()),
            cpu_cores: sys.cpus().len(),
            memory_total: sys.total_memory(),
            memory_available: sys.available_memory(),
            renderer: "(?)".to_owned(),
            hw_acceleration: "(?)".to_owned(),
        }
    }
}

impl Debug for SystemInfo {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "System: {}\nCPU: {} (x{})\nMemory (GiB): {:.1} (available: {:.1})\n\
            Renderer: {}\nHW acceleration: {}",
            self.system,
            self.cpu_brand,
            self.cpu_cores,
            self.memory_total as f64 / 2_f64.powi(30),
            self.memory_available as f64 / 2_f64.powi(30),
            self.renderer,
            self.hw_acceleration
        )
    }
}
