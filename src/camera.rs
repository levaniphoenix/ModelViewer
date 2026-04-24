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

    /// Recompute position from orbit angles around `self.target`.
    /// `yaw` rotates horizontally, `pitch` vertically, `radius` is distance.
    pub fn orbit(&mut self, yaw: f32, pitch: f32, radius: f32) {
        let pitch = pitch.clamp(-std::f32::consts::FRAC_PI_2 + 0.01, std::f32::consts::FRAC_PI_2 - 0.01);
        self.position = self.target + Vec3::new(
            radius * yaw.sin() * pitch.cos(),
            radius * pitch.sin(),
            radius * yaw.cos() * pitch.cos(),
        );
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
    inv_view_projection : [[f32;4]; 4],
    camera_pos : [f32; 4],
}

impl CameraUniform{
    pub fn new(camera : &Camera) -> Self{
        let vp = camera.build_view_projection_matrix();
        Self{
            view_projection : vp.to_cols_array_2d(),
            inv_view_projection : vp.inverse().to_cols_array_2d(),
            camera_pos : [camera.position.x, camera.position.y, camera.position.z, 1.0],
        }
    }
}
