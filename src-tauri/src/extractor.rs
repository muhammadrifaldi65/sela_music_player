use std::{process::Stdio, time::Duration};
use tokio::{io::AsyncReadExt, process::Command};
pub async fn run(command: &mut Command, limit: usize, seconds: u64) -> Result<Vec<u8>, String> {
    command
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .kill_on_drop(true);
    let mut child = command.spawn().map_err(|e| e.to_string())?;
    let stdout = child
        .stdout
        .take()
        .ok_or("Keluaran ekstraktor tidak tersedia")?;
    let result = tokio::time::timeout(Duration::from_secs(seconds), async {
        let mut bytes = Vec::new();
        stdout
            .take(limit as u64 + 1)
            .read_to_end(&mut bytes)
            .await
            .map_err(|e| e.to_string())?;
        if bytes.len() > limit {
            return Err("Respons media terlalu besar".to_string());
        }
        if !child.wait().await.map_err(|e| e.to_string())?.success() {
            return Err("Sumber media tidak dapat diakses".to_string());
        }
        Ok(bytes)
    })
    .await;
    match result {
        Ok(Ok(bytes)) => Ok(bytes),
        other => {
            let _ = child.kill().await;
            let _ = child.wait().await;
            match other {
                Ok(Err(error)) => Err(error),
                _ => Err("Permintaan media melewati batas waktu".into()),
            }
        }
    }
}
