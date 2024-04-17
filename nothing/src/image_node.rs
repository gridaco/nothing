use image::DynamicImage;
use image::GenericImageView;
use reqwest;
use wgpu::{Device, Queue, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages};

pub async fn load_image_from_url(url: &str) -> DynamicImage {
    let resp = reqwest::get(url).await.expect("Failed to download image");
    let bytes = resp.bytes().await.expect("Failed to read image bytes");
    image::load_from_memory(&bytes).expect("Failed to decode image")
}

pub fn create_texture_from_image(
    device: &Device,
    queue: &Queue,
    image: DynamicImage,
) -> wgpu::Texture {
    let rgba = image.to_rgba8();
    let (width, height) = image.dimensions();

    let size = wgpu::Extent3d {
        width,
        height,
        depth_or_array_layers: 1,
    };
    let texture = device.create_texture(&TextureDescriptor {
        label: Some("Texture"),
        size,
        mip_level_count: 1,
        sample_count: 1,
        dimension: TextureDimension::D2,
        format: TextureFormat::Rgba8UnormSrgb,
        usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
        view_formats: &[],
    });

    queue.write_texture(
        wgpu::ImageCopyTexture {
            texture: &texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        &rgba,
        wgpu::ImageDataLayout {
            offset: 0,
            bytes_per_row: Some(4 * width),
            rows_per_image: Some(height),
        },
        size,
    );

    texture
}
