mod pixmap;
mod audio;

use std::fs::OpenOptions;
use std::io::{self, Read};
use std::os::unix::io::AsRawFd;
use std::{ptr, thread};
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::thread::sleep;
use std::time::Duration;

use pixmap::{Pixmap, RGB};
use crate::pixmap::{dump, read_bin};

struct FbVarScreenInfo {
    xres: u32,
    yres: u32,
    xres_virtual: u32,
    yres_virtual: u32,
    xoffset: u32,
    yoffset: u32,
    bits_per_pixel: u32,
    grayscale: u32,
}

struct FbFixScreenInfo {
    line_length: u32,
}

fn draw_to_framebuffer(images: &[Pixmap], vinfo: &FbVarScreenInfo, finfo: &FbFixScreenInfo, fbp: *mut u8) {
    let center_x = vinfo.xres / 2;
    let center_y = vinfo.yres / 2;

    let image_start_x = center_x - (images[0].w / 2);
    let image_start_y = center_y - (images[0].h / 2);

    for image in images {
        for y in 0..image.h {
            for x in 0..image.w {
                let draw_x = image_start_x + x;
                let draw_y = image_start_y + y;

                if draw_x >= 0 && draw_x < vinfo.xres && draw_y >= 0 && draw_y < vinfo.yres {
                    let location = ((draw_x + vinfo.xoffset) * (vinfo.bits_per_pixel / 8) +
                        (draw_y + vinfo.yoffset) * finfo.line_length) as usize;

                    if vinfo.bits_per_pixel == 32 {
                        unsafe {
                            let rgb = image.at(x, y);
                            let pixel_ptr = fbp.offset(location as isize);
                            ptr::write(pixel_ptr, rgb);
                            ptr::write(pixel_ptr.offset(1), rgb);
                            ptr::write(pixel_ptr.offset(2), rgb);
                        }
                    }
                }
            }
        }
        sleep(Duration::from_micros(10000000 / 365));
    }
}

fn main() {
    println!("Reading bad apple pixmaps...");

    let pixmaps = read_bin("badapple-pixmap.bin");

    println!("Opening framebuffer...");


    let fbfd = OpenOptions::new()
        .read(true)
        .write(true)
        .open("/dev/fb0")
        .expect("Error opening framebuffer");

    let mut vinfo = FbVarScreenInfo {
        xres: 0,
        yres: 0,
        xres_virtual: 0,
        yres_virtual: 0,
        xoffset: 0,
        yoffset: 0,
        bits_per_pixel: 0,
        grayscale: 0,
    };

    unsafe {
        libc::ioctl(fbfd.as_raw_fd(), 0x4600, &mut vinfo);
    }

    let mut finfo = FbFixScreenInfo { line_length: 0 };

    unsafe {
        libc::ioctl(fbfd.as_raw_fd(), 0x4602, &mut finfo);
    }

    finfo.line_length = (vinfo.xres_virtual * vinfo.bits_per_pixel / 8);

    let screensize = vinfo.yres_virtual * finfo.line_length;

    let fbp = unsafe {
        libc::mmap(
            ptr::null_mut(),
            screensize as usize,
            libc::PROT_READ | libc::PROT_WRITE,
            libc::MAP_SHARED,
            fbfd.as_raw_fd(),
            0,
        )
    };

    if fbp == libc::MAP_FAILED {
        println!("Failed to map framebuffer to memory.");
        return;
    }

    println!("Pixmaps read! Drawing to the framebuffer...");

    let stop = Arc::new(AtomicBool::new(false));

    let stop_clone = stop.clone();

    thread::spawn(move || {
        audio::play_audio("badapple.wav", &stop_clone);
    });

    draw_to_framebuffer(&pixmaps, &vinfo, &finfo, fbp as *mut u8);

    stop.store(true, std::sync::atomic::Ordering::SeqCst);

    unsafe {
        libc::munmap(fbp, screensize as usize);
    }
}
