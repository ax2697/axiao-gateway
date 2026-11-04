/// 按账号生成稳定设备指纹（FNV-1a，不依赖外部库）
pub fn machine_id(uid: &str) -> String {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in uid.as_bytes().iter().chain(b"axiao-device-v1".iter()) {
        h ^= *b as u64;
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    let h2 = h.rotate_left(23) ^ 0x9e37_79b9_7f4a_7c15;
    format!("{h:016x}{h2:016x}")
}

/// 单账号设备 ID
#[tauri::command]
pub fn device_id(uid: String) -> Result<String, String> {
    Ok(machine_id(&uid))
}

/// 批量设备 ID（按账号）
#[tauri::command]
pub fn device_ids(uids: Vec<String>) -> Result<Vec<(String, String)>, String> {
    Ok(uids.into_iter().map(|u| {
        let id = machine_id(&u);
        (u, id)
    }).collect())
}
