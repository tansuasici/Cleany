//! Volume list DTO for the start screen. Mirrors `VolumeEntry` in
//! `ui/src/lib/ipc.ts`; `ipc_ts_mentions_every_volume_key` catches drift.

use cleany_platform::Volume;
use serde::Serialize;

/// One mounted volume as the UI sees it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VolumeEntry {
    /// Mount point, passed back to `start_scan` as the scan root. Lossy for
    /// display only: a non-UTF-8 mount point would not round-trip.
    pub mount: String,
    /// Label shown to the user.
    pub name: String,
    /// Capacity in bytes.
    pub total: u64,
    /// Bytes available to the current user.
    pub available: u64,
    /// Removable media (USB sticks, SD cards, disk images).
    pub removable: bool,
}

impl From<Volume> for VolumeEntry {
    fn from(v: Volume) -> Self {
        Self {
            mount: v.mount.to_string_lossy().into_owned(),
            name: v.name,
            total: v.total,
            available: v.available,
            removable: v.removable,
        }
    }
}

/// Mounted volumes, sorted by mount point.
pub fn list() -> Vec<VolumeEntry> {
    cleany_platform::list_volumes()
        .into_iter()
        .map(VolumeEntry::from)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::VolumeEntry;
    use cleany_platform::Volume;
    use std::path::PathBuf;

    #[test]
    fn volume_entry_json_shape() {
        let entry = VolumeEntry::from(Volume {
            mount: PathBuf::from("/Volumes/USB"),
            name: "USB".to_owned(),
            total: 10,
            available: 4,
            removable: true,
        });
        let v = serde_json::to_value(entry).unwrap();
        assert_eq!(
            v,
            serde_json::json!({
                "mount": "/Volumes/USB",
                "name": "USB",
                "total": 10,
                "available": 4,
                "removable": true,
            })
        );
    }

    #[test]
    fn ipc_ts_mentions_every_volume_key() {
        let ts = include_str!("../../ui/src/lib/ipc.ts");
        let block = ts
            .split("export interface VolumeEntry")
            .nth(1)
            .and_then(|rest| rest.split('}').next())
            .unwrap_or_default();
        let v = serde_json::to_value(VolumeEntry {
            mount: String::new(),
            name: String::new(),
            total: 0,
            available: 0,
            removable: false,
        })
        .unwrap();
        let missing: Vec<_> = v
            .as_object()
            .unwrap()
            .keys()
            .filter(|k| !block.contains(&format!("{k}:")))
            .collect();
        assert!(
            missing.is_empty(),
            "VolumeEntry in ipc.ts lacks {missing:?}"
        );
    }
}
