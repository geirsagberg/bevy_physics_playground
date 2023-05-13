use bevy::prelude::{default, Image};
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::render::texture::BevyDefault;
use perlin_noise::PerlinNoise;

const TEXTURE_SIZE: u32 = 512;

fn create_perlin_image() -> Image {
    let perlin = PerlinNoise::new();
    let mut pixels = Vec::with_capacity((TEXTURE_SIZE * TEXTURE_SIZE * 4) as usize);
    for y in 0..TEXTURE_SIZE {
        for x in 0..TEXTURE_SIZE {
            let noise_value = perlin.get2d([x as f64 / 64.0, y as f64 / 64.0]);
            let alpha = (noise_value * 255.0) as u8;
            pixels.push(255);
            pixels.push(255);
            pixels.push(255);
            pixels.push(alpha);
        }
    }
    Image::new(
        Extent3d {
            width: TEXTURE_SIZE,
            height: TEXTURE_SIZE,
            ..default()
        },
        TextureDimension::D2,
        pixels,
        TextureFormat::bevy_default(),
    )
}
