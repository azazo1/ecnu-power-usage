//! 全校宿舍名称列表.

use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Area {
    pub area_id: String,
    pub area_name: String,
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct District {
    pub district_id: String,
    pub district_name: String,
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
pub struct Building {
    #[serde(rename = "buiId")]
    pub building_id: String,
    #[serde(rename = "buiName")]
    pub building_name: String,
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Floor {
    pub floor_id: String,
    pub floor_name: String,
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Room {
    pub room_id: String,
    pub room_name: String,
}

// --- 请求及其响应结构 ---
// 以下响应都需要附带 Header 中的 `Cookie` 和 `X-CSRF-TOKEN`, 可以参见 [`crate::Cookies`].

/// 查询所有地区.
///
/// - url: https://epay.ecnu.edu.cn/epaycas/electric/queryelectricarea
/// - method: post
/// - payload(form): sysid=1
#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
pub struct Districts {
    /// 学校, 如: 华东师范大学, 一般只会为华东师范大学, 此值是不变的.
    pub areas: Vec<Area>,
    /// 地区, 如: 普陀校内宿舍
    pub districts: Vec<District>,
    /// 建筑, 如: 闵行本科生1号楼
    #[serde(rename = "buils")]
    pub buildings: Vec<Building>,
    /// 楼层, 如: 1
    pub floors: Vec<Floor>,
    /// 房间, 如:
    pub rooms: Vec<Room>,
}

/// 查询地区所有建筑
///
/// - url: https://epay.ecnu.edu.cn/epaycas/electric/queryelectricbuis
/// - method: post
/// - payload(form): sysid=1&area=___&district=___
#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
pub struct Buildings {
    #[serde(rename = "buils")]
    pub buildings: Vec<Building>,
}

/// 查询建筑的所有楼层
///
/// - url: https://epay.ecnu.edu.cn/epaycas/electric/queryelectricfloors
/// - method: post
/// - payload(form): sysid=1&area=___&district=__&build=___
#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
pub struct Floors {
    pub floors: Vec<Floor>,
}

/// 查询建筑内所有房间.
///
/// - url: https://epay.ecnu.edu.cn/epaycas/electric/queryelectricrooms
/// - method: post
/// - payload(form): sysid=1&area=___&district=___&build=___&floor=___
#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
pub struct Rooms {
    pub rooms: Vec<Room>,
}

// --- Info ---

/// 从 [`crate::config::RoomConfig`] 中通过上述请求总结出来的信息.
#[derive(Serialize, Debug, Clone, PartialEq, Eq)]
pub struct RoomInfo {
    pub area: Area,
    pub district: District,
    pub building: Building,
    pub floor: Floor,
    pub room: Room,
}
