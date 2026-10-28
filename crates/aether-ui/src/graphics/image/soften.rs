use std::sync::Arc;

use super::Bitmap;

/*
 * blurs the image by averaging each pixel with its neighbours up to
 * radius away, first along rows and then along columns; three rounds
 * of that look close to a gaussian blur
 */
pub fn soften(mut image: Bitmap, radius: u32) -> Bitmap {
    if radius == 0 {
        return image;
    }

    for _ in 0..3 {
        image.pixels = Arc::new(pass(&image, radius, 1, 0));
        image.pixels = Arc::new(pass(&image, radius, 0, 1));
    }

    image
}

// one box blur along x when step_x is 1, or along y when step_y is 1
fn pass(image: &Bitmap, radius: u32, step_x: u32, step_y: u32) -> Vec<u8> {
    let width = image.width as usize;
    let height = image.height as usize;
    let radius = radius as usize;
    let count = (radius * 2 + 1) as u64;

    let mut blurred = vec![0; image.pixels.len()];

    let horizontal = step_x == 1 && step_y == 0;
    let (line_count, line_length) = if horizontal {
        (height, width)
    } else {
        (width, height)
    };

    for line in 0..line_count {
        let index_at = |position: usize| {
            let (x, y) = if horizontal {
                (position, line)
            } else {
                (line, position)
            };
            (y * width + x) * 4
        };
        let mut total = [0u64; 4];

        for offset in 0..=radius * 2 {
            let position = offset as i64 - radius as i64;
            let sample = position.clamp(0, line_length as i64 - 1) as usize;
            let index = index_at(sample);
            for channel in 0..4 {
                total[channel] += u64::from(image.pixels[index + channel]);
            }
        }

        for position in 0..line_length {
            let index = index_at(position);
            for channel in 0..4 {
                blurred[index + channel] = (total[channel] / count) as u8;
            }

            if position + 1 == line_length {
                continue;
            }

            let leaving =
                (position as i64 - radius as i64).clamp(0, line_length as i64 - 1) as usize;
            let entering =
                (position as i64 + radius as i64 + 1).clamp(0, line_length as i64 - 1) as usize;
            let leaving_index = index_at(leaving);
            let entering_index = index_at(entering);

            for channel in 0..4 {
                total[channel] -= u64::from(image.pixels[leaving_index + channel]);
                total[channel] += u64::from(image.pixels[entering_index + channel]);
            }
        }
    }

    blurred
}

#[cfg(test)]
mod tests {
    use super::*;

    // a hard black to white edge turns into a ramp, and flat areas stay flat
    #[test]
    fn edge_becomes_ramp() {
        let width = 16;

        let mut pixels = Vec::new();

        for x in 0..width {
            let value = if x < width / 2 { 0 } else { 255 };

            pixels.extend([value, value, value, 255]);
        }

        let image = soften(
            Bitmap {
                width,
                height: 1,
                pixels: Arc::new(pixels),
            },
            2,
        );

        let red: Vec<u8> = image.pixels.chunks(4).map(|pixel| pixel[0]).collect();

        assert_eq!(red[0], 0);
        assert_eq!(red[15], 255);

        assert!(
            red[7] > 0 && red[8] < 255,
            "the edge should soften: {red:?}"
        );

        assert!(
            red.windows(2).all(|pair| pair[0] <= pair[1]),
            "should only rise: {red:?}"
        );
    }

    #[test]
    fn sliding_window_matches_repeated_edge_samples() {
        let pixels: Vec<u8> = (0..5 * 3 * 4).map(|value| (value * 17) as u8).collect();
        let image = Bitmap {
            width: 5,
            height: 3,
            pixels: Arc::new(pixels),
        };

        for (step_x, step_y) in [(1, 0), (0, 1)] {
            assert_eq!(
                pass(&image, 2, step_x, step_y),
                naive_pass(&image, 2, step_x, step_y)
            );
        }
    }

    fn naive_pass(image: &Bitmap, radius: u32, step_x: u32, step_y: u32) -> Vec<u8> {
        let mut output = vec![0; image.pixels.len()];
        for y in 0..image.height as i64 {
            for x in 0..image.width as i64 {
                let mut total = [0u64; 4];
                let mut count = 0;
                for offset in -(radius as i64)..=radius as i64 {
                    let sample_x = (x + offset * step_x as i64).clamp(0, image.width as i64 - 1);
                    let sample_y = (y + offset * step_y as i64).clamp(0, image.height as i64 - 1);
                    let index = ((sample_y * image.width as i64 + sample_x) * 4) as usize;
                    for channel in 0..4 {
                        total[channel] += u64::from(image.pixels[index + channel]);
                    }
                    count += 1;
                }
                let index = ((y * image.width as i64 + x) * 4) as usize;
                for channel in 0..4 {
                    output[index + channel] = (total[channel] / count) as u8;
                }
            }
        }
        output
    }
}
