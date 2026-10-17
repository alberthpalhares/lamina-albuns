//! CT-B2 (spec `cor-e-tiff`): uma foto em Adobe RGB, lida do arquivo, exportada em Adobe RGB sai
//! sem perder as cores, e o JPEG traz o perfil compatível com Adobe RGB (1998).

use image::codecs::png::PngEncoder;
use image::{ExtendedColorType, ImageDecoder, ImageEncoder, RgbaImage};
use lamina_core::{AlbumSpec, ColorSpace, Crop, Dpi, FrameContent, Mm, Project, RelRect};
use lamina_export::{ExportOptions, color_profile};
use lamina_media::color::convert;

use crate::export_project;

#[test]
fn ct_b2_saida_em_adobe_rgb_sem_perda() {
    let dir = tempfile::tempdir().unwrap();
    // Foto de cor lisa, guardada em Adobe RGB com o perfil embutido.
    let srgb = RgbaImage::from_pixel(120, 80, image::Rgba([200, 80, 40, 255]));
    let adobe = convert(srgb, None, ColorSpace::AdobeRgb);
    let path = dir.path().join("adobe.png");
    let mut encoder = PngEncoder::new(std::fs::File::create(&path).unwrap());
    encoder
        .set_icc_profile(color_profile(ColorSpace::AdobeRgb).unwrap().to_vec())
        .unwrap();
    encoder
        .write_image(adobe.as_raw(), 120, 80, ExtendedColorType::Rgba8)
        .unwrap();

    let spec =
        AlbumSpec::new(Mm::from_cm(10.0), Mm::from_cm(10.0), Dpi::new(100).unwrap()).unwrap();
    let mut project = Project::new("teste", spec);
    let photo = project.add_photo(&path, 120, 80);
    let spread = project.add_spread();
    let content = FrameContent::Photo {
        photo,
        crop: Crop::default(),
    };
    let rect = RelRect::new(0.1, 0.1, 0.8, 0.8).unwrap();
    project.add_frame(spread, rect, content).unwrap();

    let out = dir.path().join("saida");
    let options = ExportOptions {
        quality: 100,
        space: ColorSpace::AdobeRgb,
        ..ExportOptions::default()
    };
    let report = export_project(&project, &out, &options, &|_, _| {}).unwrap();
    let file = &report.files[0];
    let jpeg = image::open(file).unwrap().into_rgb8();
    let center = jpeg.get_pixel(jpeg.width() / 2, jpeg.height() / 2);
    let want = adobe.get_pixel(0, 0);
    for c in 0..3 {
        assert!(center[c].abs_diff(want[c]) <= 2, "{center:?} × {want:?}");
    }
    let reader = std::io::BufReader::new(std::fs::File::open(file).unwrap());
    let mut decoder = image::codecs::jpeg::JpegDecoder::new(reader).unwrap();
    let icc = decoder.icc_profile().unwrap().unwrap();
    assert_eq!(icc, color_profile(ColorSpace::AdobeRgb).unwrap());
}
