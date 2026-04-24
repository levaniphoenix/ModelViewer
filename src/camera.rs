use glam::{Mat4, Vec3};

pub struct Camera{
    pub position : Vec3,
    pub  target : Vec3,
    pub rotation : Vec3,
    pub  up : Vec3,
    pub fov : f32,
    pub z_far : f32,
    pub z_near : f32,
    pub aspect : f32,
}

impl Camera{
    pub fn new() -> Self{
        Self{
            position : Vec3::ZERO,
            target: Vec3::ZERO,
            rotation : Vec3::ZERO,
            up : Vec3::Y,
            fov : 45.0,
            z_far : 1000.0,
            z_near : 0.1,
            aspect : 1.0,
        }
    }
    
    pub  fn build_view_projection_matrix(&self) -> Mat4{
        let view = Mat4::look_at_rh(self.position, self.target, self.up);
        let projection = Mat4::perspective_rh(self.fov.to_radians(), self.aspect, self.z_near, self.z_far);
        projection * view
    }
}
#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraUniform{
    view_projection : [[f32;4]; 4],
}

impl CameraUniform{
    pub fn new(camera : &Camera) -> Self{
        Self{
            view_projection : camera.build_view_projection_matrix().to_cols_array_2d(),
        }
    }
} 
