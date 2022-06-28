use bencher::{benchmark_group, benchmark_main, black_box, Bencher};

use psf2::Font;

benchmark_main!(benches);
benchmark_group!(benches, rasterize);

const FONT: &[u8] = include_bytes!("../Tamzen6x12.psf");

fn rasterize(b: &mut Bencher) {
    let font = Font::new(FONT).unwrap();
    let glyph = font.get('A').unwrap();
    let mut buf = [0u32; 6 * 12];
    b.iter(|| {
        for (row_index, row) in glyph.rows().enumerate() {
            for (column_index, column) in row.enumerate() {
                if column {
                    buf[row_index * 6 + column_index] = u32::MAX;
                }
            }
        }
        black_box(buf);
    });
}
