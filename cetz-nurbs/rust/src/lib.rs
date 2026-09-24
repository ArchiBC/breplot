//! NURBS geometry shared by the standalone CeTZ plugin and STEP edge display.
pub mod bezier;
pub mod construct;
pub mod display;
pub mod nurbs;

#[cfg(all(target_arch = "wasm32", feature = "plugin"))]
use wasm_minimal_protocol::*;
#[cfg(all(target_arch = "wasm32", feature = "plugin"))]
initiate_protocol!();

#[cfg(all(target_arch = "wasm32", feature = "plugin"))]
#[wasm_func]
fn nurbs_native_cubics(input_json: &[u8]) -> Result<Vec<u8>, String> {
    let input: nurbs::NurbsInput =
        serde_json::from_slice(input_json).map_err(|e| format!("NURBS input: {e}"))?;
    let cubics = display::native_cubics(&input)?;
    serde_json::to_vec(&cubics).map_err(|e| format!("encode NURBS cubics: {e}"))
}

#[cfg(all(target_arch = "wasm32", feature = "plugin"))]
#[wasm_func]
fn nurbs_from_controls(input_json: &[u8]) -> Result<Vec<u8>, String> {
    let input = serde_json::from_slice(input_json).map_err(|e| format!("control points: {e}"))?;
    serde_json::to_vec(&construct::from_controls(&input)?).map_err(|e| e.to_string())
}

#[cfg(all(target_arch = "wasm32", feature = "plugin"))]
#[wasm_func]
fn nurbs_from_interpolation(input_json: &[u8]) -> Result<Vec<u8>, String> {
    let input =
        serde_json::from_slice(input_json).map_err(|e| format!("interpolation points: {e}"))?;
    serde_json::to_vec(&construct::from_interpolation(&input)?).map_err(|e| e.to_string())
}
