use std::env;
use std::fs;

use breplot::{Config, NurbsInput, render_nurbs, render_step, render_step_data};

fn main() {
    if let Err(e) = run() {
        eprintln!("breplot: {e}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let args: Vec<String> = env::args().collect();
    if args.get(1).is_some_and(|arg| arg == "cache") {
        if args.len() < 4 || args.len() > 5 {
            return Err("usage: breplot-cli cache INPUT.step OUTPUT.bin [CONFIG.json]".into());
        }
        let step = fs::read(&args[2]).map_err(|e| format!("read STEP: {e}"))?;
        let config = read_config(args.get(4))?;
        let data = render_step_data(&step, &config)?;
        fs::write(&args[3], data).map_err(|e| format!("write preview cache: {e}"))?;
        return Ok(());
    }
    if args.get(1).is_some_and(|arg| arg == "curve") {
        if args.len() != 4 {
            return Err("usage: breplot-cli curve INPUT.json OUTPUT.svg".into());
        }
        let bytes = fs::read(&args[2]).map_err(|e| format!("read NURBS JSON: {e}"))?;
        let input: NurbsInput =
            serde_json::from_slice(&bytes).map_err(|e| format!("parse NURBS JSON: {e}"))?;
        let svg = render_nurbs(&input)?;
        fs::write(&args[3], svg).map_err(|e| format!("write SVG: {e}"))?;
        return Ok(());
    }
    if args.len() < 3 || args.len() > 4 {
        return Err("usage: breplot INPUT.step OUTPUT.svg [CONFIG.json]".into());
    }
    let step = fs::read(&args[1]).map_err(|e| format!("read STEP: {e}"))?;
    let config = read_config(args.get(3))?;
    let (svg, stats) = render_step(&step, &config)?;
    fs::write(&args[2], svg).map_err(|e| format!("write SVG: {e}"))?;
    println!("{stats:#?}");
    Ok(())
}

fn read_config(path: Option<&String>) -> Result<Config, String> {
    if let Some(path) = path {
        let bytes = fs::read(path).map_err(|e| format!("read config: {e}"))?;
        serde_json::from_slice::<Config>(&bytes).map_err(|e| format!("parse config: {e}"))
    } else {
        Ok(Config::default())
    }
}
