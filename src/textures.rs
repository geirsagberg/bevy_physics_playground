use bevy::prelude::{Color, Commands, Mesh, ResMut, Resource, shape};
use bevy::asset::Assets;
use bevy::sprite::Mesh2dHandle;
use rand::random;

pub fn generate_textures(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    let meshes = VERTEX_COLORS.map(|colors| {
        let vertex_colors: Vec<[f32; 4]> = colors.map(|color| color.as_rgba_f32()).to_vec();
        let mut mesh = Mesh::from(shape::Quad::default());
        mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, vertex_colors);
        let handle: Mesh2dHandle = meshes.add(mesh).into();
        handle
    }).to_vec();
    commands.insert_resource(Meshes { meshes });
}

const VERTEX_COLORS: [[Color; 4]; 4] = [
    [Color::RED, Color::WHITE, Color::GREEN, Color::BLUE],
    [Color::YELLOW, Color::WHITE, Color::PURPLE, Color::RED],
    [Color::ORANGE, Color::BLUE, Color::WHITE, Color::YELLOW],
    [Color::PURPLE, Color::YELLOW, Color::WHITE, Color::BLUE],
];

#[derive(Resource, Debug)]
pub struct Meshes {
    meshes: Vec<Mesh2dHandle>,
}

impl Meshes {
    pub(crate) fn get_random(&self) -> Mesh2dHandle {
        let index = random::<usize>() % self.meshes.len();
        self.meshes[index].clone()
    }
}
