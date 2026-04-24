use eframe::wgpu::naga::BuiltIn::PrimitiveCount;
use ::model_viewer::App;
use gltf::Gltf;

fn main() -> eframe::Result<()> {
    let (gltf,buffers, _) = gltf::import("scenes/lumine.glb").unwrap();
    let mesh = gltf.meshes().next().unwrap();
    
    let primitive = mesh.primitives().next().unwrap();
    let reader  = primitive.reader(|buffer| Some(&buffers[buffer.index()]));
    let positions = reader.read_positions().unwrap();
    //println!("{:?}", positions);
    let normals = reader.read_normals().unwrap();
    //println!("{:?}", normals);
    let uvs = reader.read_tex_coords(0).unwrap();
    //println!("{:?}", uvs);
    
    let options = eframe::NativeOptions {
        // Request wgpu as the rendering backend
        wgpu_options: egui_wgpu::WgpuConfiguration {
            ..Default::default()
        },
        ..Default::default()
    };
    eframe::run_native(
        "3D Model Viewer",
        options,
        Box::new(|cc| Ok(Box::new(App::new(cc)))),
    )
}
