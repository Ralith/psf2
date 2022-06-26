use psf2::Font;

const FONT: &[u8] = include_bytes!("../Tamzen6x12.psf");

#[test]
fn smoke() {
    let font = Font::new(FONT).unwrap();
    assert_eq!(font.width(), 6);
    assert_eq!(font.height(), 12);
}
