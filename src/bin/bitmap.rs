use bitvec::prelude::*;
use clap::{Parser, Subcommand};
use image::imageops::BiLevel;
use image::{imageops, DynamicImage, GenericImageView, GrayImage};

#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
struct Cli {
    #[clap(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Convert { image: String },
}

struct Bitmap {
    bv: BitVec<u8, Msb0>,
    width: u32,
    height: u32,
}

impl Bitmap {
    fn new(width: u32, height: u32) -> Self {
        let mut res = Self {
            bv: BitVec::with_capacity(width as usize * height as usize),
            width,
            height,
        };
        for _ in 0..width * height {
            res.bv.push(false);
        }
        res
    }

    fn print(&self) {
        self.bv.chunks(self.width as usize).for_each(|row| {
            row.iter().for_each(|bit| {
                print!("{}", if *bit { "#" } else { " " });
            });
            println!();
        });
    }

    fn blit(&mut self, src: &Bitmap, x: u32, y: u32) {
        src.bv
            .chunks(src.width as usize)
            .enumerate()
            .for_each(|(row, bits)| {
                bits.iter().enumerate().for_each(|(col, bit)| {
                    self.bv.set(
                        (row + y as usize) * self.width as usize + col + x as usize,
                        *bit,
                    );
                });
            });
    }
}

fn convert_image(img: &GrayImage) -> Bitmap {
    let mut bv: BitVec<u8, Msb0> = BitVec::new();
    img.pixels().for_each(|p| {
        bv.push(p[0] > 0);
    });
    let (w, h) = img.dimensions();
    Bitmap {
        bv,
        width: w,
        height: h,
    }
}

pub fn main() {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Convert { image: imageName } => {
            let mut img = image::open(imageName).unwrap().into_luma8();
            imageops::dither(&mut img, &BiLevel);
            let (w, h) = img.dimensions();
            println!("image dimensions: {}x{}", w, h);

            let bitmap = convert_image(&img);
            bitmap.print();

            let mut b2 = Bitmap::new(80, 100);
            b2.blit(&bitmap, 10, 10);
            b2.print();
        }
    }
}
