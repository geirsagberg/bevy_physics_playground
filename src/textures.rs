use bevy::asset::Assets;
use bevy::color::palettes::css::*;
use bevy::color::{ColorToComponents, Srgba};
use bevy::prelude::{Commands, Mesh, Mesh2d, Rectangle, ResMut, Resource};
use rand::random;

pub fn generate_textures(mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>) {
    let meshes = VERTEX_COLORS
        .map(|colors| {
            let vertex_colors: Vec<[f32; 4]> =
                colors.map(|color| Srgba::to_f32_array(color)).to_vec();
            let mut mesh = Mesh::from(Rectangle::default());
            mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, vertex_colors);
            let handle = Mesh2d(meshes.add(mesh));
            handle
        })
        .to_vec();
    commands.insert_resource(Meshes { meshes });
}

const VERTEX_COLORS: [[Srgba; 4]; 4] = [
    [RED, WHITE, GREEN, BLUE],
    [YELLOW, WHITE, PURPLE, RED],
    [ORANGE, BLUE, WHITE, YELLOW],
    [PURPLE, YELLOW, WHITE, BLUE],
];

#[derive(Resource, Debug)]
pub struct Meshes {
    meshes: Vec<Mesh2d>,
}

impl Meshes {
    pub(crate) fn get_random(&self) -> Mesh2d {
        let index = random::<usize>() % self.meshes.len();
        self.meshes[index].clone()
    }
}
