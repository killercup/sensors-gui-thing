use std::{io::Error as IoError, process::Command, sync::Arc};

#[derive(Clone, Debug, Default)]
pub struct Sensors {
    cpu: Option<Cpu>,
    graphics: Option<Graphics>,
    others: Vec<Generic>,
}

#[derive(Clone, Debug)]
struct Cpu {
    cores: Vec<CpuCore>,
    temp: CpuTemp,
}

#[derive(Clone, Debug)]
pub struct CpuCore {
    freq: f64,
}

#[derive(Clone, Debug)]
pub enum CpuTemp {
    Zen2 {
        voltage_core: f64,
        voltage_so_c: f64,
        current_core: f64,
        current_so_c: f64,
        temp_die: f64,
        temp_ctl: f64,
        temp_ccd1: f64,
        temp_ccd2: f64,
    },
}

#[derive(Clone, Debug)]
struct Graphics {}

#[derive(Clone, Debug)]
struct Generic {}

impl Sensors {
    pub fn fetch() -> Result<Self, Error> {
        let cmd = Command::new("sensors")
            .arg("-j")
            .output()
            .map_err(|e| Error::SpawnCommand {
                cmd: "sensors -j",
                source: Arc::new(e),
            })?;
        if !cmd.status.success() {
            return Err(Error::SensorsCmdFailed);
        }
        let sensor_data: json::Devices =
            serde_json::from_slice(&cmd.stdout).map_err(|e| Error::CannotParseSensorsJson {
                source: Arc::new(e),
            })?;
        let cpu_temp = if let Some((_ident, cpu)) =
            sensor_data.0.iter().find(|(k, v)| k.starts_with("k10temp"))
        {
            json::parse_zen2(cpu)?
        } else {
            panic!("buy a Zen2 CPU");
        };

        Ok(Sensors {
            cpu: Some(Cpu {
                cores: cpu_info::fetch()?,
                temp: cpu_temp,
            }),
            graphics: None,
            others: vec![],
        })
    }
}

mod json {
    use std::collections::HashMap;

    #[derive(Clone, Debug, serde::Deserialize)]
    #[serde(transparent)]
    pub struct Devices(pub HashMap<String, Device>);

    #[derive(Clone, Debug, serde::Deserialize)]
    pub struct Device {
        #[serde(rename = "Adapter")]
        pub adapter: String,
        #[serde(flatten)]
        pub extra: HashMap<String, serde_json::Value>,
    }

    pub fn parse_zen2(cpu: &Device) -> Result<super::CpuTemp, Error> {
        log::info!("{:?}", cpu);
        let fields = &cpu.extra;
        Ok(super::CpuTemp::Zen2 {
            voltage_core: fields["Vcore"]["in0_input"].as_f64().unwrap(),
            voltage_so_c: fields["Vsoc"]["in1_input"].as_f64().unwrap(),
            current_core: fields["Icore"]["curr1_input"].as_f64().unwrap(),
            current_so_c: fields["Isoc"]["curr2_input"].as_f64().unwrap(),
            temp_die: fields["Tdie"]["temp1_input"].as_f64().unwrap(),
            temp_ctl: fields["Tctl"]["temp2_input"].as_f64().unwrap(),
            temp_ccd1: fields["Tccd1"]["temp3_input"].as_f64().unwrap(),
            temp_ccd2: fields["Tccd2"]["temp4_input"].as_f64().unwrap(),
        })
    }

    #[derive(Clone, Debug, thiserror::Error)]
    pub enum Error {}
}

mod cpu_info {
    use std::{fs, io::Error as IoError, sync::Arc};

    pub fn fetch() -> Result<Vec<super::CpuCore>, Error> {
        let data = fs::read_to_string("/proc/cpuinfo").map_err(|e| Error::CannotReadProcFile {
            source: Arc::new(e),
        })?;

        let cores = data.split("\n\n")
            .filter_map(|x| {
                x.lines()
                    .find(|l| l.starts_with("cpu MHz"))?
                    .split(":")
                    .nth(1)?
                    .trim()
                    .parse::<f64>()
                    .ok()
            })
            .map(|freq| super::CpuCore { freq })
            .collect();

        Ok(cores)
    }

    #[derive(Clone, Debug, thiserror::Error)]
    pub enum Error {
        #[error("cannot read `/proc/cpuinfo`")]
        CannotReadProcFile {
            #[source]
            source: Arc<IoError>,
        },
    }
}

#[derive(Clone, Debug, thiserror::Error)]
pub enum Error {
    #[error("failed to spawn command `{cmd}`")]
    SpawnCommand {
        cmd: &'static str,
        #[source]
        source: Arc<IoError>,
    },
    #[error("failed to run `sensors`")]
    SensorsCmdFailed,
    #[error("cannot parse `sensors` output")]
    CannotParseSensorsJson {
        #[source]
        source: Arc<serde_json::Error>,
    },
    #[error("cannot read data from `sensors`")]
    ReadJsonSensorData {
        #[from]
        source: json::Error,
    },
    #[error("cannot read `cpuinfo` data")]
    CpuInfoFailure {
        #[from]
        source: cpu_info::Error,
    }
}
