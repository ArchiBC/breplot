//! The current brepkit STEP reader requires the optional ref_direction of
//! AXIS2_PLACEMENT_3D. Rhino 8 can legitimately omit it. Fill only that
//! omitted field with a direction perpendicular to the specified axis.

use std::collections::HashMap;

pub fn normalize_optional_axis_directions(input: &str) -> Result<(String, usize), String> {
    let mut directions = HashMap::<u64, [f64; 3]>::new();
    let mut max_id = 0_u64;
    for line in input.lines() {
        let line = line.trim();
        let Some((id, body)) = parse_entity(line) else {
            continue;
        };
        max_id = max_id.max(id);
        if body.starts_with("DIRECTION(") {
            let start = body.rfind('(').ok_or("malformed DIRECTION")? + 1;
            let end = body[start..].find(')').ok_or("malformed DIRECTION")? + start;
            let values = body[start..end]
                .split(',')
                .map(|v| v.trim().parse::<f64>())
                .collect::<Result<Vec<_>, _>>()
                .map_err(|_| "malformed DIRECTION coordinates")?;
            if values.len() == 3 {
                directions.insert(id, [values[0], values[1], values[2]]);
            }
        }
    }

    let mut replacement = String::with_capacity(input.len() + 256);
    let mut added = String::new();
    let mut count = 0;
    for line in input.split_inclusive('\n') {
        let trimmed = line.trim();
        let Some((_, body)) = parse_entity(trimmed) else {
            replacement.push_str(line);
            continue;
        };
        if !body.starts_with("AXIS2_PLACEMENT_3D(") || !trimmed.ends_with(",$);") {
            replacement.push_str(line);
            continue;
        }
        let attrs = body
            .strip_prefix("AXIS2_PLACEMENT_3D(")
            .and_then(|s| s.strip_suffix(");"))
            .ok_or("malformed AXIS2_PLACEMENT_3D")?;
        let parts: Vec<_> = attrs.split(',').map(str::trim).collect();
        if parts.len() != 4 || parts[3] != "$" {
            return Err("unsupported AXIS2_PLACEMENT_3D layout".into());
        }
        let axis_id = parts[2]
            .strip_prefix('#')
            .ok_or("missing AXIS2_PLACEMENT_3D axis")?
            .parse::<u64>()
            .map_err(|_| "invalid AXIS2_PLACEMENT_3D axis reference")?;
        let axis = directions
            .get(&axis_id)
            .ok_or_else(|| format!("AXIS2_PLACEMENT_3D axis #{axis_id} not found"))?;
        let basis = if axis[0].abs() <= axis[1].abs() && axis[0].abs() <= axis[2].abs() {
            [1.0, 0.0, 0.0]
        } else if axis[1].abs() <= axis[2].abs() {
            [0.0, 1.0, 0.0]
        } else {
            [0.0, 0.0, 1.0]
        };
        let squared = axis.iter().map(|v| v * v).sum::<f64>();
        if !squared.is_finite() || squared <= 1e-24 {
            return Err(format!("AXIS2_PLACEMENT_3D axis #{axis_id} is invalid"));
        }
        let dot = axis.iter().zip(basis).map(|(a, b)| a * b).sum::<f64>();
        let mut reference = [0.0; 3];
        for i in 0..3 {
            reference[i] = basis[i] - axis[i] * dot / squared;
        }
        let length = reference.iter().map(|v| v * v).sum::<f64>().sqrt();
        for value in &mut reference {
            *value /= length;
        }
        max_id += 1;
        replacement.push_str(&line.replacen(",$);", &format!(",#{max_id});"), 1));
        added.push_str(&format!(
            "#{max_id}=DIRECTION('',({:.17E},{:.17E},{:.17E}));\n",
            reference[0], reference[1], reference[2]
        ));
        count += 1;
    }
    if count == 0 {
        return Ok((replacement, 0));
    }
    let data = replacement
        .find("DATA;")
        .ok_or("STEP DATA section missing")?;
    let end = replacement[data..]
        .find("ENDSEC;")
        .ok_or("STEP DATA ENDSEC missing")?
        + data;
    replacement.insert_str(end, &added);
    Ok((replacement, count))
}

fn parse_entity(line: &str) -> Option<(u64, &str)> {
    let after_hash = line.strip_prefix('#')?;
    let (id, body) = after_hash.split_once('=')?;
    Some((id.trim().parse().ok()?, body.trim()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fills_optional_reference_without_changing_given_one() {
        let src = "DATA;\n#1=DIRECTION('',(0.,1.,0.));\n#2=AXIS2_PLACEMENT_3D('',#5,#1,$);\n#3=AXIS2_PLACEMENT_3D('',#5,#1,#1);\nENDSEC;\n";
        let (out, count) = normalize_optional_axis_directions(src).unwrap();
        assert_eq!(count, 1);
        assert!(out.contains("#2=AXIS2_PLACEMENT_3D('',#5,#1,#4);"));
        assert!(out.contains(
            "#4=DIRECTION('',(1.00000000000000000E0,0.00000000000000000E0,0.00000000000000000E0));"
        ));
        assert!(out.contains("#3=AXIS2_PLACEMENT_3D('',#5,#1,#1);"));
    }
}
