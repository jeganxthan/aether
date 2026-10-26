use std::process::Command;

pub fn select_region() -> Result<String, String> {
    let output = Command::new("slurp").output().map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            "slurp is not installed".to_string()
        } else {
            format!("Failed to launch region selector: {e}")
        }
    })?;

    if !output.status.success() {
        return Err("Region selection cancelled".into());
    }

    let region = String::from_utf8(output.stdout)
        .map_err(|e| e.to_string())?
        .trim()
        .to_owned();

    if region.is_empty() {
        return Err("No region selected".into());
    }

    Ok(region)
}

pub fn select_point() -> Result<(i32, i32), String> {
    let output = Command::new("slurp").arg("-p").output().map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            "slurp is not installed".to_string()
        } else {
            format!("Failed to launch color selector: {e}")
        }
    })?;

    if !output.status.success() {
        return Err("Color selection cancelled".into());
    }

    let point = String::from_utf8(output.stdout).map_err(|e| e.to_string())?;

    let (x, y) = point
        .trim()
        .split_once(',')
        .ok_or("Unexpected point format from slurp")?;

    let x = x.trim().parse().map_err(|_| "Invalid X coordinate")?;
    let y = y.trim().parse().map_err(|_| "Invalid Y coordinate")?;

    Ok((x, y))
}
