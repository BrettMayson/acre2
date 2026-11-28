//! What the extension holds: the loaded terrain, the antenna library, and the models.

use std::{
    collections::HashMap,
    sync::{Arc, RwLock, atomic::AtomicU32},
};

use crate::{
    antenna::Antenna,
    map::{Climate, Map},
    math::Vec3,
    models::{Link, Signal, arcade, itm, los, multipath::Multipath},
    pbo, signal_map,
    wrp::{Landscape, LoadResult},
};

/// `acre_sys_signal_signalModel` (SQF `SIGNAL_MODEL_*`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Model {
    Arcade,
    LosSimple,
    LosMultipath,
    Itm,
}

impl Model {
    /// Unknown values (including the disabled ITWOM, 4) fall back to multipath, as in the C++.
    #[must_use]
    pub fn from_index(i: i32) -> Self {
        match i {
            0 => Self::Arcade,
            1 => Self::LosSimple,
            3 => Self::Itm,
            _ => Self::LosMultipath,
        }
    }
}

/// One transmitter antenna → receiver antenna calculation, as `process_signal` took it.
#[derive(Debug, Clone)]
pub struct Request {
    pub model: Model,
    pub tx_pos: Vec3,
    pub tx_dir: Vec3,
    pub tx_antenna: String,
    pub rx_pos: Vec3,
    pub rx_dir: Vec3,
    pub rx_antenna: String,
    pub frequency: f32,
    pub power: f32,
    pub scale: f32,
    pub omnidirectional: bool,
}

#[derive(Debug)]
struct Terrain {
    path: String,
    map: Arc<Map>,
    multipath: Multipath,
}

#[derive(Debug, Default)]
pub struct Engine {
    pbos: RwLock<pbo::Index>,
    terrain: RwLock<Option<Arc<Terrain>>>,
    antennas: RwLock<HashMap<String, Arc<Antenna>>>,
}

fn read<T>(l: &RwLock<T>) -> std::sync::RwLockReadGuard<'_, T> {
    l.read().unwrap_or_else(std::sync::PoisonError::into_inner)
}

fn write<T>(l: &RwLock<T>) -> std::sync::RwLockWriteGuard<'_, T> {
    l.write().unwrap_or_else(std::sync::PoisonError::into_inner)
}

impl Engine {
    /// Indexes the PBOs the game has open (new ones only).
    pub fn index_open_pbos(&self) {
        let pbos = crate::open_files::open_pbos();
        write(&self.pbos).add(pbos);
    }

    /// Adds PBOs to the index directly (tests and tools).
    pub fn index_pbos(&self, pbos: impl IntoIterator<Item = std::path::PathBuf>) {
        write(&self.pbos).add(pbos);
    }

    /// Reads a file from the game's PBOs, refreshing the index once if it isn't found.
    pub fn read_game_file(&self, virtual_path: &str) -> std::io::Result<Vec<u8>> {
        if let Ok(data) = read(&self.pbos).read(virtual_path) {
            return Ok(data);
        }
        self.index_open_pbos();
        read(&self.pbos).read(virtual_path)
    }

    /// Loads the terrain (`\a3\map_altis\altis.wrp`, or a compat `.fakewrp`). Loading the
    /// same terrain again only updates the climate.
    pub fn load_map(&self, wrp_path: &str, climate: Climate) -> LoadResult {
        let path = pbo::normalize(wrp_path);
        if let Some(t) = &*read(&self.terrain)
            && t.path == path
        {
            return LoadResult::Success;
        }
        // The C++ appended a bare "wrp"; with the dot is what was meant.
        let path_with_ext = if path.contains("wrp") {
            path.clone()
        } else {
            format!("{path}.wrp")
        };
        let data = match self.read_game_file(&path_with_ext) {
            Ok(d) => d,
            Err(e) => {
                tracing::error!(wrp = %path_with_ext, error = %e, "terrain file not found");
                *write(&self.terrain) = None;
                return LoadResult::Failure;
            }
        };
        let landscape = match Landscape::parse(&data) {
            Ok(l) => l,
            Err(e) => {
                tracing::error!(wrp = %path_with_ext, error = %e, "terrain file unreadable");
                *write(&self.terrain) = None;
                return LoadResult::Failure;
            }
        };
        self.set_map(&path, Map::from_landscape(&landscape), climate);
        tracing::info!(
            wrp = %path_with_ext,
            size = landscape.map_size.0,
            grid = landscape.grid_size,
            result = ?landscape.result,
            "terrain loaded"
        );
        landscape.result
    }

    /// Uses `map` as the terrain.
    pub fn set_map(&self, path: &str, mut map: Map, climate: Climate) {
        map.climate = climate;
        let map = Arc::new(map);
        *write(&self.terrain) = Some(Arc::new(Terrain {
            path: path.to_string(),
            multipath: Multipath::new(map.clone()),
            map,
        }));
    }

    #[must_use]
    pub fn map(&self) -> Option<Arc<Map>> {
        read(&self.terrain).as_ref().map(|t| t.map.clone())
    }

    /// Loads an antenna's gain pattern from the game's PBOs, once per name.
    pub fn load_antenna(&self, name: &str, path: &str) -> bool {
        if read(&self.antennas).contains_key(name) {
            return true;
        }
        let path = pbo::normalize(path);
        let path = if path.contains("aba") {
            path
        } else {
            format!("{path}.aba")
        };
        match self.read_game_file(&path).map(|d| Antenna::parse(&d)) {
            Ok(Ok(a)) => {
                self.add_antenna(name, a);
                true
            }
            Ok(Err(e)) => {
                tracing::error!(antenna = name, path, error = %e, "antenna file unreadable");
                false
            }
            Err(e) => {
                tracing::error!(antenna = name, path, error = %e, "antenna file not found");
                false
            }
        }
    }

    pub fn add_antenna(&self, name: &str, antenna: Antenna) {
        write(&self.antennas).insert(name.to_string(), Arc::new(antenna));
    }

    /// Signal at the receiver. `None` when an antenna isn't loaded. Non-finite results become
    /// 9000 (as the C++ did, so SQF can parse them).
    #[must_use]
    pub fn process(&self, r: &Request) -> Option<Signal> {
        let (tx_antenna, rx_antenna) = {
            let lib = read(&self.antennas);
            let tx = lib.get(&r.tx_antenna).cloned();
            let rx = lib.get(&r.rx_antenna).cloned();
            let (Some(tx), Some(rx)) = (tx, rx) else {
                tracing::warn!(tx = r.tx_antenna, rx = r.rx_antenna, "antenna not loaded");
                return None;
            };
            (tx, rx)
        };
        let terrain = read(&self.terrain).clone();
        let link = Link {
            tx_pos: r.tx_pos,
            tx_dir: r.tx_dir,
            rx_pos: r.rx_pos,
            rx_dir: r.rx_dir,
            tx_antenna: &tx_antenna,
            rx_antenna: &rx_antenna,
            frequency: r.frequency,
            power: r.power,
            scale: r.scale,
            omnidirectional: r.omnidirectional,
        };
        let model = if terrain.is_some() {
            r.model
        } else {
            Model::Arcade
        };
        let mut signal = match (model, &terrain) {
            (Model::LosSimple, Some(t)) => los::process(&t.map, &link),
            (Model::LosMultipath, Some(t)) => t.multipath.process(&link),
            (Model::Itm, Some(t)) => itm::process(&t.map, &link),
            _ => arcade::process(r.tx_pos, r.rx_pos, &r.rx_antenna, r.frequency, r.power),
        };
        if !signal.dbm.is_finite() || !signal.v.is_finite() {
            tracing::debug!(?r, "signal was not finite");
            signal.dbm = 9000.0;
            signal.v = 9000.0;
        }
        Some(signal)
    }

    /// Multipath signal from one transmitter across an area (the signal map debug tool).
    /// `None` without a terrain or when an antenna isn't loaded.
    #[must_use]
    pub fn signal_map(
        &self,
        area: &signal_map::Area,
        tx_antenna: &str,
        rx_antenna: &str,
        progress: &AtomicU32,
        total: &AtomicU32,
    ) -> Option<signal_map::Result> {
        let terrain = read(&self.terrain).clone()?;
        let (tx, rx) = {
            let lib = read(&self.antennas);
            (lib.get(tx_antenna).cloned()?, lib.get(rx_antenna).cloned()?)
        };
        Some(signal_map::compute(
            &terrain.map,
            &terrain.multipath,
            &tx,
            &rx,
            area,
            progress,
            total,
        ))
    }
}
