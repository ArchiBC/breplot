fn main() -> Result<(), Box<dyn std::error::Error>> {
    let folder =
        std::path::PathBuf::from(std::env::args().nth(1).ok_or("output directory required")?);
    std::fs::create_dir_all(&folder)?;
    for id in [
        "brep-shaded",
        "brep-hidden",
        "brep-wire",
        "brep-controls",
        "brep-silhouette",
    ] {
        let scene = nurbs_surface::demo::display_scene(id)?;
        std::fs::write(folder.join(format!("{id}.svg")), scene.svg()?)?;
        println!(
            "{id}: {} triangles, {} semantic lines",
            scene.mesh.triangles.len(),
            scene.lines.len()
        );
    }
    Ok(())
}
