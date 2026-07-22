fn main() {
    let face = mfd::physical_mfd_layout(mfd::term::detect_backend(), mfd::mfd_face_inches());
    println!(
        "req={:.2} ppi={:.1} px={:.4} {:?} cell={:.1}x{:.1} side={} cells={}x{} og={:.3} clipped={}",
        face.inches_requested,
        face.ppi,
        face.pixel_space.winsize_to_device,
        face.pixel_space.source,
        face.cell_device.0,
        face.cell_device.1,
        face.side_px,
        face.viewport.cols,
        face.viewport.rows,
        face.on_glass_in,
        face.clipped,
    );
}
