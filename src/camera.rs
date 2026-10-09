use glam::{Mat4, Vec3};
use crate::input::InputState;

#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraUniform {
    pub view_proj: [[f32; 4]; 4],
    pub camera_pos: [f32; 4], // xyz = position, w = mode (0 = walk, 1 = fly)
    pub sun_dir: [f32; 4],    // xyz = sun direction, w = time
}

pub struct Camera {
    pub position: Vec3,
    pub yaw: f32,   // Degres
    pub pitch: f32, // Degres
    pub fov_y: f32, // Degres
    pub is_freecam: bool,
    pub vertical_velocity: f32,
    pub is_grounded: bool,
    pub eye_height: f32,
    pub move_speed: f32,
    pub sprint_multiplier: f32,
    pub mouse_sensitivity: f32,
    pub invert_mouse_y: bool,
}

impl Camera {
    pub fn new(position: Vec3) -> Self {
        Self {
            position,
            yaw: -90.0,
            pitch: 0.0, // Regard horizontal par defaut
            fov_y: 65.0,
            is_freecam: false,
            vertical_velocity: 0.0,
            is_grounded: true,
            eye_height: 1.85,
            move_speed: 14.0,
            sprint_multiplier: 2.6,
            mouse_sensitivity: 0.12,
            invert_mouse_y: false,
        }
    }

    pub fn forward_vector(&self) -> Vec3 {
        let yaw_rad = self.yaw.to_radians();
        let pitch_rad = self.pitch.to_radians();
        Vec3::new(
            yaw_rad.cos() * pitch_rad.cos(),
            pitch_rad.sin(),
            yaw_rad.sin() * pitch_rad.cos(),
        )
        .normalize()
    }

    pub fn right_vector(&self) -> Vec3 {
        self.forward_vector().cross(Vec3::Y).normalize()
    }

    pub fn horizontal_forward(&self) -> Vec3 {
        let yaw_rad = self.yaw.to_radians();
        Vec3::new(yaw_rad.cos(), 0.0, yaw_rad.sin()).normalize()
    }

    pub fn update(
        &mut self,
        dt: f32,
        input: &mut InputState,
        terrain_height_fn: impl Fn(f32, f32) -> f32,
    ) {
        // Bascule Freecam / Marcheur
        if input.pop_freecam_toggle() {
            self.is_freecam = !self.is_freecam;
            self.vertical_velocity = 0.0;
        }

        // Bascule Inversion Axe Y Souris
        if input.pop_invert_y_toggle() {
            self.invert_mouse_y = !self.invert_mouse_y;
        }

        // Rotation de vue via souris
        let (dx, dy) = input.pop_mouse_delta();
        self.yaw += dx * self.mouse_sensitivity;
        let pitch_delta = if self.invert_mouse_y {
            dy * self.mouse_sensitivity
        } else {
            -dy * self.mouse_sensitivity
        };
        self.pitch += pitch_delta;
        self.pitch = self.pitch.clamp(-88.5, 88.5);

        // Vitesse de déplacement
        let mut speed = self.move_speed;
        if input.is_sprint() {
            speed *= self.sprint_multiplier;
        }

        let forward = if self.is_freecam {
            self.forward_vector()
        } else {
            self.horizontal_forward()
        };
        let right = self.right_vector();

        let mut move_dir = Vec3::ZERO;
        if input.is_forward() {
            move_dir += forward;
        }
        if input.is_backward() {
            move_dir -= forward;
        }
        if input.is_right() {
            move_dir += right;
        }
        if input.is_left() {
            move_dir -= right;
        }

        if move_dir.length_squared() > 0.0001 {
            move_dir = move_dir.normalize();
        }

        if self.is_freecam {
            // Mode Spectateur / Vol libre 6-DOF
            let mut fly_speed = speed * 1.5;
            if input.is_sprint() {
                fly_speed *= 2.0;
            }

            self.position += move_dir * fly_speed * dt;

            if input.is_jump() {
                self.position.y += fly_speed * dt;
            }
            if input.is_crouch() {
                self.position.y -= fly_speed * dt;
            }
        } else {
            // Mode FPS à pied avec physique et collision terrain
            self.position += move_dir * speed * dt;

            let ground_y = terrain_height_fn(self.position.x, self.position.z);
            let target_y = ground_y + self.eye_height;

            const GRAVITY: f32 = -32.0;
            const JUMP_FORCE: f32 = 11.5;

            if self.is_grounded {
                if input.is_jump() {
                    self.vertical_velocity = JUMP_FORCE;
                    self.is_grounded = false;
                } else {
                    // Adhérence au sol continue
                    self.vertical_velocity = 0.0;
                    self.position.y = target_y;
                }
            } else {
                // En vol / chute libre
                self.vertical_velocity += GRAVITY * dt;
                self.position.y += self.vertical_velocity * dt;

                // Détection d'impact avec le sol
                if self.position.y <= target_y {
                    self.position.y = target_y;
                    self.vertical_velocity = 0.0;
                    self.is_grounded = true;
                }
            }
        }
    }

    pub fn build_uniform(&self, aspect: f32, time: f32) -> CameraUniform {
        let forward = self.forward_vector();
        let view = Mat4::look_at_rh(self.position, self.position + forward, Vec3::Y);

        // Matrice de projection standard Right-Handed (le viewport Vulkan negatif prend en charge le flip vertical)
        let proj = Mat4::perspective_rh(self.fov_y.to_radians(), aspect, 0.1, 4000.0);
        let view_proj = proj * view;

        // Position du soleil en orbite douce
        let sun_angle = 0.35 + (time * 0.02).sin() * 0.15;
        let sun_dir = Vec3::new(0.65, sun_angle.max(0.2), 0.55).normalize();

        CameraUniform {
            view_proj: view_proj.to_cols_array_2d(),
            camera_pos: [
                self.position.x,
                self.position.y,
                self.position.z,
                if self.is_freecam { 1.0 } else { 0.0 },
            ],
            sun_dir: [sun_dir.x, sun_dir.y, sun_dir.z, time],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_camera_projection() {
        let camera = Camera::new(Vec3::new(0.0, 10.0, 0.0));
        let uniform = camera.build_uniform(16.0 / 9.0, 0.0);
        let view_proj = Mat4::from_cols_array_2d(&uniform.view_proj);

        let sky_point = Vec3::new(0.0, 50.0, -20.0);
        let sky_clip = view_proj.project_point3(sky_point);

        let ground_point = Vec3::new(0.0, -10.0, -20.0);
        let ground_clip = view_proj.project_point3(ground_point);

        println!("Ciel NDC: Y={}", sky_clip.y);
        println!("Sol NDC: Y={}", ground_clip.y);
    }
}
